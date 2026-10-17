//! Writing rendered skies to disk.

use std::io::Write;
use std::path::{Path, PathBuf};

use spacegen_core::{EngineConvention, Face, HdriFormat, ImageFormat, PngBitDepth, Project};

use crate::{RenderError, Renderer, SkyImage};

const FACES: [Face; 6] = [
    Face::PositiveX,
    Face::NegativeX,
    Face::PositiveY,
    Face::NegativeY,
    Face::PositiveZ,
    Face::NegativeZ,
];

/// The widest HDRI exported: its float pixels (16 bytes each) are held in memory whole, so 8192 by
/// 4096 is about 512 MiB. The PNG and JPEG panorama is streamed and has no such limit.
const MAX_HDRI_WIDTH: u32 = 8192;

/// Width of the exported PNG or JPEG panorama: four Faces wide, matching the Faces' pixel density
/// at the equator. Not capped: an 8192 Face gives a 32768 by 16384 panorama.
pub fn panorama_width(face_size: u32) -> u32 {
    face_size.saturating_mul(4)
}

/// The widest JPEG panorama: JPEG cannot be 65536 pixels wide (its limit is 65535) and the encoder
/// holds 8-bit RGB in memory (about 1.6 GiB at 32768 x 16384), so it stops at an 8192 Face's width.
const MAX_JPEG_PANORAMA_WIDTH: u32 = 32768;

/// Width of the panorama file for this format: [`panorama_width`], except that a JPEG stops at
/// 32768 pixels.
pub fn panorama_width_for(face_size: u32, format: ImageFormat) -> u32 {
    let width = panorama_width(face_size);
    match format {
        ImageFormat::Png => width,
        ImageFormat::Jpeg => width.min(MAX_JPEG_PANORAMA_WIDTH),
    }
}

/// Width of the exported HDRI: the panorama width, capped at 8192 pixels (see `MAX_HDRI_WIDTH`).
pub fn hdri_width(face_size: u32) -> u32 {
    panorama_width(face_size).min(MAX_HDRI_WIDTH)
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
    /// Write the equirectangular panorama as a floating-point HDRI, in `hdri_format`.
    pub hdri: bool,
    pub hdri_format: HdriFormat,
    /// Bits per channel of PNG output; ignored for JPEG.
    pub png_bit_depth: PngBitDepth,
    /// JPEG quality, 1 to 100.
    pub jpeg_quality: u8,
}

#[derive(Debug)]
pub enum ExportError {
    /// The export was cancelled; every file it had written was removed.
    Cancelled,
    Render(RenderError),
    Io(std::io::Error),
    Image(image::ImageError),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Cancelled => write!(f, "export cancelled"),
            ExportError::Render(error) => write!(f, "rendering failed: {error}"),
            ExportError::Io(error) => write!(f, "could not write a file: {error}"),
            ExportError::Image(error) => write!(f, "could not encode an image: {error}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl ExportError {
    /// A sentence for the user saying what went wrong and what to try. The `Display` text stays
    /// technical, for logs and support.
    pub fn user_message(&self) -> String {
        use std::io::ErrorKind;
        match self {
            ExportError::Cancelled => "Export cancelled.".to_string(),
            ExportError::Render(error) => error.user_message(),
            ExportError::Io(error) => match error.kind() {
                ErrorKind::PermissionDenied => "SpaceGen is not allowed to write to that folder. Choose another folder (for example inside Documents) and export again.".to_string(),
                ErrorKind::NotFound => "The export folder could not be found or created. Check that the drive is connected, then choose the folder again.".to_string(),
                ErrorKind::AlreadyExists | ErrorKind::NotADirectory => "The export location is not a folder. Choose a folder and export again.".to_string(),
                ErrorKind::StorageFull => "The disk is full. Free some space or choose another folder, then export again.".to_string(),
                ErrorKind::ReadOnlyFilesystem => "That folder is read-only. Choose another folder and export again.".to_string(),
                ErrorKind::OutOfMemory => "The computer ran out of memory while exporting. Choose a smaller face size and try again.".to_string(),
                _ => format!("Could not write the export files: {error}"),
            },
            ExportError::Image(error) => format!("Could not encode an image: {error}"),
        }
    }
}

impl From<RenderError> for ExportError {
    fn from(error: RenderError) -> Self {
        if error.is_cancelled() {
            return ExportError::Cancelled;
        }
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
///
/// If the Renderer's [`CancelToken`](crate::CancelToken) is cancelled, the export stops between
/// images (and between bands within one), removes the files it had written and returns
/// [`ExportError::Cancelled`].
pub fn export_skybox(
    renderer: &Renderer,
    project: &Project,
    dir: &Path,
    options: &ExportOptions,
    progress: impl FnMut(usize, usize),
) -> Result<Vec<PathBuf>, ExportError> {
    let mut written = Vec::new();
    let result = write_skybox(renderer, project, dir, options, progress, &mut written);
    if matches!(result, Err(ExportError::Cancelled)) {
        for path in &written {
            let _ = std::fs::remove_file(path);
        }
    }
    result.map(|()| written)
}

fn write_skybox(
    renderer: &Renderer,
    project: &Project,
    dir: &Path,
    options: &ExportOptions,
    mut progress: impl FnMut(usize, usize),
    written: &mut Vec<PathBuf>,
) -> Result<(), ExportError> {
    let cancel = renderer.cancel_token();
    let check = || {
        if cancel.is_cancelled() {
            Err(ExportError::Cancelled)
        } else {
            Ok(())
        }
    };
    std::fs::create_dir_all(dir)?;
    let total = if options.faces { FACES.len() } else { 0 }
        + usize::from(options.panorama)
        + usize::from(options.hdri);

    if options.faces {
        for face in FACES {
            check()?;
            progress(written.len(), total);
            let path = face_path(dir, options, face);
            let size = options.face_size;
            stream_image(&path, size, size, options, |rows| {
                renderer.render_face_rows(project, face, size, |_, band| rows.write(band))
            })?;
            written.push(path);
        }
    }
    if options.panorama {
        check()?;
        progress(written.len(), total);
        // Streamed, so its width is not limited by memory.
        let width = panorama_width_for(options.face_size, options.format);
        let path = panorama_path(dir, options);
        stream_image(&path, width, width / 2, options, |rows| {
            renderer.render_equirectangular_rows(project, width, |_, band| rows.write(band))
        })?;
        written.push(path);
    }
    if options.hdri {
        check()?;
        progress(written.len(), total);
        // The HDRI keeps float pixels, so its whole image is held in memory: capped width.
        let image = renderer.render_equirectangular(project, hdri_width(options.face_size))?;
        let path = hdri_path(dir, options);
        write_via_temp(&path, |temp| match options.hdri_format {
            HdriFormat::OpenExr => save_exr(&image, temp),
            HdriFormat::Radiance => save_radiance(&image, temp),
        })?;
        written.push(path);
    }
    progress(written.len(), total);
    Ok(())
}

/// Receives a rendered image band by band and writes it to disk, converting linear light to sRGB
/// on the way. PNG rows go straight to the file; JPEG keeps 8-bit RGB only (a fifth of the float
/// image) because the encoder needs the whole picture.
struct RowWriter {
    width: u32,
    height: u32,
    rows_written: u32,
    sink: RowSink,
}

enum RowSink {
    Png {
        stream: Box<png::StreamWriter<'static, std::io::BufWriter<std::fs::File>>>,
        sixteen: bool,
    },
    Jpeg {
        rgb: Vec<u8>,
        file: std::io::BufWriter<std::fs::File>,
        quality: u8,
    },
}

impl RowWriter {
    fn create(
        path: &Path,
        width: u32,
        height: u32,
        options: &ExportOptions,
    ) -> Result<RowWriter, ExportError> {
        let file = std::io::BufWriter::new(std::fs::File::create(path)?);
        let sink = match options.format {
            ImageFormat::Png => {
                let sixteen = options.png_bit_depth == PngBitDepth::Sixteen;
                let mut encoder = png::Encoder::new(file, width, height);
                encoder.set_color(png::ColorType::Rgb);
                // The same settings the `image` crate used before streaming: fast, adaptive filtering.
                encoder.set_compression(png::Compression::Fast);
                encoder.set_filter(png::Filter::Adaptive);
                encoder.set_depth(if sixteen {
                    png::BitDepth::Sixteen
                } else {
                    png::BitDepth::Eight
                });
                let stream = encoder
                    .write_header()
                    .and_then(|writer| writer.into_stream_writer())
                    .map_err(png_error)?;
                RowSink::Png {
                    stream: Box::new(stream),
                    sixteen,
                }
            }
            ImageFormat::Jpeg => RowSink::Jpeg {
                rgb: Vec::with_capacity(width as usize * height as usize * 3),
                file,
                quality: options.jpeg_quality.clamp(1, 100),
            },
        };
        Ok(RowWriter {
            width,
            height,
            rows_written: 0,
            sink,
        })
    }

    /// Writes the next band of whole rows.
    fn write(&mut self, band: &[[f32; 4]]) -> Result<(), ExportError> {
        self.rows_written += (band.len() / self.width as usize) as u32;
        match &mut self.sink {
            RowSink::Png { stream, sixteen } => {
                let mut bytes = Vec::with_capacity(band.len() * if *sixteen { 6 } else { 3 });
                for pixel in band {
                    for channel in &pixel[..3] {
                        let encoded = linear_to_srgb(*channel);
                        if *sixteen {
                            let value = (encoded * 65535.0).round() as u16;
                            bytes.extend_from_slice(&value.to_be_bytes());
                        } else {
                            bytes.push((encoded * 255.0).round() as u8);
                        }
                    }
                }
                stream.write_all(&bytes)?;
            }
            RowSink::Jpeg { rgb, .. } => {
                for pixel in band {
                    rgb.extend(
                        pixel[..3]
                            .iter()
                            .map(|c| (linear_to_srgb(*c) * 255.0).round() as u8),
                    );
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<(), ExportError> {
        match self.sink {
            RowSink::Png { stream, .. } => stream.finish().map_err(png_error)?,
            RowSink::Jpeg {
                rgb,
                mut file,
                quality,
            } => {
                let encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality);
                image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(
                    self.width,
                    self.height,
                    rgb,
                )
                .expect("every row was written")
                .write_with_encoder(encoder)?;
                file.flush()?;
            }
        }
        Ok(())
    }
}

fn png_error(error: png::EncodingError) -> ExportError {
    match error {
        png::EncodingError::IoError(error) => ExportError::Io(error),
        other => ExportError::Io(std::io::Error::other(other)),
    }
}

/// Renders an image straight into `path` through a [`RowWriter`]. The bytes go to a temporary name
/// next to `path` and are renamed over it only when complete, so a failed or cancelled export never
/// leaves a corrupt image or damages an earlier file of the same name.
fn stream_image(
    path: &Path,
    width: u32,
    height: u32,
    options: &ExportOptions,
    render: impl FnOnce(&mut RowWriter) -> Result<(), ExportError>,
) -> Result<(), ExportError> {
    write_via_temp(path, |temp| {
        let mut rows = RowWriter::create(temp, width, height, options)?;
        render(&mut rows)?;
        rows.finish()
    })
}

/// Runs `write` against a temporary name beside `path`, then renames it over `path`. On any error
/// the temporary file is removed and `path` is left as it was.
fn write_via_temp(
    path: &Path,
    write: impl FnOnce(&Path) -> Result<(), ExportError>,
) -> Result<(), ExportError> {
    let temp = temp_path(path);
    let result =
        write(&temp).and_then(|()| std::fs::rename(&temp, path).map_err(ExportError::from));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    path.with_file_name(name)
}

fn face_path(dir: &Path, options: &ExportOptions, face: Face) -> PathBuf {
    dir.join(
        options
            .convention
            .face_file_name(&options.name, face, options.format),
    )
}

fn panorama_path(dir: &Path, options: &ExportOptions) -> PathBuf {
    dir.join(format!(
        "{}_panorama.{}",
        options.name,
        extension(options.format)
    ))
}

fn hdri_path(dir: &Path, options: &ExportOptions) -> PathBuf {
    dir.join(format!(
        "{}.{}",
        options.name,
        hdri_extension(options.hdri_format)
    ))
}

/// The files in `dir` that an export with these options would overwrite, so the caller can ask
/// first.
pub fn existing_outputs(dir: &Path, options: &ExportOptions) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if options.faces {
        paths.extend(FACES.iter().map(|face| face_path(dir, options, *face)));
    }
    if options.panorama {
        paths.push(panorama_path(dir, options));
    }
    if options.hdri {
        paths.push(hdri_path(dir, options));
    }
    paths.retain(|path| path.exists());
    paths
}

fn save_exr(image: &SkyImage, path: &Path) -> Result<(), ExportError> {
    let mut buffer = image::Rgba32FImage::new(image.width(), image.height());
    for (target, source) in buffer.pixels_mut().zip(image.pixels()) {
        *target = image::Rgba(*source);
    }
    buffer.save_with_format(path, image::ImageFormat::OpenExr)?;
    Ok(())
}

fn save_radiance(image: &SkyImage, path: &Path) -> Result<(), ExportError> {
    let pixels: Vec<image::Rgb<f32>> = image
        .pixels()
        .iter()
        .map(|p| image::Rgb([p[0], p[1], p[2]]))
        .collect();
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    image::codecs::hdr::HdrEncoder::new(&mut file).encode(
        &pixels,
        image.width() as usize,
        image.height() as usize,
    )?;
    file.flush()?;
    Ok(())
}

fn hdri_extension(format: HdriFormat) -> &'static str {
    match format {
        HdriFormat::OpenExr => "exr",
        HdriFormat::Radiance => "hdr",
    }
}

fn extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
    }
}
