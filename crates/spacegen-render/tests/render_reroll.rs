use spacegen_core::{Face, LayerKind, Project, Seed};
use spacegen_render::{FaceImage, Renderer};

const SIZE: u32 = 96;

fn renderer() -> Option<Renderer> {
    match Renderer::software() {
        Ok(renderer) => Some(renderer),
        Err(error) => {
            eprintln!("SKIPPED: no software adapter available: {error}");
            None
        }
    }
}

fn render(renderer: &Renderer, project: &Project) -> FaceImage {
    renderer
        .render_face(project, Face::PositiveZ, SIZE)
        .unwrap()
}

fn all_layers() -> Project {
    let mut project = Project::new(Seed(5));
    for kind in [
        LayerKind::StarField,
        LayerKind::Nebula,
        LayerKind::GalaxyBand,
        LayerKind::Sun,
    ] {
        project.add_layer(kind);
    }
    project
}

/// A copy with only the Layers whose index passes `keep` visible.
fn showing(project: &Project, keep: impl Fn(usize) -> bool) -> Project {
    let mut copy = project.clone();
    for (index, layer) in project.layers().iter().enumerate() {
        copy.set_visible(layer.id(), keep(index)).unwrap();
    }
    copy
}

#[test]
fn rerolling_one_layer_leaves_every_other_layers_pixels_unchanged() {
    let Some(renderer) = renderer() else { return };
    let before = all_layers();

    for index in 0..before.layers().len() {
        let mut after = before.clone();
        after
            .reroll_layer(before.layers()[index].id(), Seed(99))
            .unwrap();

        // With the re-rolled Layer hidden, the picture is what the others make.
        let others_before = render(&renderer, &showing(&before, |i| i != index));
        let others_after = render(&renderer, &showing(&after, |i| i != index));

        assert_eq!(
            others_before, others_after,
            "re-rolling layer {index} changed another layer"
        );
    }
}

#[test]
fn rerolling_a_layer_changes_how_that_layer_looks() {
    let Some(renderer) = renderer() else { return };
    let before = all_layers();

    for index in 0..before.layers().len() {
        let mut after = before.clone();
        after
            .reroll_layer(before.layers()[index].id(), Seed(99))
            .unwrap();

        let alone_before = render(&renderer, &showing(&before, |i| i == index));
        let alone_after = render(&renderer, &showing(&after, |i| i == index));

        assert_ne!(alone_before, alone_after, "layer {index} looks the same");
    }
}

#[test]
fn rerolling_changes_a_layers_structure_and_not_only_its_colors() {
    let Some(renderer) = renderer() else { return };
    let before = all_layers();
    // The Sun is placed by hand, so only the seeded kinds have a structure to change.
    for index in 0..3 {
        let id = before.layers()[index].id();
        let mut after = before.clone();
        after.reroll_layer(id, Seed(99)).unwrap();

        // The old structure (variation 0) in the new colors.
        let mut recolored = before.clone();
        let layer = &after.layers()[index];
        match layer.kind() {
            LayerKind::StarField => recolored
                .set_star_field_settings(id, *layer.star_field_settings().unwrap())
                .unwrap(),
            LayerKind::Nebula => recolored
                .set_nebula_settings(id, *layer.nebula_settings().unwrap())
                .unwrap(),
            _ => recolored
                .set_galaxy_band_settings(id, *layer.galaxy_band_settings().unwrap())
                .unwrap(),
        }

        let new_structure = render(&renderer, &showing(&after, |i| i == index));
        let old_structure = render(&renderer, &showing(&recolored, |i| i == index));

        assert_ne!(
            new_structure, old_structure,
            "layer {index} only changed color"
        );
    }
}

#[test]
fn a_project_that_was_never_rerolled_renders_as_it_always_did() {
    let Some(renderer) = renderer() else { return };
    let project = all_layers();

    // A plain reload keeps every variation at 0, so nothing about the picture changes.
    let reloaded = Project::from_json(&project.to_json()).unwrap();

    assert_eq!(render(&renderer, &project), render(&renderer, &reloaded));
}
