use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use spacegen_core::{
    BlendMode, EditError, EngineConvention, HdriFormat, History, ImageFormat, LayerId, LayerKind,
    PngBitDepth, Project,
};
use spacegen_render::{
    CancelToken, ExportError, ExportOptions, MAX_LAYERS, Projection, Renderer, SkyUniforms,
    camera_basis, existing_outputs, export_skybox,
};

use crate::layer_cap::{CapStatus, cap_status};
use crate::preview::{PreviewCallback, PreviewResources};
use crate::preview_view::{PreviewMode, panorama_rect};
use crate::quality::QualityGovernor;
use crate::seed_source::fresh_seed;
use crate::settings::{Settings, settings_path as default_settings_path};
use crate::unsaved::{Action, Choice, Guard};

const FACE_SIZES: [u32; 6] = [512, 1024, 2048, 4096, 8192, 16384];
const DEFAULT_FOV: f32 = 1.2;
/// How long without input before the Preview is drawn at full size again.
const IDLE_SECONDS: f64 = 0.4;

enum ExportMessage {
    Progress(usize, usize),
    Finished(usize),
    Failed(String),
    Cancelled,
}

struct ExportJob {
    receiver: Receiver<ExportMessage>,
    cancel: CancelToken,
    done: usize,
    total: usize,
}

pub struct SpacegenApp {
    history: History,
    selected: Option<LayerId>,
    path: Option<PathBuf>,
    /// The project JSON as last saved or loaded, to tell whether there are unsaved changes.
    saved_json: String,
    yaw: f32,
    pitch: f32,
    fov: f32,
    settings: Settings,
    settings_path: Option<PathBuf>,
    /// A recent file picked from the menu, waiting for the unsaved-changes answer.
    recent_to_open: Option<PathBuf>,
    /// Files an export would replace, waiting for the user to confirm.
    overwrite_pending: Option<Vec<PathBuf>>,
    preview_mode: PreviewMode,
    quality: QualityGovernor,
    /// The scale the Preview is drawn at this frame (1.0 = full size).
    preview_scale: f32,
    /// egui time of the last user input, and whether the previous frame had input.
    last_input_time: f64,
    input_last_frame: bool,
    /// The settings JSON as last written, so unchanged settings are not rewritten.
    persisted_settings: String,
    job: Option<ExportJob>,
    status: String,
    shown_title: String,
    unsaved: Guard,
    close_confirmed: bool,
}

fn starter_project() -> Project {
    let mut project = Project::new(fresh_seed());
    project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::StarField);
    project
}

fn kind_name(kind: LayerKind) -> &'static str {
    match kind {
        LayerKind::StarField => "Star field",
        LayerKind::Nebula => "Nebula",
        LayerKind::GalaxyBand => "Galaxy band",
        LayerKind::Sun => "Sun",
    }
}

fn blend_name(blend: BlendMode) -> &'static str {
    match blend {
        BlendMode::Additive => "Additive",
        BlendMode::Screen => "Screen",
    }
}

fn convention_name(convention: EngineConvention) -> &'static str {
    match convention {
        EngineConvention::Unity => "Unity",
        EngineConvention::Unreal => "Unreal",
        EngineConvention::Godot => "Godot",
    }
}

impl SpacegenApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> SpacegenApp {
        if let Some(render_state) = &cc.wgpu_render_state {
            render_state
                .renderer
                .write()
                .callback_resources
                .insert(PreviewResources::new(
                    &render_state.device,
                    render_state.target_format,
                ));
        }
        let project = starter_project();
        let selected = project.layers().first().map(|l| l.id());
        let settings_path = default_settings_path();
        let settings = settings_path
            .as_deref()
            .map(Settings::load)
            .unwrap_or_default();
        SpacegenApp {
            saved_json: project.to_json(),
            history: History::new(project),
            selected,
            path: None,
            yaw: 0.4,
            pitch: 0.15,
            fov: DEFAULT_FOV,
            persisted_settings: settings.to_json(),
            settings,
            settings_path,
            recent_to_open: None,
            overwrite_pending: None,
            preview_mode: PreviewMode::FreeLook,
            quality: QualityGovernor::default(),
            preview_scale: 1.0,
            last_input_time: f64::NEG_INFINITY,
            input_last_frame: false,
            job: None,
            status: "Drag the sky to look around, scroll to zoom.".to_string(),
            shown_title: String::new(),
            unsaved: Guard::default(),
            close_confirmed: false,
        }
    }

    fn is_dirty(&self) -> bool {
        self.history.project().to_json() != self.saved_json
    }

    fn report<T>(&mut self, result: Result<T, EditError>) {
        if let Err(error) = result {
            self.status = format!("Could not do that: {error}");
        }
    }

    // ---- project files ---------------------------------------------------

    fn replace_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.selected = project.layers().first().map(|l| l.id());
        self.saved_json = project.to_json();
        self.history = History::new(project);
        self.path = path;
    }

    fn new_project(&mut self) {
        self.replace_project(starter_project(), None);
        self.status = "New project.".to_string();
    }

    fn open_project(&mut self) {
        let path = match self.recent_to_open.take() {
            Some(path) => path,
            None => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("SpaceGen project", &["spacegen"])
                    .pick_file()
                else {
                    return;
                };
                path
            }
        };
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| Project::from_json(&text).map_err(|e| e.to_string()))
        {
            Ok(project) => {
                self.status = format!("Opened {}.", path.display());
                self.settings.remember_recent(&path);
                self.replace_project(project, Some(path));
                self.persist_settings();
            }
            Err(error) => {
                self.status = format!("Could not open {}: {error}", path.display());
                self.settings.forget_recent(&path);
                self.persist_settings();
            }
        }
    }

    /// Writes the settings file when something in it changed since the last write.
    fn persist_settings(&mut self) {
        let Some(path) = &self.settings_path else {
            return;
        };
        let json = self.settings.to_json();
        if json != self.persisted_settings {
            let _ = self.settings.save(path);
            self.persisted_settings = json;
        }
    }

    /// Returns whether the Project was saved.
    fn save_project(&mut self, ask_for_path: bool) -> bool {
        let path = match (&self.path, ask_for_path) {
            (Some(path), false) => path.clone(),
            _ => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("SpaceGen project", &["spacegen"])
                    .set_file_name("sky.spacegen")
                    .save_file()
                else {
                    return false;
                };
                path
            }
        };
        let json = self.history.project().to_json();
        match std::fs::write(&path, &json) {
            Ok(()) => {
                self.status = format!("Saved {}.", path.display());
                self.saved_json = json;
                self.settings.remember_recent(&path);
                self.path = Some(path);
                self.persist_settings();
                true
            }
            Err(error) => {
                self.status = format!("Could not save {}: {error}", path.display());
                false
            }
        }
    }

    /// Runs `action` now, or asks Save / Discard / Cancel first when there are unsaved changes.
    fn attempt(&mut self, action: Action, ctx: &egui::Context) {
        if let Some(action) = self.unsaved.request(action, self.is_dirty()) {
            self.run(action, ctx);
        }
    }

    fn run(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::New => self.new_project(),
            Action::Open => self.open_project(),
            Action::Close => {
                self.close_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn unsaved_dialog(&mut self, ctx: &egui::Context) {
        if self.unsaved.pending().is_none() {
            return;
        }
        let mut choice = None;
        egui::Window::new("Unsaved changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("This project has changes that are not saved.");
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.settings.recent.is_empty(), |ui| {
                        ui.menu_button("Recent", |ui| {
                            for path in self.settings.recent.clone() {
                                let name = path.file_name().map_or_else(
                                    || path.display().to_string(),
                                    |n| n.to_string_lossy().into_owned(),
                                );
                                if ui
                                    .button(name)
                                    .on_hover_text(path.display().to_string())
                                    .clicked()
                                {
                                    self.recent_to_open = Some(path);
                                    self.attempt(Action::Open, ctx);
                                    ui.close();
                                }
                            }
                        });
                    });
                    if ui.button("Save").clicked() {
                        choice = Some(Choice::Save);
                    }
                    if ui.button("Discard").clicked() {
                        choice = Some(Choice::Discard);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(Choice::Cancel);
                    }
                });
            });
        if let Some(choice) = choice {
            let mut unsaved = std::mem::take(&mut self.unsaved);
            let action = unsaved.resolve(choice, || self.save_project(false));
            self.unsaved = unsaved;
            if let Some(action) = action {
                self.run(action, ctx);
            } else {
                self.recent_to_open = None;
            }
        }
    }

    // ---- export ------------------------------------------------------------

    /// Starts the export, or first asks before replacing files that are already in the folder.
    fn request_export(&mut self, ctx: &egui::Context) {
        let Some(folder) = self.settings.export.folder.clone() else {
            return;
        };
        let existing = existing_outputs(&folder, &self.export_options());
        if existing.is_empty() {
            self.start_export(ctx);
        } else {
            self.overwrite_pending = Some(existing);
        }
    }

    fn overwrite_dialog(&mut self, ctx: &egui::Context) {
        let Some(files) = &self.overwrite_pending else {
            return;
        };
        let names: Vec<String> = files
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .collect();
        let mut replace = false;
        let mut cancel = false;
        egui::Window::new("Replace existing files?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!(
                    "The export folder already has {} of the files this export writes:",
                    names.len()
                ));
                for name in names.iter().take(8) {
                    ui.monospace(name);
                }
                if names.len() > 8 {
                    ui.label(format!("... and {} more", names.len() - 8));
                }
                ui.horizontal(|ui| {
                    replace = ui.button("Replace").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if replace {
            self.overwrite_pending = None;
            self.start_export(ctx);
        } else if cancel {
            self.overwrite_pending = None;
        }
    }

    fn export_options(&self) -> ExportOptions {
        ExportOptions {
            name: self.settings.export.name.clone(),
            face_size: self.settings.export.face_size,
            format: self.settings.export.format,
            convention: self.settings.export.convention,
            faces: self.settings.export.faces,
            panorama: self.settings.export.panorama,
            hdri: self.settings.export.hdri,
            hdri_format: self.settings.export.hdri_format,
            png_bit_depth: self.settings.export.png_bit_depth,
            jpeg_quality: self.settings.export.jpeg_quality,
        }
    }

    fn start_export(&mut self, ctx: &egui::Context) {
        self.persist_settings();
        let Some(folder) = self.settings.export.folder.clone() else {
            return;
        };
        let project = self.history.project().clone();
        let options = self.export_options();
        let (sender, receiver) = mpsc::channel();
        let cancel = CancelToken::new();
        let thread_cancel = cancel.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let send = |message| {
                let _ = sender.send(message);
                ctx.request_repaint();
            };
            let renderer = match Renderer::gpu().map(|r| r.with_cancel_token(thread_cancel)) {
                Ok(renderer) => renderer,
                Err(error) => return send(ExportMessage::Failed(error.user_message())),
            };
            let result = export_skybox(&renderer, &project, &folder, &options, |done, total| {
                send(ExportMessage::Progress(done, total))
            });
            send(match result {
                Ok(files) => ExportMessage::Finished(files.len()),
                Err(ExportError::Cancelled) => ExportMessage::Cancelled,
                Err(error) => ExportMessage::Failed(error.user_message()),
            });
        });
        self.job = Some(ExportJob {
            receiver,
            cancel,
            done: 0,
            total: 1,
        });
        self.status = "Exporting...".to_string();
    }

    fn poll_export(&mut self) {
        let Some(job) = &mut self.job else {
            return;
        };
        let mut finished = None;
        let mut stopped = false;
        loop {
            let message = match job.receiver.try_recv() {
                Ok(message) => message,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    stopped = true;
                    break;
                }
            };
            match message {
                ExportMessage::Progress(done, total) => {
                    job.done = done;
                    job.total = total.max(1);
                }
                ExportMessage::Finished(count) => {
                    finished = Some(format!("Exported {count} files."))
                }
                ExportMessage::Failed(error) => finished = Some(error),
                ExportMessage::Cancelled => finished = Some("Export cancelled".to_string()),
            }
        }
        if finished.is_none() && stopped {
            // The export thread ended without a result: it crashed, for example in the GPU driver.
            finished = Some(
                "The export stopped unexpectedly. Try a smaller face size; if it keeps happening, update your graphics driver."
                    .to_string(),
            );
        }
        if let Some(message) = finished {
            self.status = message;
            self.job = None;
        }
    }

    fn can_export(&self) -> bool {
        self.job.is_none()
            && self.settings.export.folder.is_some()
            && !self.settings.export.name.trim().is_empty()
            && (self.settings.export.faces
                || self.settings.export.panorama
                || self.settings.export.hdri)
    }

    // ---- keyboard ------------------------------------------------------------

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let (redo_shift, undo, redo_y, save) = ctx.input_mut(|input| {
            (
                input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z),
                input.consume_key(Modifiers::COMMAND, Key::Z),
                input.consume_key(Modifiers::COMMAND, Key::Y),
                input.consume_key(Modifiers::COMMAND, Key::S),
            )
        });
        if redo_shift || redo_y {
            self.history.redo();
        } else if undo {
            self.history.undo();
        }
        if save {
            self.save_project(false);
        }
    }

    // ---- panels --------------------------------------------------------------

    fn project_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("New").clicked() {
                self.attempt(Action::New, ctx);
            }
            if ui.button("Open...").clicked() {
                self.recent_to_open = None;
                self.attempt(Action::Open, ctx);
            }
            if ui.button("Save").clicked() {
                self.save_project(false);
            }
            if ui.button("Save as...").clicked() {
                self.save_project(true);
            }
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.history.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.history.undo();
            }
            if ui
                .add_enabled(self.history.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.history.redo();
            }
        });
    }

    fn seed_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Seed");
            let mut seed = self.history.project().seed().0;
            if ui.add(egui::DragValue::new(&mut seed).speed(1.0)).changed() {
                self.history.set_seed(spacegen_core::Seed(seed));
            }
            if ui.button("Randomize").clicked() {
                self.history.randomize(fresh_seed());
            }
        });
    }

    fn layer_list(&mut self, ui: &mut egui::Ui) {
        let rows: Vec<(LayerId, LayerKind, bool)> = self
            .history
            .project()
            .layers()
            .iter()
            .map(|l| (l.id(), l.kind(), l.visible()))
            .collect();
        let count = rows.len();
        let visible_count = rows.iter().filter(|r| r.2).count();
        let cap = cap_status(visible_count, MAX_LAYERS);
        ui.label("Layers (top first)");
        ui.label(format!("{visible_count} of {MAX_LAYERS} visible Layers"));
        if let CapStatus::Over(extra) = cap {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!(
                    "The top {extra} visible Layer{} not drawn: at most {MAX_LAYERS} are. Hide or delete some.",
                    if extra == 1 { " is" } else { "s are" }
                ),
            );
        }
        let room = cap == CapStatus::Room;
        let full_hint =
            format!("At most {MAX_LAYERS} visible Layers are drawn. Hide or delete one first.");
        // Drag-and-drop reorder: the handle starts a drag carrying the Layer id; the row under
        // the pointer on release is the new index. Up/Down stay as the keyboard fallback.
        let mut row_rects: Vec<(usize, egui::Rect)> = Vec::with_capacity(count);
        for (index, &(id, kind, visible)) in rows.iter().enumerate().rev() {
            let row = ui.horizontal(|ui| {
                let handle = ui
                    .add(
                        egui::Label::new("::")
                            .selectable(false)
                            .sense(egui::Sense::drag()),
                    )
                    .on_hover_cursor(egui::CursorIcon::Grab)
                    .on_hover_text("Drag to reorder");
                handle.dnd_set_drag_payload(id);
                let mut shown = visible;
                if ui
                    .checkbox(&mut shown, "")
                    .on_hover_text("Visible")
                    .changed()
                {
                    let result = self.history.set_visible(id, shown);
                    self.report(result);
                }
                let label = format!("{} #{}", kind_name(kind), id.0);
                if ui
                    .selectable_label(self.selected == Some(id), label)
                    .clicked()
                {
                    self.selected = Some(id);
                }
                if ui
                    .add_enabled(index + 1 < count, egui::Button::new("Up"))
                    .clicked()
                {
                    let result = self.history.move_layer(id, index + 1);
                    self.report(result);
                }
                if ui
                    .add_enabled(index > 0, egui::Button::new("Down"))
                    .clicked()
                {
                    let result = self.history.move_layer(id, index - 1);
                    self.report(result);
                }
                if ui
                    .add_enabled(room, egui::Button::new("Copy"))
                    .on_disabled_hover_text(&full_hint)
                    .clicked()
                {
                    match self.history.duplicate_layer(id) {
                        Ok(copy) => self.selected = Some(copy),
                        Err(error) => self.report::<()>(Err(error)),
                    }
                }
                if ui.button("Delete").clicked() {
                    let result = self.history.remove_layer(id);
                    self.report(result);
                    if self.selected == Some(id) {
                        self.selected = self.history.project().layers().last().map(|l| l.id());
                    }
                }
            });
            row_rects.push((index, row.response.rect));
        }
        let pointer = ui.ctx().pointer_interact_pos();
        let hovered = pointer.and_then(|p| row_rects.iter().find(|(_, r)| r.contains(p)));
        if let (Some(&(target, rect)), Some(dragged)) =
            (hovered, egui::DragAndDrop::payload::<LayerId>(ui.ctx()))
        {
            let from = rows.iter().position(|r| r.0 == *dragged).unwrap_or(target);
            let y = if target > from {
                rect.top()
            } else {
                rect.bottom()
            };
            ui.painter().hline(
                rect.x_range(),
                y,
                egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
            );
            if ui.input(|i| i.pointer.any_released()) {
                let result = self.history.move_layer(*dragged, target);
                self.report(result);
            }
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(room, egui::Button::new("Add star field"))
                .on_disabled_hover_text(&full_hint)
                .clicked()
            {
                self.selected = Some(self.history.add_layer(LayerKind::StarField));
            }
            if ui
                .add_enabled(room, egui::Button::new("Add nebula"))
                .on_disabled_hover_text(&full_hint)
                .clicked()
            {
                self.selected = Some(self.history.add_layer(LayerKind::Nebula));
            }
            if ui
                .add_enabled(room, egui::Button::new("Add galaxy band"))
                .on_disabled_hover_text(&full_hint)
                .clicked()
            {
                self.selected = Some(self.history.add_layer(LayerKind::GalaxyBand));
            }
            if ui
                .add_enabled(room, egui::Button::new("Add sun"))
                .on_disabled_hover_text(&full_hint)
                .clicked()
            {
                self.selected = Some(self.history.add_layer(LayerKind::Sun));
            }
        });
    }

    fn selected_layer(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected else {
            return;
        };
        let Some(layer) = self
            .history
            .project()
            .layers()
            .iter()
            .find(|l| l.id() == id)
        else {
            return;
        };
        let (kind, blend, opacity, star, nebula, band, sun) = (
            layer.kind(),
            layer.blend(),
            layer.opacity(),
            layer.star_field_settings().copied(),
            layer.nebula_settings().copied(),
            layer.galaxy_band_settings().copied(),
            layer.sun_settings().copied(),
        );
        ui.heading(format!("{} #{}", kind_name(kind), id.0));
        if ui
            .button("Re-roll this layer")
            .on_hover_text("A new look and new colors for this layer only")
            .clicked()
        {
            let result = self.history.reroll_layer(id, fresh_seed());
            self.report(result);
        }

        let mut new_opacity = opacity;
        if ui
            .add(egui::Slider::new(&mut new_opacity, 0.0..=1.0).text("Opacity"))
            .changed()
        {
            let result = self.history.set_opacity(id, new_opacity);
            self.report(result);
        }

        let mut new_blend = blend;
        egui::ComboBox::from_label("Blend")
            .selected_text(blend_name(blend))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut new_blend, BlendMode::Additive, "Additive");
                ui.selectable_value(&mut new_blend, BlendMode::Screen, "Screen");
            });
        if new_blend != blend {
            let result = self.history.set_blend(id, new_blend);
            self.report(result);
        }

        if let Some(mut settings) = nebula {
            let mut changed = false;
            changed |= unit_slider(ui, &mut settings.coverage, "Coverage");
            changed |= unit_slider(ui, &mut settings.scale, "Cloud size");
            changed |= unit_slider(ui, &mut settings.brightness, "Brightness");
            ui.horizontal(|ui| {
                ui.label("Colors");
                changed |= ui.color_edit_button_rgb(&mut settings.color_a).changed();
                changed |= ui.color_edit_button_rgb(&mut settings.color_b).changed();
            });
            if changed {
                let result = self.history.set_nebula_settings(id, settings);
                self.report(result);
            }
            return;
        }
        if let Some(mut settings) = band {
            let mut changed = false;
            changed |= unit_slider(ui, &mut settings.tilt, "Tilt");
            changed |= unit_slider(ui, &mut settings.width, "Width");
            changed |= unit_slider(ui, &mut settings.brightness, "Brightness");
            changed |= unit_slider(ui, &mut settings.dust, "Dust lanes");
            ui.horizontal(|ui| {
                ui.label("Colors");
                changed |= ui.color_edit_button_rgb(&mut settings.color_a).changed();
                changed |= ui.color_edit_button_rgb(&mut settings.color_b).changed();
            });
            if changed {
                let result = self.history.set_galaxy_band_settings(id, settings);
                self.report(result);
            }
            return;
        }
        if let Some(mut settings) = sun {
            let mut changed = false;
            changed |= unit_slider(ui, &mut settings.azimuth, "Direction");
            changed |= unit_slider(ui, &mut settings.elevation, "Height");
            changed |= unit_slider(ui, &mut settings.size, "Size");
            changed |= unit_slider(ui, &mut settings.brightness, "Brightness");
            changed |= unit_slider(ui, &mut settings.glare, "Glare");
            ui.horizontal(|ui| {
                ui.label("Color");
                changed |= ui.color_edit_button_rgb(&mut settings.color).changed();
            });
            if changed {
                let result = self.history.set_sun_settings(id, settings);
                self.report(result);
            }
            return;
        }
        let Some(mut settings) = star else {
            ui.label("This layer kind has no settings yet and is not drawn.");
            return;
        };
        let mut changed = false;
        changed |= unit_slider(ui, &mut settings.density, "Density");
        changed |= unit_slider(ui, &mut settings.brightness, "Brightness");
        changed |= unit_slider(ui, &mut settings.size, "Star size");
        changed |= unit_slider(ui, &mut settings.color_variation, "Color variety");
        ui.horizontal(|ui| {
            ui.label("Tint");
            changed |= ui.color_edit_button_rgb(&mut settings.tint).changed();
        });
        if changed {
            let result = self.history.set_star_field_settings(id, settings);
            self.report(result);
        }
    }

    fn export_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Export");
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.settings.export.name);
        });
        ui.horizontal(|ui| {
            if ui.button("Folder...").clicked()
                && let Some(folder) = rfd::FileDialog::new().pick_folder()
            {
                self.settings.export.folder = Some(folder);
            }
            match &self.settings.export.folder {
                Some(folder) => ui.label(folder.display().to_string()),
                None => ui.label("choose where to save"),
            };
        });
        egui::ComboBox::from_label("Face size")
            .selected_text(format!("{0} x {0}", self.settings.export.face_size))
            .show_ui(ui, |ui| {
                for size in FACE_SIZES {
                    ui.selectable_value(
                        &mut self.settings.export.face_size,
                        size,
                        format!("{size} x {size}"),
                    );
                }
            });
        if self.settings.export.face_size >= 16384 {
            ui.small(
                "16384 Faces are 268 megapixels each and the panorama is 65536 x 32768 \
                 (JPEG panoramas stop at 32768, the HDRI at 8192). Expect many minutes and \
                 several GB of disk.",
            );
        }
        ui.horizontal(|ui| {
            ui.label("Format");
            ui.selectable_value(&mut self.settings.export.format, ImageFormat::Png, "PNG");
            ui.selectable_value(&mut self.settings.export.format, ImageFormat::Jpeg, "JPEG");
        });
        if self.settings.export.format == ImageFormat::Png {
            ui.horizontal(|ui| {
                ui.label("PNG depth");
                ui.selectable_value(
                    &mut self.settings.export.png_bit_depth,
                    PngBitDepth::Sixteen,
                    "16-bit",
                );
                ui.selectable_value(
                    &mut self.settings.export.png_bit_depth,
                    PngBitDepth::Eight,
                    "8-bit",
                );
            });
        }
        if self.settings.export.format == ImageFormat::Jpeg {
            ui.add(
                egui::Slider::new(&mut self.settings.export.jpeg_quality, 50..=100)
                    .text("JPEG quality"),
            );
        }
        egui::ComboBox::from_label("Face names for")
            .selected_text(convention_name(self.settings.export.convention))
            .show_ui(ui, |ui| {
                for convention in [
                    EngineConvention::Unity,
                    EngineConvention::Unreal,
                    EngineConvention::Godot,
                ] {
                    ui.selectable_value(
                        &mut self.settings.export.convention,
                        convention,
                        convention_name(convention),
                    );
                }
            });
        ui.checkbox(&mut self.settings.export.faces, "Six cube faces");
        ui.checkbox(
            &mut self.settings.export.panorama,
            "Panorama image (equirectangular)",
        );
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.settings.export.hdri, "HDRI");
            ui.add_enabled_ui(self.settings.export.hdri, |ui| {
                ui.selectable_value(
                    &mut self.settings.export.hdri_format,
                    HdriFormat::OpenExr,
                    ".exr",
                );
                ui.selectable_value(
                    &mut self.settings.export.hdri_format,
                    HdriFormat::Radiance,
                    ".hdr",
                );
            });
        });

        if ui
            .add_enabled(self.can_export(), egui::Button::new("Export"))
            .clicked()
        {
            self.request_export(ctx);
        }
        if let Some(job) = &self.job {
            ui.add(
                egui::ProgressBar::new(job.done as f32 / job.total as f32)
                    .text(format!("{} of {}", job.done, job.total)),
            );
            if ui.button("Cancel").clicked() {
                job.cancel.cancel();
                self.status = "Cancelling...".to_string();
            }
        }
    }

    /// Decides how sharp this frame's Preview is: full size once the user has paused, otherwise the
    /// size the last frames' speed allows (see `QualityGovernor`).
    fn update_preview_quality(&mut self, ctx: &egui::Context) {
        let (now, dt, input) = ctx.input(|i| {
            let input = i.pointer.any_down()
                || i.pointer.delta() != egui::Vec2::ZERO
                || i.smooth_scroll_delta != egui::Vec2::ZERO
                || !i.events.is_empty();
            (i.time, i.unstable_dt, input)
        });
        if input {
            self.last_input_time = now;
        }
        // Only frames that followed each other directly say how fast drawing is; a long gap is
        // just the app waiting for input.
        if input && self.input_last_frame && dt < 0.5 {
            self.quality.record_frame(dt * 1000.0, self.preview_scale);
        }
        self.input_last_frame = input;
        let idle = now - self.last_input_time > IDLE_SECONDS;
        self.preview_scale = self.quality.scale(idle);
        if !idle && self.preview_scale < 1.0 {
            // Come back to draw the sharp frame once the user pauses.
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(IDLE_SECONDS + 0.05));
        }
    }

    /// The offscreen size to draw at for a panel of `rect`, and the scale actually used.
    fn reduced_size(&self, rect: egui::Rect, pixels_per_point: f32) -> (Option<[u32; 2]>, f32) {
        if self.preview_scale >= 0.99 {
            return (None, 1.0);
        }
        let width = (rect.width() * pixels_per_point * self.preview_scale)
            .ceil()
            .max(1.0);
        let height = (rect.height() * pixels_per_point * self.preview_scale)
            .ceil()
            .max(1.0);
        let actual = height / (rect.height() * pixels_per_point).max(1.0);
        (Some([width as u32, height as u32]), actual)
    }

    /// The whole sky at once, as the equirectangular panorama the Export writes (not draggable).
    fn flat_preview(&mut self, ui: &mut egui::Ui) {
        let (area, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let rect = panorama_rect(area);
        let ppp = ui.ctx().pixels_per_point();
        let (reduced_size, scale) = self.reduced_size(rect, ppp);
        let pixel_angle = std::f32::consts::TAU / (rect.width().max(1.0) * ppp * scale);
        let uniforms = SkyUniforms::new(
            self.history.project(),
            Projection::Equirectangular,
            pixel_angle,
        );
        ui.painter().add(
            PreviewCallback {
                uniforms,
                reduced_size,
            }
            .paint_callback(rect),
        );
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.preview_mode, PreviewMode::FreeLook, "Free look");
            ui.selectable_value(&mut self.preview_mode, PreviewMode::Flat, "Flat panorama");
        });
        if self.preview_mode == PreviewMode::Flat {
            self.flat_preview(ui);
            return;
        }
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        let height = rect.height().max(1.0);
        if response.dragged() {
            // One pixel of dragging moves the sky one pixel, whatever the zoom.
            let per_pixel = self.fov / height;
            let delta = response.drag_delta();
            self.yaw -= delta.x * per_pixel;
            self.pitch = (self.pitch + delta.y * per_pixel).clamp(-1.55, 1.55);
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            self.fov = (self.fov * (1.0 - scroll * 0.002)).clamp(0.2, 2.4);
        }
        let ppp = ui.ctx().pixels_per_point();
        let (reduced_size, scale) = self.reduced_size(rect, ppp);
        let pixel_angle = self.fov / (height * ppp * scale);
        let basis = camera_basis(self.yaw, self.pitch, self.fov, rect.width() / height);
        let uniforms = SkyUniforms::new(
            self.history.project(),
            Projection::Basis(basis),
            pixel_angle,
        );
        ui.painter().add(
            PreviewCallback {
                uniforms,
                reduced_size,
            }
            .paint_callback(rect),
        );
    }
}

fn unit_slider(ui: &mut egui::Ui, value: &mut f32, label: &str) -> bool {
    ui.add(egui::Slider::new(value, 0.0..=1.0).text(label))
        .changed()
}

impl eframe::App for SpacegenApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_export();
        self.handle_shortcuts(&ctx);
        self.update_preview_quality(&ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.close_confirmed && self.is_dirty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.attempt(Action::Close, &ctx);
        }
        self.unsaved_dialog(&ctx);
        self.overwrite_dialog(&ctx);
        self.persist_settings();

        let title = format!(
            "{}{} - SpaceGen",
            self.path
                .as_ref()
                .and_then(|p| p.file_name())
                .map_or("Untitled".to_string(), |n| n.to_string_lossy().into_owned()),
            if self.is_dirty() { "*" } else { "" }
        );
        if title != self.shown_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.shown_title = title;
        }

        egui::Panel::left("controls")
            .default_size(380.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.project_bar(ui, &ctx);
                    ui.separator();
                    self.seed_controls(ui);
                    ui.separator();
                    self.layer_list(ui);
                    ui.separator();
                    self.selected_layer(ui);
                    ui.separator();
                    self.export_controls(ui, &ctx);
                });
            });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.label(&self.status);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            self.preview(ui);
        });
    }
}
