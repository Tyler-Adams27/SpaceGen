use spacegen_core::{Face, LayerKind, NebulaSettings, Project, Seed};
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

fn lit_pixels(image: &FaceImage) -> usize {
    image
        .pixels()
        .iter()
        .filter(|p| p[0] > 0.0 || p[1] > 0.0 || p[2] > 0.0)
        .count()
}

fn nebula_project(seed: u64, change: impl FnOnce(&mut NebulaSettings)) -> Project {
    let mut project = Project::new(Seed(seed));
    let layer = project.add_layer(LayerKind::Nebula);
    let mut settings = NebulaSettings::default();
    change(&mut settings);
    project.set_nebula_settings(layer, settings).unwrap();
    project
}

#[test]
fn a_default_nebula_puts_clouds_in_the_sky() {
    let Some(renderer) = renderer() else { return };

    let image = render(&renderer, &nebula_project(5, |_| {}));

    assert!(lit_pixels(&image) > 0, "no nebula light was rendered");
}

#[test]
fn the_same_project_renders_the_same_nebula_twice() {
    let Some(renderer) = renderer() else { return };
    let project = nebula_project(5, |_| {});

    assert_eq!(render(&renderer, &project), render(&renderer, &project));
}

#[test]
fn a_different_seed_renders_a_different_nebula() {
    let Some(renderer) = renderer() else { return };

    let first = render(&renderer, &nebula_project(5, |_| {}));
    let second = render(&renderer, &nebula_project(6, |_| {}));

    assert_ne!(first, second);
}

#[test]
fn zero_coverage_or_zero_brightness_renders_black() {
    let Some(renderer) = renderer() else { return };

    let no_coverage = nebula_project(5, |s| s.coverage = 0.0);
    let no_brightness = nebula_project(5, |s| s.brightness = 0.0);

    assert_eq!(lit_pixels(&render(&renderer, &no_coverage)), 0);
    assert_eq!(lit_pixels(&render(&renderer, &no_brightness)), 0);
}

#[test]
fn more_coverage_lights_more_of_the_sky() {
    let Some(renderer) = renderer() else { return };

    let wispy = nebula_project(5, |s| s.coverage = 0.3);
    let thick = nebula_project(5, |s| s.coverage = 0.9);

    assert!(lit_pixels(&render(&renderer, &thick)) > lit_pixels(&render(&renderer, &wispy)));
}

#[test]
fn hidden_and_fully_transparent_nebulae_render_black() {
    let Some(renderer) = renderer() else { return };

    let mut hidden = nebula_project(5, |_| {});
    hidden.set_visible(hidden.layers()[0].id(), false).unwrap();
    let mut transparent = nebula_project(5, |_| {});
    transparent
        .set_opacity(transparent.layers()[0].id(), 0.0)
        .unwrap();

    assert_eq!(lit_pixels(&render(&renderer, &hidden)), 0);
    assert_eq!(lit_pixels(&render(&renderer, &transparent)), 0);
}

#[test]
fn a_nebula_painted_in_pure_red_gives_no_green_or_blue_light() {
    let Some(renderer) = renderer() else { return };
    let red = nebula_project(5, |s| {
        s.color_a = [1.0, 0.0, 0.0];
        s.color_b = [1.0, 0.0, 0.0];
    });

    let image = render(&renderer, &red);

    assert!(image.pixels().iter().any(|p| p[0] > 0.0));
    assert!(image.pixels().iter().all(|p| p[1] == 0.0 && p[2] == 0.0));
}

#[test]
fn the_two_colors_both_show_up_in_the_clouds() {
    let Some(renderer) = renderer() else { return };
    let two_tone = nebula_project(5, |s| {
        s.coverage = 1.0;
        s.color_a = [1.0, 0.0, 0.0];
        s.color_b = [0.0, 0.0, 1.0];
    });

    let image = render(&renderer, &two_tone);

    assert!(
        image
            .pixels()
            .iter()
            .any(|p| p[0] > 0.05 && p[2] < p[0] * 0.5),
        "no color A clouds"
    );
    assert!(
        image
            .pixels()
            .iter()
            .any(|p| p[2] > 0.05 && p[0] < p[2] * 0.5),
        "no color B clouds"
    );
}

#[test]
fn a_nebula_and_a_star_field_add_their_light_together() {
    let Some(renderer) = renderer() else { return };
    let nebula_only = nebula_project(5, |_| {});
    let mut both = nebula_project(5, |_| {});
    both.add_layer(LayerKind::StarField);

    let nebula = render(&renderer, &nebula_only);
    let combined = render(&renderer, &both);

    assert_ne!(nebula, combined);
    assert!(
        nebula
            .pixels()
            .iter()
            .zip(combined.pixels())
            .all(|(a, b)| (0..3).all(|c| b[c] >= a[c]))
    );
}
