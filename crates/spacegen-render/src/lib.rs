mod basis;
mod sky;

pub use basis::{FaceBasis, face_basis};
pub use sky::{MAX_LAYERS, Projection, SkyPipeline, SkyUniforms};

use spacegen_core::{Face, Project};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
const BYTES_PER_PIXEL: u32 = 16;

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
pub struct RenderError(String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: SkyPipeline,
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
        .map_err(|error| RenderError(format!("no suitable adapter: {error}")))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| RenderError(format!("could not open the adapter: {error}")))?;
        let pipeline = SkyPipeline::new(&device, FORMAT);
        Ok(Renderer {
            device,
            queue,
            pipeline,
        })
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
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(uniforms));

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
            .map_err(|error| RenderError(format!("GPU wait failed: {error}")))?;
        receiver
            .recv()
            .map_err(|error| RenderError(format!("GPU readback lost: {error}")))?
            .map_err(|error| RenderError(format!("GPU readback failed: {error}")))?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|error| RenderError(format!("GPU readback unavailable: {error}")))?;
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for row in mapped.chunks(padded_row_bytes as usize) {
            let floats: &[f32] = bytemuck::cast_slice(&row[..row_bytes as usize]);
            pixels.extend(floats.chunks_exact(4).map(|p| [p[0], p[1], p[2], p[3]]));
        }
        drop(mapped);
        readback.unmap();
        Ok(SkyImage {
            width,
            height,
            pixels,
        })
    }
}
