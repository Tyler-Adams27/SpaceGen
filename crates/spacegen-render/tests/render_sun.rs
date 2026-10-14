use spacegen_core::{Face, LayerKind, Project, Seed, SunSettings};
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

/// Pixels bright enough to see, since a glow fades toward but never reaches zero.
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

fn brightest_channel(image: &FaceImage) -> f32 {
    image
        .pixels()
        .iter()
        .map(|p| p[0].max(p[1]).max(p[2]))
        .fold(0.0, f32::max)
}

/// (column, row) of the brightest pixel.
fn brightest_position(image: &FaceImage) -> (u32, u32) {
    let (index, _) = image
        .pixels()
        .iter()
        .map(|p| p[0] + p[1] + p[2])
        .enumerate()
        .fold(
            (0, -1.0),
            |best, (i, v)| if v > best.1 { (i, v) } else { best },
        );
    (index as u32 % SIZE, index as u32 / SIZE)
}

fn sun_project(seed: u64, change: impl FnOnce(&mut SunSettings)) -> Project {
    let mut project = Project::new(Seed(seed));
    let layer = project.add_layer(LayerKind::Sun);
    let mut settings = SunSettings::default();
    change(&mut settings);
    project.set_sun_settings(layer, settings).unwrap();
    project
}

#[test]
fn a_default_sun_shines_in_the_sky_ahead() {
    let Some(renderer) = renderer() else { return };

    let image = render(&renderer, &sun_project(5, |_| {}));

    assert!(visible_pixels(&image) > 0, "no sun light was rendered");
}

#[test]
fn the_sun_is_brighter_than_white_so_the_hdri_keeps_its_glare() {
    let Some(renderer) = renderer() else { return };

    let image = render(&renderer, &sun_project(5, |_| {}));

    assert!(
        brightest_channel(&image) > 1.0,
        "brightest was {}",
        brightest_channel(&image)
    );
}

#[test]
fn the_same_project_renders_the_same_sun_twice() {
    let Some(renderer) = renderer() else { return };
    let project = sun_project(5, |_| {});

    assert_eq!(render(&renderer, &project), render(&renderer, &project));
}

#[test]
fn the_seed_does_not_move_or_change_a_hand_placed_sun() {
    let Some(renderer) = renderer() else { return };

    let first = render(&renderer, &sun_project(5, |_| {}));
    let second = render(&renderer, &sun_project(6, |_| {}));

    assert_eq!(first, second);
}

#[test]
fn zero_brightness_renders_black() {
    let Some(renderer) = renderer() else { return };
    let dark = sun_project(5, |s| s.brightness = 0.0);

    assert_eq!(lit_pixels(&render(&renderer, &dark)), 0);
}

#[test]
fn hidden_and_fully_transparent_suns_render_black() {
    let Some(renderer) = renderer() else { return };
    let mut hidden = sun_project(5, |_| {});
    hidden.set_visible(hidden.layers()[0].id(), false).unwrap();
    let mut transparent = sun_project(5, |_| {});
    transparent
        .set_opacity(transparent.layers()[0].id(), 0.0)
        .unwrap();

    assert_eq!(lit_pixels(&render(&renderer, &hidden)), 0);
    assert_eq!(lit_pixels(&render(&renderer, &transparent)), 0);
}

#[test]
fn a_sun_behind_the_viewer_is_not_seen_in_the_front_face() {
    let Some(renderer) = renderer() else { return };
    let behind = sun_project(5, |s| s.azimuth = 0.5);

    assert_eq!(visible_pixels(&render(&renderer, &behind)), 0);
}

#[test]
fn azimuth_turns_the_sun_to_the_right_and_elevation_raises_it() {
    let Some(renderer) = renderer() else { return };
    let ahead = brightest_position(&render(&renderer, &sun_project(5, |_| {})));
    let right = brightest_position(&render(&renderer, &sun_project(5, |s| s.azimuth = 0.05)));
    let higher = brightest_position(&render(&renderer, &sun_project(5, |s| s.elevation = 0.7)));

    assert!(
        right.0 > ahead.0,
        "right {right:?} was not right of {ahead:?}"
    );
    assert!(
        higher.1 < ahead.1,
        "higher {higher:?} was not above {ahead:?}"
    );
}

#[test]
fn a_bigger_sun_covers_more_of_the_sky() {
    let Some(renderer) = renderer() else { return };

    let small = sun_project(5, |s| {
        s.size = 0.2;
        s.glare = 0.0;
    });
    let big = sun_project(5, |s| {
        s.size = 1.0;
        s.glare = 0.0;
    });

    assert!(visible_pixels(&render(&renderer, &big)) > visible_pixels(&render(&renderer, &small)));
}

#[test]
fn more_glare_adds_light_around_the_sun_and_no_glare_still_leaves_the_disc() {
    let Some(renderer) = renderer() else { return };

    let none = render(&renderer, &sun_project(5, |s| s.glare = 0.0));
    let full = render(&renderer, &sun_project(5, |s| s.glare = 1.0));

    assert!(visible_pixels(&none) > 0, "the disc disappeared");
    assert!(total_light(&full) > total_light(&none));
}

#[test]
fn a_sun_painted_in_pure_red_gives_no_green_or_blue_light() {
    let Some(renderer) = renderer() else { return };
    let red = sun_project(5, |s| s.color = [1.0, 0.0, 0.0]);

    let image = render(&renderer, &red);

    assert!(image.pixels().iter().any(|p| p[0] > 0.0));
    assert!(image.pixels().iter().all(|p| p[1] == 0.0 && p[2] == 0.0));
}

#[test]
fn a_sun_and_a_star_field_add_their_light_together() {
    let Some(renderer) = renderer() else { return };
    let sun_only = sun_project(5, |_| {});
    let mut both = sun_project(5, |_| {});
    both.add_layer(LayerKind::StarField);

    let sun = render(&renderer, &sun_only);
    let combined = render(&renderer, &both);

    assert_ne!(sun, combined);
    assert!(
        sun.pixels()
            .iter()
            .zip(combined.pixels())
            .all(|(a, b)| (0..3).all(|c| b[c] >= a[c]))
    );
}
