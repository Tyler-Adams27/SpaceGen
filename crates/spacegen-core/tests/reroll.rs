use spacegen_core::{EditError, History, LayerId, LayerKind, Project, Seed};

fn project() -> Project {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::Sun);
    project
}

/// The saved form of one Layer, for comparing whether anything about it changed.
fn saved_layer(project: &Project, index: usize) -> serde_json::Value {
    let json: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    json["layers"][index].clone()
}

#[test]
fn a_new_layer_has_variation_zero() {
    let project = project();

    assert!(project.layers().iter().all(|l| l.variation() == 0));
}

#[test]
fn variation_zero_leaves_the_layers_seed_exactly_as_before() {
    let project = project();
    let layer = &project.layers()[0];

    assert_eq!(
        layer.seed_under(project.seed()),
        project.seed().layer_seed(layer.id())
    );
}

#[test]
fn rerolling_changes_only_the_named_layers_variation_and_colors() {
    let mut project = project();
    let nebula = project.layers()[1].id();
    let before = project.clone();

    project.reroll_layer(nebula, Seed(99)).unwrap();

    assert_ne!(project.layers()[1].variation(), 0);
    assert_ne!(
        project.layers()[1].nebula_settings().unwrap().color_a,
        before.layers()[1].nebula_settings().unwrap().color_a
    );
    assert_eq!(saved_layer(&project, 0), saved_layer(&before, 0));
    assert_eq!(saved_layer(&project, 2), saved_layer(&before, 2));
    assert_eq!(project.seed(), before.seed());
}

#[test]
fn rerolling_keeps_a_layers_other_settings() {
    let mut project = project();
    let nebula = project.layers()[1].id();
    project.set_opacity(nebula, 0.4).unwrap();
    let before = project.layers()[1].nebula_settings().copied().unwrap();

    project.reroll_layer(nebula, Seed(99)).unwrap();

    let after = project.layers()[1].nebula_settings().unwrap();
    assert_eq!(project.layers()[1].opacity(), 0.4);
    assert_eq!(
        (after.coverage, after.scale, after.brightness),
        (before.coverage, before.scale, before.brightness)
    );
}

#[test]
fn different_rerolls_give_different_layer_seeds_and_the_same_one_gives_the_same() {
    let mut first = project();
    let mut second = project();
    let mut third = project();
    let id = first.layers()[0].id();

    first.reroll_layer(id, Seed(99)).unwrap();
    second.reroll_layer(id, Seed(99)).unwrap();
    third.reroll_layer(id, Seed(100)).unwrap();

    let seed_of = |p: &Project| p.layers()[0].seed_under(p.seed());
    assert_eq!(seed_of(&first), seed_of(&second));
    assert_ne!(seed_of(&first), seed_of(&third));
    assert_ne!(seed_of(&first), project().layers()[0].seed_under(Seed(5)));
}

#[test]
fn rerolling_with_the_seed_zero_still_changes_the_look() {
    let mut project = project();
    let id = project.layers()[0].id();

    project.reroll_layer(id, Seed(0)).unwrap();

    assert_ne!(project.layers()[0].variation(), 0);
}

#[test]
fn rerolling_an_unknown_layer_is_an_error_and_changes_nothing() {
    let mut project = project();
    let before = project.clone();

    let error = project.reroll_layer(LayerId(77), Seed(1)).unwrap_err();

    assert_eq!(error, EditError::UnknownLayer { layer: LayerId(77) });
    assert_eq!(project, before);
}

#[test]
fn variation_survives_a_save_and_load() {
    let mut project = project();
    let id = project.layers()[1].id();
    project.reroll_layer(id, Seed(99)).unwrap();

    let loaded = Project::from_json(&project.to_json()).unwrap();

    assert_eq!(loaded, project);
    assert_eq!(
        loaded.layers()[1].variation(),
        project.layers()[1].variation()
    );
}

#[test]
fn a_file_saved_before_variation_existed_loads_with_variation_zero() {
    let project = project();
    let mut file: serde_json::Value = serde_json::from_str(&project.to_json()).unwrap();
    for layer in file["layers"].as_array_mut().unwrap() {
        layer.as_object_mut().unwrap().remove("variation");
    }

    let loaded = Project::from_json(&file.to_string()).unwrap();

    assert_eq!(loaded, project);
}

#[test]
fn history_reroll_is_one_undo_step_that_restores_variation_and_colors() {
    let mut history = History::new(project());
    let id = history.project().layers()[1].id();
    let before = history.project().clone();

    history.reroll_layer(id, Seed(99)).unwrap();
    history.reroll_layer(id, Seed(100)).unwrap();
    history.undo();

    // Two re-rolls are two steps, never merged like a slider drag.
    assert_ne!(history.project(), &before);
    history.undo();
    assert_eq!(history.project(), &before);
}

#[test]
fn a_failed_reroll_creates_no_undo_step() {
    let mut history = History::new(project());

    assert!(history.reroll_layer(LayerId(77), Seed(1)).is_err());

    assert!(!history.can_undo());
}
