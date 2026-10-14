use spacegen_core::{Face, GalaxyBandSettings, LayerKind, Project, Seed};
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

/// Pixels bright enough to see, since the band's glow fades toward but never reaches zero.
fn visible_pixels(image: &FaceImage) -> usize {
    image
        .pixels()
        .iter()
        .filter(|p| p[0].max(p[1]).max(p[2]) > 0.02)
        .count()
}

fn total_light(image: &FaceImage) -> f32 {
    image.pixels().iter().map(|p| p[0] + p[1] + p[2]).sum()
}

fn band_project(seed: u64, change: impl FnOnce(&mut GalaxyBandSettings)) -> Project {
    let mut project = Project::new(Seed(seed));
    let layer = project.add_layer(LayerKind::GalaxyBand);
    let mut settings = GalaxyBandSettings::default();
    change(&mut settings);
    project.set_galaxy_band_settings(layer, settings).unwrap();
    project
}

#[test]
fn a_default_galaxy_band_puts_light_in_the_sky() {
    let Some(renderer) = renderer() else { return };

    let image = render(&renderer, &band_project(5, |_| {}));

    assert!(lit_pixels(&image) > 0, "no galaxy band light was rendered");
}

#[test]
fn the_same_project_renders_the_same_galaxy_band_twice() {
    let Some(renderer) = renderer() else { return };
    let project = band_project(5, |_| {});

    assert_eq!(render(&renderer, &project), render(&renderer, &project));
}

#[test]
fn a_different_seed_renders_a_different_galaxy_band() {
    let Some(renderer) = renderer() else { return };

    let first = render(&renderer, &band_project(5, |_| {}));
    let second = render(&renderer, &band_project(6, |_| {}));

    assert_ne!(first, second);
}

#[test]
fn zero_brightness_renders_black() {
    let Some(renderer) = renderer() else { return };
    let dark = band_project(5, |s| s.brightness = 0.0);

    assert_eq!(lit_pixels(&render(&renderer, &dark)), 0);
}

#[test]
fn hidden_and_fully_transparent_galaxy_bands_render_black() {
    let Some(renderer) = renderer() else { return };
    let mut hidden = band_project(5, |_| {});
    hidden.set_visible(hidden.layers()[0].id(), false).unwrap();
    let mut transparent = band_project(5, |_| {});
    transparent
        .set_opacity(transparent.layers()[0].id(), 0.0)
        .unwrap();

    assert_eq!(lit_pixels(&render(&renderer, &hidden)), 0);
    assert_eq!(lit_pixels(&render(&renderer, &transparent)), 0);
}

#[test]
fn a_wider_band_lights_more_of_the_sky() {
    let Some(renderer) = renderer() else { return };

    let narrow = band_project(5, |s| s.width = 0.1);
    let wide = band_project(5, |s| s.width = 0.9);

    assert!(
        visible_pixels(&render(&renderer, &wide)) > visible_pixels(&render(&renderer, &narrow))
    );
}

#[test]
fn tilting_the_band_moves_it_across_the_sky() {
    let Some(renderer) = renderer() else { return };

    let low = render(&renderer, &band_project(5, |s| s.tilt = 0.0));
    let high = render(&renderer, &band_project(5, |s| s.tilt = 0.5));

    assert_ne!(low, high);
}

#[test]
fn stronger_dust_lanes_take_light_out_of_the_band() {
    let Some(renderer) = renderer() else { return };

    let clear = total_light(&render(&renderer, &band_project(5, |s| s.dust = 0.0)));
    let dusty = total_light(&render(&renderer, &band_project(5, |s| s.dust = 1.0)));

    assert!(
        dusty < clear,
        "dust {dusty} was not darker than clear {clear}"
    );
}

#[test]
fn a_band_painted_in_pure_red_gives_no_green_or_blue_light() {
    let Some(renderer) = renderer() else { return };
    let red = band_project(5, |s| {
        s.color_a = [1.0, 0.0, 0.0];
        s.color_b = [1.0, 0.0, 0.0];
    });

    let image = render(&renderer, &red);

    assert!(image.pixels().iter().any(|p| p[0] > 0.0));
    assert!(image.pixels().iter().all(|p| p[1] == 0.0 && p[2] == 0.0));
}

#[test]
fn the_core_color_and_the_outer_glow_color_both_show_up() {
    let Some(renderer) = renderer() else { return };
    let two_tone = band_project(5, |s| {
        s.width = 1.0;
        s.dust = 0.0;
        s.color_a = [1.0, 0.0, 0.0];
        s.color_b = [0.0, 0.0, 1.0];
    });

    let image = render(&renderer, &two_tone);

    assert!(
        image
            .pixels()
            .iter()
            .any(|p| p[0] > 0.05 && p[2] < p[0] * 0.5),
        "no core color"
    );
    assert!(
        image
            .pixels()
            .iter()
            .any(|p| p[2] > 0.02 && p[0] < p[2] * 0.5),
        "no outer glow color"
    );
}

#[test]
fn a_galaxy_band_and_a_star_field_add_their_light_together() {
    let Some(renderer) = renderer() else { return };
    let band_only = band_project(5, |_| {});
    let mut both = band_project(5, |_| {});
    both.add_layer(LayerKind::StarField);

    let band = render(&renderer, &band_only);
    let combined = render(&renderer, &both);

    assert_ne!(band, combined);
    assert!(
        band.pixels()
            .iter()
            .zip(combined.pixels())
            .all(|(a, b)| (0..3).all(|c| b[c] >= a[c]))
    );
}
