mod basis;
mod export;
mod sky;

pub use basis::{FaceBasis, camera_basis, face_basis};
pub use export::{ExportError, ExportOptions, export_skybox, linear_to_srgb, panorama_width};
pub use sky::{MAX_LAYERS, Projection, SkyPipeline, SkyUniforms};

use spacegen_core::{Face, Project};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
const BYTES_PER_PIXEL: u32 = 16;
/// How much of an image one GPU pass reads back at most, unless overridden.
const BAND_BUDGET_BYTES: u32 = 64 * 1024 * 1024;

/// A rendered image: `width * height` linear RGBA pixels, row by row from the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct SkyImage {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

/// A rendered cubemap Face (square).
pub type FaceImage = SkyImage;

impl SkyImage {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The side length of a square image such as a Face.
    pub fn size(&self) -> u32 {
        self.width
    }

    pub fn pixels(&self) -> &[[f32; 4]] {
        &self.pixels
    }
}

#[derive(Debug)]
pub struct RenderError {
    message: String,
    cancelled: bool,
}

impl RenderError {
    fn failed(message: String) -> RenderError {
        RenderError {
            message,
            cancelled: false,
        }
    }

    fn cancelled() -> RenderError {
        RenderError {
            message: "cancelled".to_string(),
            cancelled: true,
        }
    }

    /// True when rendering stopped because the Renderer's [`CancelToken`] was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// A shared flag that asks a running render or export to stop. Clones share the flag.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl CancelToken {
    pub fn new() -> CancelToken {
        CancelToken::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl std::error::Error for RenderError {}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: SkyPipeline,
    max_band_rows: Option<u32>,
    cancel: CancelToken,
}

impl Renderer {
    /// A renderer on the software adapter: slow, but identical on every machine.
    pub fn software() -> Result<Renderer, RenderError> {
        Self::on_adapter(true)
    }

    /// A renderer on the machine's best GPU.
    pub fn gpu() -> Result<Renderer, RenderError> {
        Self::on_adapter(false)
    }

    fn on_adapter(software: bool) -> Result<Renderer, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: software,
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|error| RenderError::failed(format!("no suitable adapter: {error}")))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| {
                    RenderError::failed(format!("could not open the adapter: {error}"))
                })?;
        let pipeline = SkyPipeline::new(&device, FORMAT);
        Ok(Renderer {
            device,
            queue,
            pipeline,
            max_band_rows: None,
            cancel: CancelToken::new(),
        })
    }

    /// Renders at most this many rows per GPU pass. By default the band height is chosen so a
    /// pass reads back about 64 MiB, which keeps 8K Faces inside the GPU's buffer limits.
    pub fn with_max_band_rows(mut self, rows: u32) -> Renderer {
        self.max_band_rows = Some(rows.max(1));
        self
    }

    /// Makes renders and exports on this Renderer stop, between bands and between images, once
    /// `token` is cancelled.
    pub fn with_cancel_token(mut self, token: CancelToken) -> Renderer {
        self.cancel = token;
        self
    }

    pub fn cancel_token(&self) -> &CancelToken {
        &self.cancel
    }

    /// Renders one cubemap Face, `size` pixels square.
    pub fn render_face(
        &self,
        project: &Project,
        face: Face,
        size: u32,
    ) -> Result<FaceImage, RenderError> {
        let uniforms = SkyUniforms::new(
            project,
            Projection::Basis(face_basis(face)),
            2.0 / size as f32,
        );
        self.render(&uniforms, size, size)
    }

    /// Renders one cubemap Face band by band, handing each band to `sink` as soon as it is read
    /// back: `sink(first_row, pixels)` gets whole rows, top to bottom. Only one band is held in
    /// memory at a time. An error from `sink` stops the render and is returned as it is.
    pub fn render_face_rows<E: From<RenderError>>(
        &self,
        project: &Project,
        face: Face,
        size: u32,
        sink: impl FnMut(u32, &[[f32; 4]]) -> Result<(), E>,
    ) -> Result<(), E> {
        let uniforms = SkyUniforms::new(
            project,
            Projection::Basis(face_basis(face)),
            2.0 / size as f32,
        );
        self.render_rows(&uniforms, size, size, sink)
    }

    /// Like [`Renderer::render_face_rows`], for the equirectangular panorama.
    pub fn render_equirectangular_rows<E: From<RenderError>>(
        &self,
        project: &Project,
        width: u32,
        sink: impl FnMut(u32, &[[f32; 4]]) -> Result<(), E>,
    ) -> Result<(), E> {
        let uniforms = SkyUniforms::new(
            project,
            Projection::Equirectangular,
            std::f32::consts::TAU / width as f32,
        );
        self.render_rows(&uniforms, width, width / 2, sink)
    }

    /// Renders the whole sky as an equirectangular panorama, `width` by `width / 2` pixels.
    pub fn render_equirectangular(
        &self,
        project: &Project,
        width: u32,
    ) -> Result<SkyImage, RenderError> {
        let uniforms = SkyUniforms::new(
            project,
            Projection::Equirectangular,
            std::f32::consts::TAU / width as f32,
        );
        self.render(&uniforms, width, width / 2)
    }

    fn render(
        &self,
        uniforms: &SkyUniforms,
        width: u32,
        height: u32,
    ) -> Result<SkyImage, RenderError> {
        let mut pixels = Vec::with_capacity((width * height) as usize);
        self.render_rows(uniforms, width, height, |_, band| {
            pixels.extend_from_slice(band);
            Ok::<(), RenderError>(())
        })?;
        Ok(SkyImage {
            width,
            height,
            pixels,
        })
    }

    /// Renders band by band, calling `sink(first_row, pixels)` for each, top to bottom.
    fn render_rows<E: From<RenderError>>(
        &self,
        uniforms: &SkyUniforms,
        width: u32,
        height: u32,
        mut sink: impl FnMut(u32, &[[f32; 4]]) -> Result<(), E>,
    ) -> Result<(), E> {
        let row_bytes = width * BYTES_PER_PIXEL;
        let rows_per_band = self
            .max_band_rows
            .unwrap_or_else(|| (BAND_BUDGET_BYTES / row_bytes.max(1)).max(1))
            .clamp(1, height.max(1));
        let mut pixels = Vec::with_capacity((width * rows_per_band) as usize);
        let mut start = 0;
        while start < height {
            if self.cancel.is_cancelled() {
                return Err(RenderError::cancelled().into());
            }
            let rows = rows_per_band.min(height - start);
            let band = uniforms.with_band(
                start as f32 / height as f32,
                (start + rows) as f32 / height as f32,
            );
            pixels.clear();
            self.render_band(&band, width, rows, &mut pixels)?;
            sink(start, &pixels)?;
            start += rows;
        }
        Ok(())
    }

    /// Renders one band of rows and appends its pixels.
    fn render_band(
        &self,
        uniforms: &SkyUniforms,
        width: u32,
        height: u32,
        pixels: &mut Vec<[f32; 4]>,
    ) -> Result<(), RenderError> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sky"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (uniform_buffer, bind_group) = self.pipeline.create_uniform_binding(&self.device);
        self.queue
            .write_buffer(&uniform_buffer, 0, uniforms.as_bytes());

        // Rows in a texture-to-buffer copy must be padded to a 256-byte multiple.
        let row_bytes = width * BYTES_PER_PIXEL;
        let padded_row_bytes = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded_row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sky"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.pipeline.draw(&mut pass, &bind_group);
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);

        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| RenderError::failed(format!("GPU wait failed: {error}")))?;
        receiver
            .recv()
            .map_err(|error| RenderError::failed(format!("GPU readback lost: {error}")))?
            .map_err(|error| RenderError::failed(format!("GPU readback failed: {error}")))?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|error| RenderError::failed(format!("GPU readback unavailable: {error}")))?;
        for row in mapped.chunks(padded_row_bytes as usize) {
            let floats: &[f32] = bytemuck::cast_slice(&row[..row_bytes as usize]);
            pixels.extend_from_slice(floats.as_chunks::<4>().0);
        }
        drop(mapped);
        readback.unmap();
        Ok(())
    }
}
