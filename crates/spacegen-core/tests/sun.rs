use spacegen_core::{History, LayerKind, Project, Seed, SunSettings};

#[test]
fn a_new_sun_layer_sits_in_front_just_above_the_horizon_with_a_warm_white_color() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::Sun);

    let settings = project.layers()[0].sun_settings().unwrap();

    assert_eq!(settings.azimuth, 0.0);
    assert_eq!(settings.elevation, 0.6);
    assert_eq!(settings.size, 0.5);
    assert_eq!(settings.brightness, 0.5);
    assert_eq!(settings.glare, 0.5);
    assert_eq!(settings.color, [1.0, 0.95, 0.85]);
}

fn chosen() -> SunSettings {
    SunSettings {
        azimuth: 0.25,
        elevation: 0.8,
        size: 0.2,
        brightness: 0.9,
        glare: 0.1,
        color: [1.0, 0.5, 0.0],
    }
}

#[test]
fn other_kinds_have_no_sun_settings() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);

    assert!(project.layers()[0].sun_settings().is_none());
}

#[test]
fn set_sun_settings_replaces_the_settings_of_only_the_named_layer() {
    let mut project = Project::new(Seed(5));
    let first = project.add_layer(LayerKind::Sun);
    project.add_layer(LayerKind::Sun);

    project.set_sun_settings(first, chosen()).unwrap();

    assert_eq!(project.layers()[0].sun_settings(), Some(&chosen()));
    assert_eq!(project.layers()[1].sun_settings().unwrap().azimuth, 0.0);
}

#[test]
fn sun_settings_cannot_be_set_on_a_layer_of_another_kind() {
    let mut project = Project::new(Seed(5));
    let stars = project.add_layer(LayerKind::StarField);

    let error = project
        .set_sun_settings(stars, SunSettings::default())
        .unwrap_err();

    assert!(
        error.to_string().contains("sun"),
        "unexpected message: {error}"
    );
    assert!(project.layers()[0].sun_settings().is_none());
}

#[test]
fn sun_settings_with_any_value_outside_zero_to_one_are_rejected_and_change_nothing() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::Sun);
    let good = SunSettings::default();
    let bad = [
        (
            "azimuth",
            SunSettings {
                azimuth: 1.1,
                ..good
            },
        ),
        (
            "elevation",
            SunSettings {
                elevation: -0.1,
                ..good
            },
        ),
        ("size", SunSettings { size: 2.0, ..good }),
        (
            "brightness",
            SunSettings {
                brightness: f32::NAN,
                ..good
            },
        ),
        (
            "glare",
            SunSettings {
                glare: -1.0,
                ..good
            },
        ),
        (
            "color",
            SunSettings {
                color: [0.0, 2.0, 0.0],
                ..good
            },
        ),
    ];

    for (field, settings) in bad {
        let error = project.set_sun_settings(layer, settings).unwrap_err();
        assert!(
            error.to_string().contains(field),
            "{field}: unexpected message: {error}"
        );
    }

    assert_eq!(project.layers()[0].sun_settings(), Some(&good));
}

#[test]
fn loading_a_file_with_out_of_range_sun_settings_is_rejected_naming_the_layer() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Sun);
    let mut file: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    file["layers"][1]["settings"]["glare"] = serde_json::json!(1.5);

    let error = Project::from_json(&file.to_string()).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("layer 1"), "unexpected message: {message}");
    assert!(message.contains("glare"), "unexpected message: {message}");
}

#[test]
fn sun_settings_survive_a_save_and_load() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::Sun);
    project.set_sun_settings(layer, chosen()).unwrap();

    let loaded = Project::from_json(&project.to_json()).unwrap();

    assert_eq!(loaded, project);
    assert_eq!(loaded.layers()[0].sun_settings(), Some(&chosen()));
}

#[test]
fn a_file_saved_before_the_sun_had_settings_loads_with_the_defaults() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::Sun);
    let mut file: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    file["layers"][0]["settings"] = serde_json::json!({ "kind": "sun" });

    let loaded = Project::from_json(&file.to_string()).unwrap();

    assert_eq!(
        loaded.layers()[0].sun_settings(),
        Some(&SunSettings::default())
    );
}

#[test]
fn a_duplicated_sun_layer_keeps_the_original_settings() {
    let mut project = Project::new(Seed(5));
    let original = project.add_layer(LayerKind::Sun);
    project.set_sun_settings(original, chosen()).unwrap();

    project.duplicate_layer(original).unwrap();

    assert_eq!(project.layers()[1].sun_settings(), Some(&chosen()));
}

#[test]
fn sun_changes_undo_and_redo_and_a_drag_is_one_step() {
    let mut history = History::new(Project::new(Seed(5)));
    let layer = history.add_layer(LayerKind::Sun);
    let with_size = |size| SunSettings {
        size,
        ..SunSettings::default()
    };
    let size = |history: &History| history.project().layers()[0].sun_settings().unwrap().size;

    for value in [0.6, 0.7, 0.8] {
        history.set_sun_settings(layer, with_size(value)).unwrap();
    }
    assert_eq!(size(&history), 0.8);

    history.undo();
    assert_eq!(size(&history), 0.5);
    history.redo();
    assert_eq!(size(&history), 0.8);
}
