use spacegen_core::{History, LayerKind, Project, Seed};

fn full_project() -> Project {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::GalaxyBand);
    project.add_layer(LayerKind::Sun);
    project
}

/// Every color in the project, in a fixed order.
fn colors(project: &Project) -> Vec<[f32; 3]> {
    let layers = project.layers();
    let nebula = layers[1].nebula_settings().unwrap();
    let band = layers[2].galaxy_band_settings().unwrap();
    vec![
        layers[0].star_field_settings().unwrap().tint,
        nebula.color_a,
        nebula.color_b,
        band.color_a,
        band.color_b,
        layers[3].sun_settings().unwrap().color,
    ]
}

#[test]
fn randomizing_changes_every_color_of_every_layer() {
    let mut project = full_project();
    let before = colors(&project);

    project.randomize_colors(Seed(99));

    for (index, (old, new)) in before.iter().zip(colors(&project)).enumerate() {
        assert_ne!(*old, new, "color {index} was left unchanged");
    }
}

#[test]
fn the_same_seed_gives_the_same_colors_and_a_different_seed_gives_different_ones() {
    let mut first = full_project();
    let mut second = full_project();
    let mut third = full_project();

    first.randomize_colors(Seed(99));
    second.randomize_colors(Seed(99));
    third.randomize_colors(Seed(100));

    assert_eq!(colors(&first), colors(&second));
    assert_ne!(colors(&first), colors(&third));
}

#[test]
fn random_colors_are_valid_and_survive_a_save_and_load() {
    for seed in 0..50 {
        let mut project = full_project();
        project.randomize_colors(Seed(seed));

        let loaded = Project::from_json(&project.to_json()).unwrap();

        assert_eq!(loaded, project);
    }
}

#[test]
fn star_tints_stay_close_to_white_while_nebula_and_band_colors_are_vivid() {
    for seed in 0..50 {
        let mut project = full_project();
        project.randomize_colors(Seed(seed));
        let all = colors(&project);
        let spread = |c: &[f32; 3]| c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]);

        assert!(
            spread(&all[0]) <= 0.31,
            "seed {seed}: star tint {:?}",
            all[0]
        );
        assert!(
            spread(&all[5]) <= 0.31,
            "seed {seed}: sun color {:?}",
            all[5]
        );
        for vivid in &all[1..5] {
            assert!(spread(vivid) >= 0.3, "seed {seed}: dull color {vivid:?}");
        }
    }
}

#[test]
fn randomizing_leaves_everything_but_colors_alone() {
    let mut project = full_project();
    project.set_opacity(project.layers()[1].id(), 0.4).unwrap();
    let mut recolored = project.clone();

    recolored.randomize_colors(Seed(99));

    let strip = |p: &Project| {
        let mut json: serde_json::Value = serde_json::from_str(&p.to_json()).unwrap();
        for layer in json["layers"].as_array_mut().unwrap() {
            for key in ["tint", "color", "color_a", "color_b"] {
                layer["settings"].as_object_mut().unwrap().remove(key);
            }
        }
        json
    };
    assert_eq!(strip(&project), strip(&recolored));
}

#[test]
fn history_randomize_sets_the_seed_and_the_colors_as_one_undo_step() {
    let mut history = History::new(full_project());
    let before = colors(history.project());

    history.randomize(Seed(99));

    assert_eq!(history.project().seed(), Seed(99));
    assert_ne!(colors(history.project()), before);

    history.undo();

    assert_eq!(history.project().seed(), Seed(5));
    assert_eq!(colors(history.project()), before);
    assert!(!history.can_undo());
}
