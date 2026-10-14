use std::path::PathBuf;

use spacegen_core::{
    EngineConvention, HdriFormat, ImageFormat, LayerKind, PngBitDepth, Project, Seed,
};
use spacegen_render::{ExportOptions, Renderer, export_skybox, linear_to_srgb, panorama_width};

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("spacegen-test-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn star_project() -> Project {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project
}

fn faces_only(face_size: u32) -> ExportOptions {
    ExportOptions {
        name: "sky".to_string(),
        face_size,
        format: ImageFormat::Png,
        convention: EngineConvention::Unity,
        faces: true,
        panorama: false,
        hdri: false,
        hdri_format: HdriFormat::OpenExr,
        png_bit_depth: PngBitDepth::Sixteen,
        jpeg_quality: 95,
    }
}

#[test]
fn jpeg_export_writes_jpg_files_that_decode_at_the_face_size() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("jpeg");
    let options = ExportOptions {
        format: ImageFormat::Jpeg,
        convention: EngineConvention::Godot,
        ..faces_only(16)
    };

    let written = export_skybox(&renderer, &star_project(), &dir, &options, |_, _| {}).unwrap();

    assert_eq!(written.len(), 6);
    assert!(written.iter().all(|p| p.extension().unwrap() == "jpg"));
    assert!(
        written
            .iter()
            .any(|p| p.file_name().unwrap() == "sky_px.jpg")
    );
    for path in &written {
        let image = image::open(path).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
    }
}

#[test]
fn the_panorama_is_one_equirectangular_image_four_faces_wide() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("panorama");
    let options = ExportOptions {
        faces: false,
        panorama: true,
        ..faces_only(16)
    };

    let written = export_skybox(&renderer, &star_project(), &dir, &options, |_, _| {}).unwrap();

    assert_eq!(written.len(), 1);
    assert_eq!(written[0].file_name().unwrap(), "sky_panorama.png");
    let image = image::open(&written[0]).unwrap();
    assert_eq!((image.width(), image.height()), (64, 32));
}

#[test]
fn the_hdri_is_a_floating_point_exr_that_keeps_light_brighter_than_white() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("hdri");
    let mut bright = star_project();
    let layer = bright.layers()[0].id();
    bright
        .set_star_field_settings(
            layer,
            spacegen_core::StarFieldSettings {
                brightness: 1.0,
                density: 1.0,
                size: 1.0,
                ..spacegen_core::StarFieldSettings::default()
            },
        )
        .unwrap();
    let options = ExportOptions {
        faces: false,
        hdri: true,
        ..faces_only(32)
    };

    let written = export_skybox(&renderer, &bright, &dir, &options, |_, _| {}).unwrap();

    assert_eq!(written.len(), 1);
    assert_eq!(written[0].file_name().unwrap(), "sky.exr");
    let image = image::open(&written[0]).unwrap().into_rgba32f();
    assert_eq!((image.width(), image.height()), (128, 64));
    let brightest = image
        .pixels()
        .map(|p| p[0].max(p[1]).max(p[2]))
        .fold(0.0, f32::max);
    assert!(
        brightest > 1.0,
        "expected values above 1.0, brightest was {brightest}"
    );
}

#[test]
fn progress_counts_up_to_the_total_number_of_files() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("progress");
    let options = ExportOptions {
        panorama: true,
        hdri: true,
        ..faces_only(8)
    };
    let mut reports = Vec::new();

    let written = export_skybox(&renderer, &star_project(), &dir, &options, |done, total| {
        reports.push((done, total))
    })
    .unwrap();

    assert_eq!(written.len(), 8);
    assert!(reports.iter().all(|&(_, total)| total == 8));
    assert_eq!(reports.first(), Some(&(0, 8)));
    assert_eq!(reports.last(), Some(&(8, 8)));
    assert!(reports.windows(2).all(|w| w[0].0 <= w[1].0));
}

#[test]
fn exporting_faces_writes_six_square_images_named_by_the_engine_convention() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("faces");

    let written =
        export_skybox(&renderer, &star_project(), &dir, &faces_only(16), |_, _| {}).unwrap();

    let mut names: Vec<String> = written
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "sky_Back.png",
            "sky_Down.png",
            "sky_Front.png",
            "sky_Left.png",
            "sky_Right.png",
            "sky_Up.png"
        ]
    );
    for path in &written {
        let image = image::open(path).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
    }
}

#[test]
fn the_panorama_is_four_faces_wide_but_never_wider_than_8192() {
    assert_eq!(panorama_width(512), 2048);
    assert_eq!(panorama_width(2048), 8192);
    assert_eq!(panorama_width(4096), 8192);
    assert_eq!(panorama_width(8192), 8192);
}

#[test]
fn linear_light_is_encoded_with_the_srgb_curve() {
    // Reference values from the sRGB transfer function (IEC 61966-2-1).
    assert_eq!(linear_to_srgb(0.0), 0.0);
    assert_eq!(linear_to_srgb(1.0), 1.0);
    assert!((linear_to_srgb(0.5) - 0.735357).abs() < 1e-5);
    assert!((linear_to_srgb(0.001) - 0.012920).abs() < 1e-5);
}

#[test]
fn encoding_clamps_values_outside_zero_to_one() {
    assert_eq!(linear_to_srgb(-0.5), 0.0);
    assert_eq!(linear_to_srgb(4.0), 1.0);
}

#[test]
fn the_hdri_can_be_written_as_radiance_hdr_and_keeps_light_brighter_than_white() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("hdri-radiance");
    let mut bright = star_project();
    let layer = bright.layers()[0].id();
    bright
        .set_star_field_settings(
            layer,
            spacegen_core::StarFieldSettings {
                brightness: 1.0,
                density: 1.0,
                size: 1.0,
                ..spacegen_core::StarFieldSettings::default()
            },
        )
        .unwrap();
    let options = ExportOptions {
        faces: false,
        hdri: true,
        hdri_format: HdriFormat::Radiance,
        ..faces_only(32)
    };

    let written = export_skybox(&renderer, &bright, &dir, &options, |_, _| {}).unwrap();

    assert_eq!(written.len(), 1);
    assert_eq!(written[0].file_name().unwrap(), "sky.hdr");
    let image = image::open(&written[0]).unwrap().into_rgb32f();
    assert_eq!((image.width(), image.height()), (128, 64));
    let brightest = image
        .pixels()
        .map(|p| p[0].max(p[1]).max(p[2]))
        .fold(0.0, f32::max);
    assert!(
        brightest > 1.0,
        "expected values above 1.0, brightest was {brightest}"
    );
}

#[test]
fn png_faces_and_panorama_are_written_at_the_chosen_bit_depth() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    for (depth, expected) in [
        (PngBitDepth::Sixteen, image::ColorType::Rgb16),
        (PngBitDepth::Eight, image::ColorType::Rgb8),
    ] {
        let dir = temp_dir(&format!("png-depth-{depth:?}"));
        let options = ExportOptions {
            panorama: true,
            png_bit_depth: depth,
            ..faces_only(8)
        };

        let written = export_skybox(&renderer, &star_project(), &dir, &options, |_, _| {}).unwrap();

        assert_eq!(written.len(), 7);
        for path in &written {
            let color = image::open(path).unwrap().color();
            assert_eq!(color, expected, "{} at {depth:?}", path.display());
        }
    }
}
