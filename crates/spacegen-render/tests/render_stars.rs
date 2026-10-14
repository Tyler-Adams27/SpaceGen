use spacegen_core::{Face, LayerKind, Project, Seed, StarFieldSettings};
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

fn star_field_project(seed: u64) -> Project {
    let mut project = Project::new(Seed(seed));
    project.add_layer(LayerKind::StarField);
    project
}

fn with_settings(seed: u64, change: impl FnOnce(&mut StarFieldSettings)) -> Project {
    let mut project = Project::new(Seed(seed));
    let layer = project.add_layer(LayerKind::StarField);
    let mut settings = StarFieldSettings::default();
    change(&mut settings);
    project.set_star_field_settings(layer, settings).unwrap();
    project
}

#[test]
fn a_default_star_field_puts_stars_in_the_sky() {
    let Some(renderer) = renderer() else { return };

    let image = render(&renderer, &star_field_project(5));

    assert!(lit_pixels(&image) > 0, "no star was rendered");
}

#[test]
fn the_same_project_renders_the_same_image_twice() {
    let Some(renderer) = renderer() else { return };
    let project = star_field_project(5);

    assert_eq!(render(&renderer, &project), render(&renderer, &project));
}

#[test]
fn a_different_seed_renders_a_different_image() {
    let Some(renderer) = renderer() else { return };

    let first = render(&renderer, &star_field_project(5));
    let second = render(&renderer, &star_field_project(6));

    assert_ne!(first, second);
}

#[test]
fn zero_density_or_zero_brightness_renders_black() {
    let Some(renderer) = renderer() else { return };

    let no_density = with_settings(5, |s| s.density = 0.0);
    let no_brightness = with_settings(5, |s| s.brightness = 0.0);

    assert_eq!(lit_pixels(&render(&renderer, &no_density)), 0);
    assert_eq!(lit_pixels(&render(&renderer, &no_brightness)), 0);
}

#[test]
fn higher_density_lights_more_pixels() {
    let Some(renderer) = renderer() else { return };

    let sparse = with_settings(5, |s| s.density = 0.3);
    let dense = with_settings(5, |s| s.density = 0.9);

    assert!(lit_pixels(&render(&renderer, &dense)) > lit_pixels(&render(&renderer, &sparse)));
}

#[test]
fn hidden_and_fully_transparent_layers_render_black() {
    let Some(renderer) = renderer() else { return };

    let mut hidden = star_field_project(5);
    hidden.set_visible(hidden.layers()[0].id(), false).unwrap();
    let mut transparent = star_field_project(5);
    transparent
        .set_opacity(transparent.layers()[0].id(), 0.0)
        .unwrap();

    assert_eq!(lit_pixels(&render(&renderer, &hidden)), 0);
    assert_eq!(lit_pixels(&render(&renderer, &transparent)), 0);
}

#[test]
fn a_pure_red_tint_gives_no_green_or_blue_light() {
    let Some(renderer) = renderer() else { return };
    let red = with_settings(5, |s| s.tint = [1.0, 0.0, 0.0]);

    let image = render(&renderer, &red);

    assert!(image.pixels().iter().any(|p| p[0] > 0.0));
    assert!(image.pixels().iter().all(|p| p[1] == 0.0 && p[2] == 0.0));
}

#[test]
fn the_equirectangular_panorama_is_twice_as_wide_as_tall_and_holds_stars() {
    let Some(renderer) = renderer() else { return };

    let image = renderer
        .render_equirectangular(&star_field_project(5), 128)
        .unwrap();

    assert_eq!((image.width(), image.height()), (128, 64));
    assert_eq!(image.pixels().len(), 128 * 64);
    assert!(lit_pixels(&image) > 0);
}

fn largest_difference(a: &FaceImage, b: &FaceImage) -> f32 {
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    a.pixels()
        .iter()
        .zip(b.pixels())
        .flat_map(|(p, q)| (0..4).map(move |c| (p[c] - q[c]).abs()))
        .fold(0.0, f32::max)
}

#[test]
fn rendering_in_bands_gives_the_same_image_as_rendering_in_one_pass() {
    let Some(whole_renderer) = renderer() else {
        return;
    };
    let project = star_field_project(5);
    let whole = whole_renderer
        .render_face(&project, Face::PositiveZ, 41)
        .unwrap();
    let banded_renderer = Renderer::software().unwrap().with_max_band_rows(7);

    let banded = banded_renderer
        .render_face(&project, Face::PositiveZ, 41)
        .unwrap();

    assert!(lit_pixels(&whole) > 0);
    assert!(largest_difference(&whole, &banded) < 1e-3);
}

#[test]
fn bands_also_cover_the_panorama() {
    let Some(whole_renderer) = renderer() else {
        return;
    };
    let project = star_field_project(5);
    let whole = whole_renderer.render_equirectangular(&project, 90).unwrap();
    let banded_renderer = Renderer::software().unwrap().with_max_band_rows(4);

    let banded = banded_renderer
        .render_equirectangular(&project, 90)
        .unwrap();

    assert!(largest_difference(&whole, &banded) < 1e-3);
}

#[test]
fn every_pixel_is_opaque() {
    let Some(renderer) = renderer() else { return };

    let image = render(&renderer, &star_field_project(5));

    assert!(image.pixels().iter().all(|p| p[3] == 1.0));
}

#[test]
fn adding_a_layer_never_changes_how_an_existing_layer_looks() {
    let Some(renderer) = renderer() else { return };
    let alone = star_field_project(5);
    let mut with_hidden_sibling = star_field_project(5);
    let sibling = with_hidden_sibling.add_layer(LayerKind::StarField);
    with_hidden_sibling.set_visible(sibling, false).unwrap();
    let mut with_visible_sibling = star_field_project(5);
    with_visible_sibling.add_layer(LayerKind::StarField);

    let base = render(&renderer, &alone);
    let hidden = render(&renderer, &with_hidden_sibling);
    let combined = render(&renderer, &with_visible_sibling);

    assert_eq!(base, hidden);
    // The sibling has its own stars, in places where the first layer drew none.
    assert!(
        base.pixels()
            .iter()
            .zip(combined.pixels())
            .any(|(a, b)| a[..3] == [0.0; 3] && b[..3] != [0.0; 3])
    );
    // Additive light only ever adds to what the first layer already drew.
    assert!(
        base.pixels()
            .iter()
            .zip(combined.pixels())
            .all(|(a, b)| (0..3).all(|c| b[c] >= a[c]))
    );
}
