//! Writing rendered skies to disk.

use std::path::{Path, PathBuf};

use spacegen_core::{EngineConvention, Face, ImageFormat, Project};

use crate::{RenderError, Renderer, SkyImage};

const FACES: [Face; 6] = [
    Face::PositiveX,
    Face::NegativeX,
    Face::PositiveY,
    Face::NegativeY,
    Face::PositiveZ,
    Face::NegativeZ,
];

/// The widest panorama the MVP exports; wider would need more GPU memory than is safe to assume.
const MAX_PANORAMA_WIDTH: u32 = 8192;

/// Width of the exported panorama: four Faces wide (matching the Faces' pixel density at the
/// equator), capped at 8192 pixels.
pub fn panorama_width(face_size: u32) -> u32 {
    face_size.saturating_mul(4).min(MAX_PANORAMA_WIDTH)
}

/// Encodes linear light (clamped to 0..1) with the sRGB transfer curve.
pub fn linear_to_srgb(linear: f32) -> f32 {
    // f64 so that pure white stays exactly 1.0 after the round trip through the curve.
    let linear = f64::from(linear.clamp(0.0, 1.0));
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    encoded.clamp(0.0, 1.0) as f32
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Base name of every file written.
    pub name: String,
    /// Side length of each cubemap Face in pixels.
    pub face_size: u32,
    pub format: ImageFormat,
    pub convention: EngineConvention,
    /// Write the six individual Faces.
    pub faces: bool,
    /// Write one equirectangular panorama image.
    pub panorama: bool,
    /// Write the equirectangular panorama as a floating-point OpenEXR HDRI.
    pub hdri: bool,
    /// JPEG quality, 1 to 100.
    pub jpeg_quality: u8,
}

#[derive(Debug)]
pub enum ExportError {
    Render(RenderError),
    Io(std::io::Error),
    Image(image::ImageError),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Render(error) => write!(f, "rendering failed: {error}"),
            ExportError::Io(error) => write!(f, "could not write a file: {error}"),
            ExportError::Image(error) => write!(f, "could not encode an image: {error}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<RenderError> for ExportError {
    fn from(error: RenderError) -> Self {
        ExportError::Render(error)
    }
}

impl From<std::io::Error> for ExportError {
    fn from(error: std::io::Error) -> Self {
        ExportError::Io(error)
    }
}

impl From<image::ImageError> for ExportError {
    fn from(error: image::ImageError) -> Self {
        ExportError::Image(error)
    }
}

/// Renders the Skybox and writes the requested images into `dir`, returning the files written.
/// `progress(done, total)` is called before each image and once at the end.
pub fn export_skybox(
    renderer: &Renderer,
    project: &Project,
    dir: &Path,
    options: &ExportOptions,
    mut progress: impl FnMut(usize, usize),
) -> Result<Vec<PathBuf>, ExportError> {
    std::fs::create_dir_all(dir)?;
    let total = if options.faces { FACES.len() } else { 0 }
        + usize::from(options.panorama)
        + usize::from(options.hdri);
    let mut written = Vec::new();

    if options.faces {
        for face in FACES {
            progress(written.len(), total);
            let image = renderer.render_face(project, face, options.face_size)?;
            let path = dir.join(options.convention.face_file_name(
                &options.name,
                face,
                options.format,
            ));
            save_image(&image, &path, options)?;
            written.push(path);
        }
    }
    if options.panorama || options.hdri {
        progress(written.len(), total);
        let image = renderer.render_equirectangular(project, panorama_width(options.face_size))?;
        if options.panorama {
            let path = dir.join(format!(
                "{}_panorama.{}",
                options.name,
                extension(options.format)
            ));
            save_image(&image, &path, options)?;
            written.push(path);
        }
        if options.hdri {
            progress(written.len(), total);
            let path = dir.join(format!("{}.exr", options.name));
            save_exr(&image, &path)?;
            written.push(path);
        }
    }
    progress(written.len(), total);
    Ok(written)
}

fn save_exr(image: &SkyImage, path: &Path) -> Result<(), ExportError> {
    let mut buffer = image::Rgba32FImage::new(image.width(), image.height());
    for (target, source) in buffer.pixels_mut().zip(image.pixels()) {
        *target = image::Rgba(*source);
    }
    buffer.save(path)?;
    Ok(())
}

fn extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
    }
}

fn save_image(image: &SkyImage, path: &Path, options: &ExportOptions) -> Result<(), ExportError> {
    match options.format {
        ImageFormat::Png => {
            let mut buffer = image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::new(
                image.width(),
                image.height(),
            );
            for (target, source) in buffer.pixels_mut().zip(image.pixels()) {
                *target = image::Rgb(std::array::from_fn(|c| {
                    (linear_to_srgb(source[c]) * 65535.0).round() as u16
                }));
            }
            buffer.save(path)?;
        }
        ImageFormat::Jpeg => {
            let mut buffer = image::RgbImage::new(image.width(), image.height());
            for (target, source) in buffer.pixels_mut().zip(image.pixels()) {
                *target = image::Rgb(std::array::from_fn(|c| {
                    (linear_to_srgb(source[c]) * 255.0).round() as u8
                }));
            }
            let file = std::io::BufWriter::new(std::fs::File::create(path)?);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                file,
                options.jpeg_quality.clamp(1, 100),
            );
            buffer.write_with_encoder(encoder)?;
        }
    }
    Ok(())
}
