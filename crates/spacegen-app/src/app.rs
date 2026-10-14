use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use spacegen_core::{
    BlendMode, EditError, EngineConvention, HdriFormat, History, ImageFormat, LayerId, LayerKind,
    PngBitDepth, Project,
};
use spacegen_render::{
    ExportOptions, Projection, Renderer, SkyUniforms, camera_basis, export_skybox,
};

use crate::preview::{PreviewCallback, PreviewResources};
use crate::seed_source::fresh_seed;

const FACE_SIZES: [u32; 5] = [512, 1024, 2048, 4096, 8192];
const DEFAULT_FOV: f32 = 1.2;

enum ExportMessage {
    Progress(usize, usize),
    Finished(usize),
    Failed(String),
}

struct ExportJob {
    receiver: Receiver<ExportMessage>,
    done: usize,
    total: usize,
}

struct ExportSettings {
    name: String,
    folder: Option<PathBuf>,
    face_size: u32,
    format: ImageFormat,
    convention: EngineConvention,
    faces: bool,
    panorama: bool,
    hdri: bool,
    hdri_format: HdriFormat,
    png_bit_depth: PngBitDepth,
    jpeg_quality: u8,
}

impl Default for ExportSettings {
    fn default() -> Self {
        ExportSettings {
            name: "skybox".to_string(),
            folder: None,
            face_size: 2048,
            format: ImageFormat::Png,
            convention: EngineConvention::Unity,
            faces: true,
            panorama: true,
            hdri: true,
            hdri_format: HdriFormat::OpenExr,
            png_bit_depth: PngBitDepth::Sixteen,
            jpeg_quality: 95,
        }
    }
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
    export: ExportSettings,
    job: Option<ExportJob>,
    status: String,
    shown_title: String,
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
        SpacegenApp {
            saved_json: project.to_json(),
            history: History::new(project),
            selected,
            path: None,
            yaw: 0.4,
            pitch: 0.15,
            fov: DEFAULT_FOV,
            export: ExportSettings::default(),
            job: None,
            status: "Drag the sky to look around, scroll to zoom.".to_string(),
            shown_title: String::new(),
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
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SpaceGen project", &["spacegen"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| Project::from_json(&text).map_err(|e| e.to_string()))
        {
            Ok(project) => {
                self.status = format!("Opened {}.", path.display());
                self.replace_project(project, Some(path));
            }
            Err(error) => self.status = format!("Could not open {}: {error}", path.display()),
        }
    }

    fn save_project(&mut self, ask_for_path: bool) {
        let path = match (&self.path, ask_for_path) {
            (Some(path), false) => path.clone(),
            _ => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("SpaceGen project", &["spacegen"])
                    .set_file_name("sky.spacegen")
                    .save_file()
                else {
                    return;
                };
                path
            }
        };
        let json = self.history.project().to_json();
        match std::fs::write(&path, &json) {
            Ok(()) => {
                self.status = format!("Saved {}.", path.display());
                self.saved_json = json;
                self.path = Some(path);
            }
            Err(error) => self.status = format!("Could not save {}: {error}", path.display()),
        }
    }

    // ---- export ------------------------------------------------------------

    fn start_export(&mut self, ctx: &egui::Context) {
        let Some(folder) = self.export.folder.clone() else {
            return;
        };
        let project = self.history.project().clone();
        let options = ExportOptions {
            name: self.export.name.clone(),
            face_size: self.export.face_size,
            format: self.export.format,
            convention: self.export.convention,
            faces: self.export.faces,
            panorama: self.export.panorama,
            hdri: self.export.hdri,
            hdri_format: self.export.hdri_format,
            png_bit_depth: self.export.png_bit_depth,
            jpeg_quality: self.export.jpeg_quality,
        };
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let send = |message| {
                let _ = sender.send(message);
                ctx.request_repaint();
            };
            let renderer = match Renderer::gpu() {
                Ok(renderer) => renderer,
                Err(error) => return send(ExportMessage::Failed(error.to_string())),
            };
            let result = export_skybox(&renderer, &project, &folder, &options, |done, total| {
                send(ExportMessage::Progress(done, total))
            });
            send(match result {
                Ok(files) => ExportMessage::Finished(files.len()),
                Err(error) => ExportMessage::Failed(error.to_string()),
            });
        });
        self.job = Some(ExportJob {
            receiver,
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
        while let Ok(message) = job.receiver.try_recv() {
            match message {
                ExportMessage::Progress(done, total) => {
                    job.done = done;
                    job.total = total.max(1);
                }
                ExportMessage::Finished(count) => {
                    finished = Some(format!("Exported {count} files."))
                }
                ExportMessage::Failed(error) => finished = Some(format!("Export failed: {error}")),
            }
        }
        if let Some(message) = finished {
            self.status = message;
            self.job = None;
        }
    }

    fn can_export(&self) -> bool {
        self.job.is_none()
            && self.export.folder.is_some()
            && !self.export.name.trim().is_empty()
            && (self.export.faces || self.export.panorama || self.export.hdri)
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

    fn project_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("New").clicked() {
                self.new_project();
            }
            if ui.button("Open...").clicked() {
                self.open_project();
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
        ui.label("Layers (top first)");
        for (index, &(id, kind, visible)) in rows.iter().enumerate().rev() {
            ui.horizontal(|ui| {
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
                if ui.button("Copy").clicked() {
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
        }
        ui.horizontal(|ui| {
            if ui.button("Add star field").clicked() {
                self.selected = Some(self.history.add_layer(LayerKind::StarField));
            }
            if ui.button("Add nebula").clicked() {
                self.selected = Some(self.history.add_layer(LayerKind::Nebula));
            }
            if ui.button("Add galaxy band").clicked() {
                self.selected = Some(self.history.add_layer(LayerKind::GalaxyBand));
            }
            if ui.button("Add sun").clicked() {
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
            ui.text_edit_singleline(&mut self.export.name);
        });
        ui.horizontal(|ui| {
            if ui.button("Folder...").clicked()
                && let Some(folder) = rfd::FileDialog::new().pick_folder()
            {
                self.export.folder = Some(folder);
            }
            match &self.export.folder {
                Some(folder) => ui.label(folder.display().to_string()),
                None => ui.label("choose where to save"),
            };
        });
        egui::ComboBox::from_label("Face size")
            .selected_text(format!("{0} x {0}", self.export.face_size))
            .show_ui(ui, |ui| {
                for size in FACE_SIZES {
                    ui.selectable_value(
                        &mut self.export.face_size,
                        size,
                        format!("{size} x {size}"),
                    );
                }
            });
        ui.horizontal(|ui| {
            ui.label("Format");
            ui.selectable_value(&mut self.export.format, ImageFormat::Png, "PNG");
            ui.selectable_value(&mut self.export.format, ImageFormat::Jpeg, "JPEG");
        });
        if self.export.format == ImageFormat::Png {
            ui.horizontal(|ui| {
                ui.label("PNG depth");
                ui.selectable_value(
                    &mut self.export.png_bit_depth,
                    PngBitDepth::Sixteen,
                    "16-bit",
                );
                ui.selectable_value(&mut self.export.png_bit_depth, PngBitDepth::Eight, "8-bit");
            });
        }
        if self.export.format == ImageFormat::Jpeg {
            ui.add(egui::Slider::new(&mut self.export.jpeg_quality, 50..=100).text("JPEG quality"));
        }
        egui::ComboBox::from_label("Face names for")
            .selected_text(convention_name(self.export.convention))
            .show_ui(ui, |ui| {
                for convention in [
                    EngineConvention::Unity,
                    EngineConvention::Unreal,
                    EngineConvention::Godot,
                ] {
                    ui.selectable_value(
                        &mut self.export.convention,
                        convention,
                        convention_name(convention),
                    );
                }
            });
        ui.checkbox(&mut self.export.faces, "Six cube faces");
        ui.checkbox(
            &mut self.export.panorama,
            "Panorama image (equirectangular)",
        );
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.export.hdri, "HDRI");
            ui.add_enabled_ui(self.export.hdri, |ui| {
                ui.selectable_value(&mut self.export.hdri_format, HdriFormat::OpenExr, ".exr");
                ui.selectable_value(&mut self.export.hdri_format, HdriFormat::Radiance, ".hdr");
            });
        });

        if ui
            .add_enabled(self.can_export(), egui::Button::new("Export"))
            .clicked()
        {
            self.start_export(ctx);
        }
        if let Some(job) = &self.job {
            ui.add(
                egui::ProgressBar::new(job.done as f32 / job.total as f32)
                    .text(format!("{} of {}", job.done, job.total)),
            );
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
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
        let pixel_angle = self.fov / (height * ui.ctx().pixels_per_point());
        let basis = camera_basis(self.yaw, self.pitch, self.fov, rect.width() / height);
        let uniforms = SkyUniforms::new(
            self.history.project(),
            Projection::Basis(basis),
            pixel_angle,
        );
        ui.painter()
            .add(PreviewCallback { uniforms }.paint_callback(rect));
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
                    self.project_bar(ui);
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
