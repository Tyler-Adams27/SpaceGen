//! Drives the real window (every panel, button and slider) with `egui_kittest`, without a GPU
//! window: the Preview's paint callback is recorded but not drawn.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

/// A fresh app on a temporary settings file, in a harness the size of the real window.
fn harness(label: &str) -> (Harness<'static, SpacegenApp>, PathBuf) {
    let dir = std::env::temp_dir().join(format!("spacegen-ui-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app = SpacegenApp::with_settings_path(Some(dir.join("settings.json")));
    let harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut SpacegenApp| app.show(ui), app);
    (harness, dir)
}

#[test]
fn the_window_opens_with_the_starter_sky() {
    let (mut harness, _) = harness("opens");

    harness.run();

    assert_eq!(harness.state().history.project().layers().len(), 2);
    harness.get_by_label("Add galaxy band");
    harness.get_by_label("Randomize");
}

fn layer_kinds(harness: &Harness<'_, SpacegenApp>) -> Vec<LayerKind> {
    harness
        .state()
        .history
        .project()
        .layers()
        .iter()
        .map(|l| l.kind())
        .collect()
}

#[test]
fn add_layer_buttons_add_a_layer_on_top_and_select_it() {
    let (mut harness, _) = harness("add-layer");
    harness.run();

    harness.get_by_label("Add galaxy band").click();
    harness.run();
    harness.get_by_label("Add sun").click();
    harness.run();

    assert_eq!(
        layer_kinds(&harness),
        [
            LayerKind::Nebula,
            LayerKind::StarField,
            LayerKind::GalaxyBand,
            LayerKind::Sun
        ]
    );
    let newest = harness
        .state()
        .history
        .project()
        .layers()
        .last()
        .unwrap()
        .id();
    assert_eq!(harness.state().selected, Some(newest));
}

#[test]
fn moving_a_slider_edits_the_selected_layer_and_undo_puts_it_back() {
    let (mut harness, _) = harness("slider-undo");
    harness.run();
    let coverage = |h: &Harness<'_, SpacegenApp>| {
        h.state().history.project().layers()[0]
            .nebula_settings()
            .unwrap()
            .coverage
    };
    let before = coverage(&harness);

    harness
        .get_by_role_and_label(egui::accesskit::Role::Slider, "Coverage")
        .focus();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    let after = coverage(&harness);
    assert!(after > before, "coverage {before} -> {after}");

    harness.get_by_label("Undo").click();
    harness.run();
    assert_eq!(coverage(&harness), before);
    harness.get_by_label("Redo").click();
    harness.run();
    assert_eq!(coverage(&harness), after);
}

#[test]
fn save_new_and_open_from_recent_bring_the_project_back() {
    let (mut harness, dir) = harness("save-open");
    harness.run();
    let file = dir.join("sky.spacegen");
    harness.state_mut().path = Some(file.clone());
    harness.get_by_label("Add sun").click();
    harness.run();
    let saved = harness.state().history.project().to_json();

    harness.get_by_label("Save").click();
    harness.run();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), saved);
    assert!(!harness.state().is_dirty());

    harness.get_by_label("New").click();
    harness.run();
    assert_eq!(harness.state().history.project().layers().len(), 2);

    harness.get_by_label_contains("Recent").click();
    harness.run();
    harness.get_by_label("sky.spacegen").click();
    harness.run();
    assert_eq!(harness.state().history.project().to_json(), saved);
    assert_eq!(harness.state().path.as_deref(), Some(file.as_path()));
}

#[test]
fn new_with_unsaved_changes_asks_first_and_cancel_keeps_the_project() {
    let (mut harness, _) = harness("unsaved");
    harness.run();
    harness.get_by_label("Add sun").click();
    harness.run();

    harness.get_by_label("New").click();
    harness.run();
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().history.project().layers().len(), 3);

    harness.get_by_label("New").click();
    harness.run();
    harness.get_by_label("Discard").click();
    harness.run();
    assert_eq!(harness.state().history.project().layers().len(), 2);
}

#[test]
fn the_flat_panorama_button_switches_the_preview() {
    let (mut harness, _) = harness("flat");
    harness.run();
    assert_eq!(harness.state().preview_mode, PreviewMode::FreeLook);

    harness.get_by_label("Flat panorama").click();
    harness.run();
    assert_eq!(harness.state().preview_mode, PreviewMode::Flat);
    harness.get_by_label("Free look").click();
    harness.run();
    assert_eq!(harness.state().preview_mode, PreviewMode::FreeLook);
}

#[test]
fn adding_stops_at_sixteen_visible_layers_with_a_note() {
    let (mut harness, _) = harness("cap");
    harness.run();
    for _ in 0..14 {
        harness.get_by_label("Add sun").click();
        harness.run();
    }
    assert_eq!(harness.state().history.project().layers().len(), 16);
    harness.get_by_label("16 of 16 visible Layers");

    // The button is disabled now: clicking it adds nothing.
    harness.get_by_label("Add sun").click();
    harness.run();
    assert_eq!(harness.state().history.project().layers().len(), 16);
}

/// Faces only at the smallest size, into `dir/out`, so the test export is quick.
fn small_export_into(harness: &mut Harness<'_, SpacegenApp>, dir: &std::path::Path) -> PathBuf {
    let out = dir.join("out");
    let export = &mut harness.state_mut().settings.export;
    export.folder = Some(out.clone());
    export.name = "t".to_string();
    export.face_size = 512;
    export.panorama = false;
    export.hdri = false;
    out
}

/// Steps the window until the background export has reported back.
fn wait_for_export(harness: &mut Harness<'_, SpacegenApp>) {
    for _ in 0..2400 {
        harness.step();
        if harness.state().job.is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("the export did not finish in time");
}

fn png_count(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter(|e| {
                    e.as_ref()
                        .unwrap()
                        .path()
                        .extension()
                        .is_some_and(|x| x == "png")
                })
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn export_writes_the_faces_to_the_chosen_folder_and_reports_it() {
    let (mut harness, dir) = harness("export");
    harness.run();
    let out = small_export_into(&mut harness, &dir);

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    harness.step();
    wait_for_export(&mut harness);

    assert_eq!(png_count(&out), 6);
    assert_eq!(harness.state().status, "Exported 6 files.");
}

#[test]
fn a_second_export_asks_before_replacing_and_cancel_leaves_the_files_alone() {
    let (mut harness, dir) = harness("replace");
    harness.run();
    let out = small_export_into(&mut harness, &dir);
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    harness.step();
    wait_for_export(&mut harness);
    let first = std::fs::read(out.join("t_Up.png")).unwrap();
    // A different sky, so a replaced file would differ from the first.
    harness.get_by_label("Randomize").click();
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    harness.run();
    harness.get_by_label("Replace existing files?");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(harness.state().job.is_none());
    assert_eq!(std::fs::read(out.join("t_Up.png")).unwrap(), first);

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    harness.run();
    harness.get_by_label("Replace").click();
    harness.step();
    wait_for_export(&mut harness);
    assert_ne!(std::fs::read(out.join("t_Up.png")).unwrap(), first);
    assert_eq!(png_count(&out), 6);
}

#[test]
fn export_stays_disabled_until_a_folder_is_chosen() {
    let (mut harness, _) = harness("no-folder");
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    harness.run();

    assert!(harness.state().job.is_none());
}
