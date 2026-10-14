use spacegen_core::{BlendMode, LayerId, LayerKind, Project, Seed};

#[test]
fn added_layers_are_listed_bottom_first_with_default_settings() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);

    let layers = project.layers();

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].id(), LayerId(0));
    assert_eq!(layers[0].kind(), LayerKind::StarField);
    assert_eq!(layers[1].id(), LayerId(1));
    assert_eq!(layers[1].kind(), LayerKind::Nebula);
    assert_eq!(layers[1].blend(), BlendMode::Additive);
    assert_eq!(layers[1].opacity(), 1.0);
    assert!(layers[1].visible());
}

#[test]
fn set_opacity_changes_only_the_named_layer() {
    let mut project = Project::new(Seed(5));
    let first = project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);

    project.set_opacity(first, 0.25).unwrap();

    assert_eq!(project.layers()[0].opacity(), 0.25);
    assert_eq!(project.layers()[1].opacity(), 1.0);
}

#[test]
fn set_opacity_rejects_values_outside_zero_to_one_and_leaves_the_layer_unchanged() {
    let mut project = Project::new(Seed(5));
    let layer = project.add_layer(LayerKind::StarField);

    assert!(project.set_opacity(layer, 1.5).is_err());
    assert!(project.set_opacity(layer, -0.1).is_err());
    assert!(project.set_opacity(layer, f32::NAN).is_err());

    assert_eq!(project.layers()[0].opacity(), 1.0);
}

#[test]
fn editing_a_layer_that_does_not_exist_is_an_error_naming_it() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);

    let error = project.set_opacity(LayerId(99), 0.5).unwrap_err();

    assert!(
        error.to_string().contains("layer 99"),
        "unexpected message: {error}"
    );
}

#[test]
fn set_blend_changes_only_the_named_layer() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    let second = project.add_layer(LayerKind::Nebula);

    project.set_blend(second, BlendMode::Screen).unwrap();

    assert_eq!(project.layers()[0].blend(), BlendMode::Additive);
    assert_eq!(project.layers()[1].blend(), BlendMode::Screen);
}

#[test]
fn set_visible_hides_and_shows_only_the_named_layer() {
    let mut project = Project::new(Seed(5));
    let first = project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);

    project.set_visible(first, false).unwrap();
    assert!(!project.layers()[0].visible());
    assert!(project.layers()[1].visible());

    project.set_visible(first, true).unwrap();
    assert!(project.layers()[0].visible());
}

#[test]
fn remove_layer_takes_it_out_of_the_stack_and_never_reuses_its_id() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    let middle = project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::Sun);

    project.remove_layer(middle).unwrap();

    let ids: Vec<LayerId> = project.layers().iter().map(|l| l.id()).collect();
    assert_eq!(ids, [LayerId(0), LayerId(2)]);
    assert_eq!(project.add_layer(LayerKind::GalaxyBand), LayerId(3));
    assert!(project.remove_layer(middle).is_err());
}

fn ids(project: &Project) -> Vec<LayerId> {
    project.layers().iter().map(|l| l.id()).collect()
}

#[test]
fn move_layer_reorders_the_stack_bottom_first() {
    let mut project = Project::new(Seed(5));
    let bottom = project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::Sun);

    project.move_layer(bottom, 2).unwrap();
    assert_eq!(ids(&project), [LayerId(1), LayerId(2), LayerId(0)]);

    project.move_layer(bottom, 0).unwrap();
    assert_eq!(ids(&project), [LayerId(0), LayerId(1), LayerId(2)]);
}

#[test]
fn move_layer_to_an_index_past_the_top_is_an_error_and_leaves_the_stack_unchanged() {
    let mut project = Project::new(Seed(5));
    let bottom = project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);

    let error = project.move_layer(bottom, 2).unwrap_err();

    assert!(
        error.to_string().contains("index 2"),
        "unexpected message: {error}"
    );
    assert_eq!(ids(&project), [LayerId(0), LayerId(1)]);
}

#[test]
fn duplicate_layer_copies_settings_places_the_copy_above_and_gives_it_a_new_id() {
    let mut project = Project::new(Seed(5));
    let original = project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);
    project.set_opacity(original, 0.5).unwrap();
    project.set_blend(original, BlendMode::Screen).unwrap();
    project.set_visible(original, false).unwrap();

    let copy = project.duplicate_layer(original).unwrap();

    assert_eq!(copy, LayerId(2));
    assert_eq!(ids(&project), [LayerId(0), LayerId(2), LayerId(1)]);
    let copied = &project.layers()[1];
    assert_eq!(copied.kind(), LayerKind::StarField);
    assert_eq!(copied.blend(), BlendMode::Screen);
    assert_eq!(copied.opacity(), 0.5);
    assert!(!copied.visible());
}
