use spacegen_core::{LayerKind, Project, Seed, StarFieldSettings};

#[test]
fn a_new_star_field_layer_has_mid_range_settings_and_a_white_tint() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);

    let settings = project.layers()[0].star_field_settings().unwrap();

    assert_eq!(settings.density, 0.5);
    assert_eq!(settings.brightness, 0.5);
    assert_eq!(settings.size, 0.5);
    assert_eq!(settings.color_variation, 0.5);
    assert_eq!(settings.tint, [1.0, 1.0, 1.0]);
}

#[test]
fn set_star_field_settings_replaces_the_settings_of_only_the_named_layer() {
    let mut project = Project::new(Seed(5));
    let first = project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::StarField);
    let chosen = StarFieldSettings {
        density: 0.9,
        brightness: 0.1,
        size: 0.25,
        color_variation: 0.0,
        tint: [0.5, 0.75, 1.0],
    };

    project.set_star_field_settings(first, chosen).unwrap();

    assert_eq!(project.layers()[0].star_field_settings(), Some(&chosen));
    assert_eq!(
        project.layers()[1].star_field_settings().unwrap().density,
        0.5
    );
}

#[test]
fn star_field_settings_with_any_value_outside_zero_to_one_are_rejected_and_change_nothing() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::StarField);
    let good = StarFieldSettings::default();
    let bad = [
        (
            "density",
            StarFieldSettings {
                density: 1.1,
                ..good
            },
        ),
        (
            "brightness",
            StarFieldSettings {
                brightness: -0.1,
                ..good
            },
        ),
        (
            "size",
            StarFieldSettings {
                size: f32::NAN,
                ..good
            },
        ),
        (
            "color_variation",
            StarFieldSettings {
                color_variation: 2.0,
                ..good
            },
        ),
        (
            "tint",
            StarFieldSettings {
                tint: [1.0, 1.5, 1.0],
                ..good
            },
        ),
        (
            "tint",
            StarFieldSettings {
                tint: [f32::NAN, 1.0, 1.0],
                ..good
            },
        ),
    ];

    for (field, settings) in bad {
        let error = project
            .set_star_field_settings(layer, settings)
            .unwrap_err();
        assert!(
            error.to_string().contains(field),
            "{field}: unexpected message: {error}"
        );
    }

    assert_eq!(project.layers()[0].star_field_settings(), Some(&good));
}

#[test]
fn star_field_settings_cannot_be_set_on_a_layer_of_another_kind() {
    let mut project = Project::new(Seed(5));
    let nebula = project.add_layer(LayerKind::Nebula);

    let error = project
        .set_star_field_settings(nebula, StarFieldSettings::default())
        .unwrap_err();

    assert!(
        error.to_string().contains("star field"),
        "unexpected message: {error}"
    );
    assert_eq!(project.layers()[0].kind(), LayerKind::Nebula);
    assert!(project.layers()[0].star_field_settings().is_none());
}

#[test]
fn star_field_settings_survive_a_save_and_load() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Sun);
    let chosen = StarFieldSettings {
        density: 0.9,
        brightness: 0.1,
        size: 0.25,
        color_variation: 0.0,
        tint: [0.5, 0.75, 1.0],
    };
    project.set_star_field_settings(layer, chosen).unwrap();

    let loaded = Project::from_json(&project.to_json()).unwrap();

    assert_eq!(loaded, project);
    assert_eq!(loaded.layers()[0].star_field_settings(), Some(&chosen));
    assert_eq!(loaded.layers()[1].kind(), LayerKind::Sun);
}

#[test]
fn loading_a_file_with_out_of_range_star_field_settings_is_rejected_naming_the_layer() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::StarField);
    let mut file: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    file["layers"][1]["settings"]["density"] = serde_json::json!(1.5);

    let error = Project::from_json(&file.to_string()).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("layer 1"), "unexpected message: {message}");
    assert!(message.contains("density"), "unexpected message: {message}");
}

#[test]
fn a_duplicated_star_field_layer_keeps_the_original_settings() {
    let mut project = Project::new(Seed(5));
    let original = project.add_layer(LayerKind::StarField);
    let chosen = StarFieldSettings {
        density: 0.9,
        tint: [0.5, 0.75, 1.0],
        ..StarFieldSettings::default()
    };
    project.set_star_field_settings(original, chosen).unwrap();

    project.duplicate_layer(original).unwrap();

    assert_eq!(project.layers()[1].star_field_settings(), Some(&chosen));
}

#[test]
fn layers_of_other_kinds_have_no_star_field_settings() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::Nebula);

    assert!(project.layers()[0].star_field_settings().is_none());
}
