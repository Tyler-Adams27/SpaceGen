use std::path::PathBuf;

use spacegen_core::{
    EngineConvention, HdriFormat, ImageFormat, LayerKind, PngBitDepth, Project, Seed,
};
use spacegen_render::{
    CancelToken, ExportError, ExportOptions, Renderer, existing_outputs, export_skybox, hdri_width,
    linear_to_srgb, panorama_width,
};

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
fn the_panorama_is_four_faces_wide_with_no_cap_but_the_hdri_stops_at_8192() {
    assert_eq!(panorama_width(512), 2048);
    assert_eq!(panorama_width(2048), 8192);
    assert_eq!(panorama_width(4096), 16384);
    assert_eq!(panorama_width(8192), 32768);
    assert_eq!(hdri_width(512), 2048);
    assert_eq!(hdri_width(2048), 8192);
    assert_eq!(hdri_width(4096), 8192);
    assert_eq!(hdri_width(8192), 8192);
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

#[test]
fn cancelling_mid_export_stops_with_cancelled_and_leaves_no_files_behind() {
    let token = CancelToken::new();
    let Ok(renderer) = Renderer::software().map(|r| r.with_cancel_token(token.clone())) else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("cancel");
    let mut calls = 0;

    let result = export_skybox(
        &renderer,
        &star_project(),
        &dir,
        &faces_only(8),
        |done, _| {
            calls += 1;
            if done == 2 {
                token.cancel();
            }
        },
    );

    assert!(matches!(result, Err(ExportError::Cancelled)), "{result:?}");
    assert!(
        calls < 7,
        "export kept going after the cancel ({calls} reports)"
    );
    let left: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
    assert!(left.is_empty(), "files left behind: {left:?}");
}

#[test]
fn cancelling_stops_between_bands_within_one_image() {
    let token = CancelToken::new();
    let renderer = match Renderer::software() {
        Ok(r) => r.with_max_band_rows(1).with_cancel_token(token.clone()),
        Err(_) => {
            eprintln!("SKIPPED: no software adapter");
            return;
        }
    };
    token.cancel();

    let error = renderer
        .render_face(&star_project(), spacegen_core::Face::PositiveZ, 8)
        .unwrap_err();

    assert!(error.is_cancelled());
}

/// What the exported file should decode to: the in-memory render, encoded the way the format
/// says (sRGB, 8 or 16 bit, JPEG through the same encoder and quality).
fn expected_pixels(image: &spacegen_render::SkyImage, options: &ExportOptions) -> Vec<u8> {
    let (width, height) = (image.width(), image.height());
    match (options.format, options.png_bit_depth) {
        (ImageFormat::Png, PngBitDepth::Sixteen) => {
            let mut buffer = image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::new(width, height);
            for (target, source) in buffer.pixels_mut().zip(image.pixels()) {
                *target = image::Rgb(std::array::from_fn(|c| {
                    (linear_to_srgb(source[c]) * 65535.0).round() as u16
                }));
            }
            image::DynamicImage::ImageRgb16(buffer).as_bytes().to_vec()
        }
        (format, _) => {
            let mut buffer = image::RgbImage::new(width, height);
            for (target, source) in buffer.pixels_mut().zip(image.pixels()) {
                *target = image::Rgb(std::array::from_fn(|c| {
                    (linear_to_srgb(source[c]) * 255.0).round() as u8
                }));
            }
            if format == ImageFormat::Jpeg {
                let mut encoded = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    &mut encoded,
                    options.jpeg_quality,
                );
                buffer.write_with_encoder(encoder).unwrap();
                return image::load_from_memory(&encoded)
                    .unwrap()
                    .as_bytes()
                    .to_vec();
            }
            buffer.into_raw()
        }
    }
}

#[test]
fn streamed_files_decode_to_exactly_the_in_memory_render_for_every_format() {
    // 3-row bands, so every image really is written in several pieces. (Banded and single-pass
    // renders may differ by a pixel on some GPUs, so each file is compared with the render made
    // by this same Renderer.)
    let Ok(renderer) =
        Renderer::software().map(|r| r.with_max_band_rows(3).with_max_tile_width(24))
    else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let project = star_project();
    for (label, format, depth) in [
        ("png16", ImageFormat::Png, PngBitDepth::Sixteen),
        ("png8", ImageFormat::Png, PngBitDepth::Eight),
        ("jpeg", ImageFormat::Jpeg, PngBitDepth::Eight),
    ] {
        let options = ExportOptions {
            format,
            png_bit_depth: depth,
            panorama: true,
            ..faces_only(16)
        };
        let dir = temp_dir(&format!("stream-{label}"));

        let files = export_skybox(&renderer, &project, &dir, &options, |_, _| {}).unwrap();

        assert_eq!(files.len(), 7);
        for file in &files {
            let name = file.file_name().unwrap().to_string_lossy().into_owned();
            let image = if name.contains("panorama") {
                renderer
                    .render_equirectangular(&project, panorama_width(16))
                    .unwrap()
            } else {
                let face = [
                    spacegen_core::Face::PositiveX,
                    spacegen_core::Face::NegativeX,
                    spacegen_core::Face::PositiveY,
                    spacegen_core::Face::NegativeY,
                    spacegen_core::Face::PositiveZ,
                    spacegen_core::Face::NegativeZ,
                ]
                .into_iter()
                .find(|f| options.convention.face_file_name("sky", *f, format) == name)
                .unwrap_or_else(|| panic!("unknown file {name}"));
                renderer.render_face(&project, face, 16).unwrap()
            };
            let decoded = image::open(file).unwrap();
            assert_eq!(
                decoded.as_bytes(),
                expected_pixels(&image, &options).as_slice(),
                "{label}: {name} differs from the in-memory render"
            );
        }
    }
}

fn file_names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_finished_export_leaves_only_the_finished_files_no_temporary_names() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("no-part-files");
    let options = ExportOptions {
        panorama: true,
        hdri: true,
        ..faces_only(8)
    };

    let written = export_skybox(&renderer, &star_project(), &dir, &options, |_, _| {}).unwrap();

    assert_eq!(written.len(), 8);
    assert_eq!(file_names(&dir).len(), 8, "{:?}", file_names(&dir));
    assert!(file_names(&dir).iter().all(|n| !n.contains(".part")));
}

#[test]
fn a_failed_export_leaves_an_existing_file_of_the_same_name_untouched() {
    let token = CancelToken::new();
    let Ok(renderer) = Renderer::software().map(|r| r.with_cancel_token(token.clone())) else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("keep-old");
    let old = dir.join("sky_panorama.png");
    std::fs::write(&old, "the previous export").unwrap();
    let options = ExportOptions {
        faces: false,
        panorama: true,
        ..faces_only(8)
    };

    // Cancelled just as the panorama starts, after its file has been opened for writing.
    let result = export_skybox(&renderer, &star_project(), &dir, &options, |_, _| {
        token.cancel()
    });

    assert!(matches!(result, Err(ExportError::Cancelled)));
    assert_eq!(
        std::fs::read_to_string(&old).unwrap(),
        "the previous export"
    );
    assert_eq!(file_names(&dir), ["sky_panorama.png"]);
}

#[test]
fn existing_outputs_lists_exactly_the_files_an_export_would_replace() {
    let dir = temp_dir("existing");
    let options = ExportOptions {
        panorama: true,
        hdri: true,
        ..faces_only(8)
    };
    assert!(existing_outputs(&dir, &options).is_empty());
    std::fs::write(dir.join("sky_Up.png"), "x").unwrap();
    std::fs::write(dir.join("sky_panorama.png"), "x").unwrap();
    std::fs::write(dir.join("sky.exr"), "x").unwrap();
    std::fs::write(dir.join("other.png"), "x").unwrap();

    let mut names: Vec<String> = existing_outputs(&dir, &options)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();

    assert_eq!(names, ["sky.exr", "sky_Up.png", "sky_panorama.png"]);
    // A file the export would not write (faces off) is not a conflict.
    let panorama_only = ExportOptions {
        faces: false,
        hdri: false,
        ..options
    };
    assert_eq!(existing_outputs(&dir, &panorama_only).len(), 1);
}

#[test]
fn export_failures_are_explained_in_plain_language_with_something_to_try() {
    use std::io::{Error, ErrorKind};
    let message = |kind| ExportError::Io(Error::from(kind)).user_message();

    assert!(message(ErrorKind::PermissionDenied).contains("not allowed to write"));
    assert!(message(ErrorKind::NotFound).contains("folder"));
    assert!(message(ErrorKind::StorageFull).contains("full"));
    assert!(message(ErrorKind::OutOfMemory).contains("memory"));
    assert_eq!(ExportError::Cancelled.user_message(), "Export cancelled.");
    // Anything else still reads as a sentence, with the system's own words kept for support.
    let other = message(ErrorKind::Other);
    assert!(other.starts_with("Could not write"), "{other}");
}

#[test]
fn exporting_into_a_path_that_is_a_file_is_an_error_not_a_panic() {
    let Ok(renderer) = Renderer::software() else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };
    let dir = temp_dir("dir-is-file");
    let not_a_folder = dir.join("file.txt");
    std::fs::write(&not_a_folder, "x").unwrap();

    let result = export_skybox(
        &renderer,
        &star_project(),
        &not_a_folder,
        &faces_only(8),
        |_, _| {},
    );

    let error = result.unwrap_err();
    assert!(matches!(error, ExportError::Io(_)));
    assert!(!error.user_message().is_empty());
}

#[test]
fn a_jpeg_panorama_stays_within_what_jpeg_and_memory_allow() {
    use spacegen_render::panorama_width_for;
    // PNG panoramas are four Faces wide at any size; JPEG cannot be 65536 wide (its limit is
    // 65535) and holds 8-bit RGB in memory, so it stops at the 8K Face's 32768.
    assert_eq!(panorama_width_for(512, ImageFormat::Png), 2048);
    assert_eq!(panorama_width_for(8192, ImageFormat::Png), 32768);
    assert_eq!(panorama_width_for(16384, ImageFormat::Png), 65536);
    assert_eq!(panorama_width_for(512, ImageFormat::Jpeg), 2048);
    assert_eq!(panorama_width_for(8192, ImageFormat::Jpeg), 32768);
    assert_eq!(panorama_width_for(16384, ImageFormat::Jpeg), 32768);
}
