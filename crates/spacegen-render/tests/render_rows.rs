use spacegen_core::{Face, LayerKind, Project, Seed};
use spacegen_render::{RenderError, Renderer};

#[derive(Debug)]
enum TestError {
    Render,
    Sink,
}

impl From<RenderError> for TestError {
    fn from(_: RenderError) -> Self {
        TestError::Render
    }
}

fn project() -> Project {
    let mut project = Project::new(Seed(9));
    project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::StarField);
    project
}

#[test]
fn face_rows_arrive_in_bands_in_order_and_add_up_to_the_whole_face() {
    let Ok(renderer) = Renderer::software().map(|r| r.with_max_band_rows(3)) else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let whole = renderer
        .render_face(&project(), Face::PositiveZ, 16)
        .unwrap();
    let mut bands = Vec::new();
    let mut pixels: Vec<[f32; 4]> = Vec::new();

    renderer
        .render_face_rows(&project(), Face::PositiveZ, 16, |first_row, band| {
            bands.push((first_row, band.len() / 16));
            pixels.extend_from_slice(band);
            Ok::<(), TestError>(())
        })
        .unwrap();

    assert_eq!(bands, [(0, 3), (3, 3), (6, 3), (9, 3), (12, 3), (15, 1)]);
    assert_eq!(pixels, whole.pixels());
}

#[test]
fn an_error_from_the_sink_stops_the_render_and_comes_back_unchanged() {
    let Ok(renderer) = Renderer::software().map(|r| r.with_max_band_rows(4)) else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let mut calls = 0;

    let result = renderer.render_face_rows(&project(), Face::PositiveX, 16, |_, _| {
        calls += 1;
        Err(TestError::Sink)
    });

    assert!(matches!(result, Err(TestError::Sink)), "{result:?}");
    assert_eq!(calls, 1);
}

#[test]
fn a_panorama_rendered_in_column_tiles_matches_a_single_pass() {
    let (Ok(tiled), Ok(single)) = (
        Renderer::software().map(|r| r.with_max_tile_width(11).with_max_band_rows(5)),
        Renderer::software(),
    ) else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };

    // 64 wide in tiles of 11 is five full tiles and a narrow one of 9: the edges fall mid-sky.
    let a = tiled.render_equirectangular(&project(), 64).unwrap();
    let b = single.render_equirectangular(&project(), 64).unwrap();

    assert_eq!((a.width(), a.height()), (64, 32));
    // Tile and single-pass renders may differ in the last bit on some GPUs (ADR 0001), so compare
    // by mean difference; a tile drawn from the wrong part of the sky or shifted by even half a
    // pixel is far above this.
    let mean: f32 = a
        .pixels()
        .iter()
        .zip(b.pixels())
        .map(|(p, q)| (0..3).map(|c| (p[c] - q[c]).abs()).sum::<f32>() / 3.0)
        .sum::<f32>()
        / a.pixels().len() as f32;
    assert!(
        mean < 0.002,
        "tiled and single-pass differ by {mean} on average"
    );
}
