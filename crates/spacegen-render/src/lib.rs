use spacegen_core::{Face, Project};

/// Where the pixels of one cubemap Face look: the direction at image position
/// `(u, v)` is `origin + u * right + v * down`, with `(0, 0)` the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceBasis {
    pub origin: [f32; 3],
    pub right: [f32; 3],
    pub down: [f32; 3],
}

impl FaceBasis {
    pub fn direction(&self, u: f32, v: f32) -> [f32; 3] {
        std::array::from_fn(|axis| self.origin[axis] + u * self.right[axis] + v * self.down[axis])
    }
}

pub fn face_basis(face: Face) -> FaceBasis {
    match face {
        Face::PositiveX => FaceBasis {
            origin: [1.0, 1.0, 1.0],
            right: [0.0, 0.0, -2.0],
            down: [0.0, -2.0, 0.0],
        },
        Face::NegativeX => FaceBasis {
            origin: [-1.0, 1.0, -1.0],
            right: [0.0, 0.0, 2.0],
            down: [0.0, -2.0, 0.0],
        },
        Face::PositiveY => FaceBasis {
            origin: [-1.0, 1.0, -1.0],
            right: [2.0, 0.0, 0.0],
            down: [0.0, 0.0, 2.0],
        },
        Face::NegativeY => FaceBasis {
            origin: [-1.0, -1.0, 1.0],
            right: [2.0, 0.0, 0.0],
            down: [0.0, 0.0, -2.0],
        },
        Face::PositiveZ => FaceBasis {
            origin: [-1.0, 1.0, 1.0],
            right: [2.0, 0.0, 0.0],
            down: [0.0, -2.0, 0.0],
        },
        Face::NegativeZ => FaceBasis {
            origin: [1.0, 1.0, -1.0],
            right: [-2.0, 0.0, 0.0],
            down: [0.0, -2.0, 0.0],
        },
    }
}

/// A rendered Face: `size * size` linear RGBA pixels, row by row from the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceImage {
    size: u32,
    pixels: Vec<[f32; 4]>,
}

impl FaceImage {
    pub fn size(&self) -> u32 {
        self.size
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

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
const BYTES_PER_PIXEL: u32 = 16;

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Renderer {
    /// A renderer on the software adapter: slow, but identical on every machine.
    pub fn software() -> Result<Renderer, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: true,
            ..Default::default()
        }))
        .map_err(|error| RenderError(format!("no software adapter: {error}")))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| RenderError(format!("could not open the adapter: {error}")))?;
        Ok(Renderer { device, queue })
    }

    pub fn render_face(
        &self,
        _project: &Project,
        _face: Face,
        size: u32,
    ) -> Result<FaceImage, RenderError> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("face"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
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

        // Rows in a texture-to-buffer copy must be padded to a 256-byte multiple.
        let row_bytes = size * BYTES_PER_PIXEL;
        let padded_row_bytes = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded_row_bytes) * u64::from(size),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(size),
                },
            },
            wgpu::Extent3d {
                width: size,
                height: size,
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
        let mut pixels = Vec::with_capacity((size * size) as usize);
        for row in mapped.chunks(padded_row_bytes as usize) {
            let floats: &[f32] = bytemuck::cast_slice(&row[..row_bytes as usize]);
            pixels.extend(floats.chunks_exact(4).map(|p| [p[0], p[1], p[2], p[3]]));
        }
        drop(mapped);
        readback.unmap();
        Ok(FaceImage { size, pixels })
    }
}
