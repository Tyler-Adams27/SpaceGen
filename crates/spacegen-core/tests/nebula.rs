use spacegen_core::{LayerKind, NebulaSettings, Project, Seed};

#[test]
fn a_new_nebula_layer_has_mid_range_settings_and_a_blue_to_pink_palette() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::Nebula);

    let settings = project.layers()[0].nebula_settings().unwrap();

    assert_eq!(settings.coverage, 0.5);
    assert_eq!(settings.scale, 0.5);
    assert_eq!(settings.brightness, 0.5);
    assert_eq!(settings.color_a, [0.2, 0.4, 1.0]);
    assert_eq!(settings.color_b, [1.0, 0.3, 0.6]);
}

fn chosen() -> NebulaSettings {
    NebulaSettings {
        coverage: 0.9,
        scale: 0.2,
        brightness: 0.7,
        color_a: [0.0, 1.0, 0.5],
        color_b: [0.5, 0.0, 1.0],
    }
}

#[test]
fn set_nebula_settings_replaces_the_settings_of_only_the_named_layer() {
    let mut project = Project::new(Seed(5));
    let first = project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::Nebula);

    project.set_nebula_settings(first, chosen()).unwrap();

    assert_eq!(project.layers()[0].nebula_settings(), Some(&chosen()));
    assert_eq!(project.layers()[1].nebula_settings().unwrap().coverage, 0.5);
}

#[test]
fn nebula_settings_with_any_value_outside_zero_to_one_are_rejected_and_change_nothing() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::Nebula);
    let good = NebulaSettings::default();
    let bad = [
        (
            "coverage",
            NebulaSettings {
                coverage: 1.1,
                ..good
            },
        ),
        (
            "scale",
            NebulaSettings {
                scale: -0.1,
                ..good
            },
        ),
        (
            "brightness",
            NebulaSettings {
                brightness: f32::NAN,
                ..good
            },
        ),
        (
            "color_a",
            NebulaSettings {
                color_a: [0.0, 2.0, 0.0],
                ..good
            },
        ),
        (
            "color_b",
            NebulaSettings {
                color_b: [f32::NAN, 0.0, 0.0],
                ..good
            },
        ),
    ];

    for (field, settings) in bad {
        let error = project.set_nebula_settings(layer, settings).unwrap_err();
        assert!(
            error.to_string().contains(field),
            "{field}: unexpected message: {error}"
        );
    }

    assert_eq!(project.layers()[0].nebula_settings(), Some(&good));
}

#[test]
fn nebula_settings_cannot_be_set_on_a_layer_of_another_kind() {
    let mut project = Project::new(Seed(5));
    let stars = project.add_layer(LayerKind::StarField);

    let error = project
        .set_nebula_settings(stars, NebulaSettings::default())
        .unwrap_err();

    assert!(
        error.to_string().contains("nebula"),
        "unexpected message: {error}"
    );
    assert!(project.layers()[0].nebula_settings().is_none());
    assert_eq!(project.layers()[0].kind(), LayerKind::StarField);
}

#[test]
fn loading_a_file_with_out_of_range_nebula_settings_is_rejected_naming_the_layer() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);
    let mut file: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    file["layers"][1]["settings"]["coverage"] = serde_json::json!(1.5);

    let error = Project::from_json(&file.to_string()).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("layer 1"), "unexpected message: {message}");
    assert!(
        message.contains("coverage"),
        "unexpected message: {message}"
    );
}

#[test]
fn nebula_settings_survive_a_save_and_load() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::Nebula);
    project.set_nebula_settings(layer, chosen()).unwrap();

    let loaded = Project::from_json(&project.to_json()).unwrap();

    assert_eq!(loaded, project);
    assert_eq!(loaded.layers()[0].nebula_settings(), Some(&chosen()));
}

#[test]
fn a_duplicated_nebula_layer_keeps_the_original_settings() {
    let mut project = Project::new(Seed(5));
    let original = project.add_layer(LayerKind::Nebula);
    project.set_nebula_settings(original, chosen()).unwrap();

    project.duplicate_layer(original).unwrap();

    assert_eq!(project.layers()[1].nebula_settings(), Some(&chosen()));
}

#[test]
fn other_kinds_have_no_nebula_settings() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);

    assert!(project.layers()[0].nebula_settings().is_none());
}
