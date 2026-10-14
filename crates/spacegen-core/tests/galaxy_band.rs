use spacegen_core::{GalaxyBandSettings, History, LayerKind, Project, Seed};

#[test]
fn a_new_galaxy_band_layer_has_mid_range_settings_and_a_warm_core_with_cool_edges() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::GalaxyBand);

    let settings = project.layers()[0].galaxy_band_settings().unwrap();

    assert_eq!(settings.tilt, 0.3);
    assert_eq!(settings.width, 0.4);
    assert_eq!(settings.brightness, 0.5);
    assert_eq!(settings.dust, 0.5);
    assert_eq!(settings.color_a, [1.0, 0.85, 0.6]);
    assert_eq!(settings.color_b, [0.5, 0.6, 1.0]);
}

fn chosen() -> GalaxyBandSettings {
    GalaxyBandSettings {
        tilt: 0.8,
        width: 0.2,
        brightness: 0.7,
        dust: 0.9,
        color_a: [1.0, 0.0, 0.5],
        color_b: [0.0, 0.5, 1.0],
    }
}

#[test]
fn other_kinds_have_no_galaxy_band_settings() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);

    assert!(project.layers()[0].galaxy_band_settings().is_none());
}

#[test]
fn set_galaxy_band_settings_replaces_the_settings_of_only_the_named_layer() {
    let mut project = Project::new(Seed(5));
    let first = project.add_layer(LayerKind::GalaxyBand);
    project.add_layer(LayerKind::GalaxyBand);

    project.set_galaxy_band_settings(first, chosen()).unwrap();

    assert_eq!(project.layers()[0].galaxy_band_settings(), Some(&chosen()));
    assert_eq!(
        project.layers()[1].galaxy_band_settings().unwrap().tilt,
        0.3
    );
}

#[test]
fn galaxy_band_settings_cannot_be_set_on_a_layer_of_another_kind() {
    let mut project = Project::new(Seed(5));
    let stars = project.add_layer(LayerKind::StarField);

    let error = project
        .set_galaxy_band_settings(stars, GalaxyBandSettings::default())
        .unwrap_err();

    assert!(
        error.to_string().contains("galaxy band"),
        "unexpected message: {error}"
    );
    assert!(project.layers()[0].galaxy_band_settings().is_none());
}

#[test]
fn galaxy_band_settings_with_any_value_outside_zero_to_one_are_rejected_and_change_nothing() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::GalaxyBand);
    let good = GalaxyBandSettings::default();
    let bad = [
        ("tilt", GalaxyBandSettings { tilt: 1.1, ..good }),
        (
            "width",
            GalaxyBandSettings {
                width: -0.1,
                ..good
            },
        ),
        (
            "brightness",
            GalaxyBandSettings {
                brightness: f32::NAN,
                ..good
            },
        ),
        ("dust", GalaxyBandSettings { dust: 2.0, ..good }),
        (
            "color_a",
            GalaxyBandSettings {
                color_a: [0.0, 2.0, 0.0],
                ..good
            },
        ),
        (
            "color_b",
            GalaxyBandSettings {
                color_b: [f32::NAN, 0.0, 0.0],
                ..good
            },
        ),
    ];

    for (field, settings) in bad {
        let error = project
            .set_galaxy_band_settings(layer, settings)
            .unwrap_err();
        assert!(
            error.to_string().contains(field),
            "{field}: unexpected message: {error}"
        );
    }

    assert_eq!(project.layers()[0].galaxy_band_settings(), Some(&good));
}

#[test]
fn loading_a_file_with_out_of_range_galaxy_band_settings_is_rejected_naming_the_layer() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::GalaxyBand);
    let mut file: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    file["layers"][1]["settings"]["dust"] = serde_json::json!(1.5);

    let error = Project::from_json(&file.to_string()).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("layer 1"), "unexpected message: {message}");
    assert!(message.contains("dust"), "unexpected message: {message}");
}

#[test]
fn galaxy_band_settings_survive_a_save_and_load() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::GalaxyBand);
    project.set_galaxy_band_settings(layer, chosen()).unwrap();

    let loaded = Project::from_json(&project.to_json()).unwrap();

    assert_eq!(loaded, project);
    assert_eq!(loaded.layers()[0].galaxy_band_settings(), Some(&chosen()));
}

#[test]
fn a_file_saved_before_the_galaxy_band_had_settings_loads_with_the_defaults() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::GalaxyBand);
    let mut file: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    file["layers"][0]["settings"] = serde_json::json!({ "kind": "galaxy_band" });

    let loaded = Project::from_json(&file.to_string()).unwrap();

    assert_eq!(
        loaded.layers()[0].galaxy_band_settings(),
        Some(&GalaxyBandSettings::default())
    );
}

#[test]
fn a_duplicated_galaxy_band_layer_keeps_the_original_settings() {
    let mut project = Project::new(Seed(5));
    let original = project.add_layer(LayerKind::GalaxyBand);
    project
        .set_galaxy_band_settings(original, chosen())
        .unwrap();

    project.duplicate_layer(original).unwrap();

    assert_eq!(project.layers()[1].galaxy_band_settings(), Some(&chosen()));
}

#[test]
fn galaxy_band_changes_undo_and_redo_and_a_drag_is_one_step() {
    let mut history = History::new(Project::new(Seed(5)));
    let layer = history.add_layer(LayerKind::GalaxyBand);
    let with_width = |width| GalaxyBandSettings {
        width,
        ..GalaxyBandSettings::default()
    };
    let width = |history: &History| {
        history.project().layers()[0]
            .galaxy_band_settings()
            .unwrap()
            .width
    };

    for value in [0.6, 0.7, 0.8] {
        history
            .set_galaxy_band_settings(layer, with_width(value))
            .unwrap();
    }
    assert_eq!(width(&history), 0.8);

    history.undo();
    assert_eq!(width(&history), 0.4);
    history.redo();
    assert_eq!(width(&history), 0.8);
}
