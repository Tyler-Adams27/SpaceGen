use spacegen_core::{
    BlendMode, History, LayerId, LayerKind, NebulaSettings, Project, Seed, StarFieldSettings,
};

fn fresh() -> History {
    History::new(Project::new(Seed(5)))
}

#[test]
fn undo_reverts_an_added_layer() {
    let mut history = fresh();
    history.add_layer(LayerKind::StarField);
    assert!(history.can_undo());

    history.undo();

    assert!(history.project().layers().is_empty());
}

#[test]
fn redo_reapplies_an_undone_edit() {
    let mut history = fresh();
    history.add_layer(LayerKind::StarField);
    history.undo();
    assert!(history.can_redo());

    history.redo();

    assert_eq!(history.project().layers().len(), 1);
    assert!(!history.can_redo());
}

#[test]
fn a_new_edit_after_an_undo_discards_the_redo_history() {
    let mut history = fresh();
    history.add_layer(LayerKind::StarField);
    history.undo();

    history.add_layer(LayerKind::Nebula);

    assert!(!history.can_redo());
}

#[test]
fn a_failed_edit_creates_no_undo_step_and_keeps_the_redo_history() {
    let mut history = fresh();
    let layer = history.add_layer(LayerKind::StarField);
    history.add_layer(LayerKind::Nebula);
    history.undo();

    assert!(history.set_opacity(layer, 1.5).is_err());

    assert!(history.can_redo());
    history.undo();
    assert!(history.project().layers().is_empty());
    assert!(!history.can_undo());
}

#[test]
fn undo_and_redo_cover_blend_changes() {
    let mut history = fresh();
    let layer = history.add_layer(LayerKind::StarField);

    history.set_blend(layer, BlendMode::Screen).unwrap();
    assert_eq!(history.project().layers()[0].blend(), BlendMode::Screen);

    history.undo();
    assert_eq!(history.project().layers()[0].blend(), BlendMode::Additive);
    history.redo();
    assert_eq!(history.project().layers()[0].blend(), BlendMode::Screen);
}

#[test]
fn undo_and_redo_cover_visibility_changes() {
    let mut history = fresh();
    let layer = history.add_layer(LayerKind::StarField);

    history.set_visible(layer, false).unwrap();
    assert!(!history.project().layers()[0].visible());

    history.undo();
    assert!(history.project().layers()[0].visible());
    history.redo();
    assert!(!history.project().layers()[0].visible());
}

#[test]
fn undoing_a_removal_restores_the_layer_at_its_position() {
    let mut history = fresh();
    history.add_layer(LayerKind::StarField);
    let middle = history.add_layer(LayerKind::Nebula);
    history.add_layer(LayerKind::Sun);

    history.remove_layer(middle).unwrap();
    assert_eq!(history.project().layers().len(), 2);

    history.undo();
    let ids: Vec<LayerId> = history.project().layers().iter().map(|l| l.id()).collect();
    assert_eq!(ids, [LayerId(0), LayerId(1), LayerId(2)]);
}

#[test]
fn undoing_a_move_restores_the_previous_order() {
    let mut history = fresh();
    let bottom = history.add_layer(LayerKind::StarField);
    history.add_layer(LayerKind::Nebula);

    history.move_layer(bottom, 1).unwrap();
    assert_eq!(history.project().layers()[1].id(), LayerId(0));

    history.undo();
    assert_eq!(history.project().layers()[0].id(), LayerId(0));
}

#[test]
fn undoing_a_duplicate_removes_the_copy() {
    let mut history = fresh();
    let original = history.add_layer(LayerKind::StarField);

    let copy = history.duplicate_layer(original).unwrap();
    assert_eq!(copy, LayerId(1));
    assert_eq!(history.project().layers().len(), 2);

    history.undo();
    assert_eq!(history.project().layers().len(), 1);
}

#[test]
fn a_loaded_project_starts_with_an_empty_history() {
    let mut saved = Project::new(Seed(5));
    saved.add_layer(LayerKind::StarField);
    let loaded = Project::from_json(&saved.to_json()).unwrap();

    let mut history = History::new(loaded);

    assert!(!history.can_undo());
    assert!(!history.can_redo());
    history.undo();
    assert_eq!(history.project().layers().len(), 1);
}

#[test]
fn undo_and_redo_cover_star_field_settings() {
    let mut history = fresh();
    let layer = history.add_layer(LayerKind::StarField);
    let chosen = StarFieldSettings {
        density: 0.9,
        ..StarFieldSettings::default()
    };

    history.set_star_field_settings(layer, chosen).unwrap();
    assert_eq!(
        history.project().layers()[0].star_field_settings(),
        Some(&chosen)
    );

    history.undo();
    assert_eq!(
        history.project().layers()[0].star_field_settings(),
        Some(&StarFieldSettings::default())
    );
    history.redo();
    assert_eq!(
        history.project().layers()[0].star_field_settings(),
        Some(&chosen)
    );
}

#[test]
fn consecutive_settings_changes_to_one_layer_undo_as_a_single_step_but_not_with_opacity() {
    let mut history = fresh();
    let layer = history.add_layer(LayerKind::StarField);
    let with_density = |density| StarFieldSettings {
        density,
        ..StarFieldSettings::default()
    };

    history.set_opacity(layer, 0.4).unwrap();
    history
        .set_star_field_settings(layer, with_density(0.6))
        .unwrap();
    history
        .set_star_field_settings(layer, with_density(0.7))
        .unwrap();
    history
        .set_star_field_settings(layer, with_density(0.8))
        .unwrap();

    history.undo();
    assert_eq!(
        history.project().layers()[0]
            .star_field_settings()
            .unwrap()
            .density,
        0.5
    );
    assert_eq!(history.project().layers()[0].opacity(), 0.4);
    history.undo();
    assert_eq!(history.project().layers()[0].opacity(), 1.0);
}

#[test]
fn each_seed_change_is_its_own_undo_step() {
    let mut history = fresh();

    history.set_seed(Seed(7));
    history.set_seed(Seed(8));
    assert_eq!(history.project().seed(), Seed(8));

    history.undo();
    assert_eq!(history.project().seed(), Seed(7));
    history.undo();
    assert_eq!(history.project().seed(), Seed(5));
    history.redo();
    assert_eq!(history.project().seed(), Seed(7));
}

#[test]
fn nebula_settings_changes_undo_and_redo_and_a_drag_is_one_step() {
    let mut history = fresh();
    let layer = history.add_layer(LayerKind::Nebula);
    let with_coverage = |coverage| NebulaSettings {
        coverage,
        ..NebulaSettings::default()
    };

    history
        .set_nebula_settings(layer, with_coverage(0.6))
        .unwrap();
    history
        .set_nebula_settings(layer, with_coverage(0.7))
        .unwrap();
    history
        .set_nebula_settings(layer, with_coverage(0.8))
        .unwrap();
    assert_eq!(
        history.project().layers()[0]
            .nebula_settings()
            .unwrap()
            .coverage,
        0.8
    );

    history.undo();
    assert_eq!(
        history.project().layers()[0]
            .nebula_settings()
            .unwrap()
            .coverage,
        0.5
    );
    history.redo();
    assert_eq!(
        history.project().layers()[0]
            .nebula_settings()
            .unwrap()
            .coverage,
        0.8
    );
}

#[test]
fn history_keeps_only_the_last_100_undo_steps() {
    let mut history = fresh();
    for _ in 0..101 {
        history.add_layer(LayerKind::StarField);
    }

    let mut undone = 0;
    while history.can_undo() {
        history.undo();
        undone += 1;
    }

    assert_eq!(undone, 100);
    assert_eq!(history.project().layers().len(), 1);
}

#[test]
fn consecutive_opacity_changes_to_one_layer_undo_as_a_single_step() {
    let mut history = fresh();
    let layer = history.add_layer(LayerKind::StarField);

    history.set_opacity(layer, 0.8).unwrap();
    history.set_opacity(layer, 0.6).unwrap();
    history.set_opacity(layer, 0.4).unwrap();
    assert_eq!(history.project().layers()[0].opacity(), 0.4);

    history.undo();
    assert_eq!(history.project().layers()[0].opacity(), 1.0);
    history.undo();
    assert!(history.project().layers().is_empty());
}

#[test]
fn opacity_changes_do_not_merge_across_layers_or_across_other_edits_or_an_undo() {
    let mut history = fresh();
    let first = history.add_layer(LayerKind::StarField);
    let second = history.add_layer(LayerKind::Nebula);

    // Different Layers: two separate steps.
    history.set_opacity(first, 0.5).unwrap();
    history.set_opacity(second, 0.5).unwrap();
    history.undo();
    assert_eq!(history.project().layers()[0].opacity(), 0.5);
    assert_eq!(history.project().layers()[1].opacity(), 1.0);

    // Same Layer after an undo: a new step, not a merge into the earlier one.
    history.set_opacity(second, 0.3).unwrap();
    history.undo();
    assert_eq!(history.project().layers()[1].opacity(), 1.0);
    assert_eq!(history.project().layers()[0].opacity(), 0.5);

    // Same Layer with another edit in between: two separate steps.
    history.set_opacity(first, 0.2).unwrap();
    history.add_layer(LayerKind::Sun);
    history.set_opacity(first, 0.1).unwrap();
    history.undo();
    assert_eq!(history.project().layers().len(), 3);
    assert_eq!(history.project().layers()[0].opacity(), 0.2);
}
