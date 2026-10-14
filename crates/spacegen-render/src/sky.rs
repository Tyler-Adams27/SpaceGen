//! The GPU side of the sky: uniform layout, the Project-to-uniform translation,
//! and the render pipeline shared by the Preview and by offscreen renders.

use bytemuck::{Pod, Zeroable};
use spacegen_core::{BlendMode, Project};

use crate::basis::FaceBasis;

pub const MAX_LAYERS: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLayer {
    seed: [u32; 2],
    blend: u32,
    /// 0 = star field, 1 = nebula, 2 = galaxy band, 3 = sun.
    kind: u32,
    /// Star field: density, brightness, size, color variation.
    /// Nebula: coverage, scale, brightness, unused.
    /// Galaxy band: tilt, width, brightness, dust.
    /// Sun: azimuth, elevation, size, brightness.
    params: [f32; 4],
    /// Star field tint, or nebula, galaxy band or sun color A, in rgb; the Layer's opacity in w.
    color_a_opacity: [f32; 4],
    /// Nebula or galaxy band color B in rgb; the sun's glare in x; unused for stars.
    color_b: [f32; 4],
}

const KIND_STAR_FIELD: u32 = 0;
const KIND_NEBULA: u32 = 1;
const KIND_GALAXY_BAND: u32 = 2;
const KIND_SUN: u32 = 3;

/// Mirrors `Globals` in `sky.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SkyUniforms {
    origin: [f32; 4],
    right: [f32; 4],
    down: [f32; 4],
    info: [u32; 4],
    pixel: [f32; 4],
    band: [f32; 4],
    layers: [GpuLayer; MAX_LAYERS],
}

/// How image positions map to sky directions.
#[derive(Clone, Copy)]
pub enum Projection {
    /// `origin + u * right + v * down`: a cubemap Face or a perspective camera.
    Basis(FaceBasis),
    /// A full 360 by 180 degree panorama.
    Equirectangular,
}

impl SkyUniforms {
    /// `pixel_angle` is the angle one pixel covers, in radians; stars never shrink below it.
    pub fn new(project: &Project, projection: Projection, pixel_angle: f32) -> SkyUniforms {
        let mut layers = [GpuLayer::zeroed(); MAX_LAYERS];
        let mut count = 0;
        for layer in project.layers() {
            if !layer.visible() || count == MAX_LAYERS {
                continue;
            }
            let seed = layer.seed_under(project.seed()).0;
            let blend = match layer.blend() {
                BlendMode::Additive => 0,
                BlendMode::Screen => 1,
            };
            let rgb_opacity = |rgb: [f32; 3]| [rgb[0], rgb[1], rgb[2], layer.opacity()];
            let common = GpuLayer {
                seed: [seed as u32, (seed >> 32) as u32],
                blend,
                ..GpuLayer::zeroed()
            };
            if let Some(stars) = layer.star_field_settings() {
                layers[count] = GpuLayer {
                    kind: KIND_STAR_FIELD,
                    params: [
                        stars.density,
                        stars.brightness,
                        stars.size,
                        stars.color_variation,
                    ],
                    color_a_opacity: rgb_opacity(stars.tint),
                    ..common
                };
            } else if let Some(nebula) = layer.nebula_settings() {
                layers[count] = GpuLayer {
                    kind: KIND_NEBULA,
                    params: [nebula.coverage, nebula.scale, nebula.brightness, 0.0],
                    color_a_opacity: rgb_opacity(nebula.color_a),
                    color_b: [nebula.color_b[0], nebula.color_b[1], nebula.color_b[2], 0.0],
                    ..common
                };
            } else if let Some(band) = layer.galaxy_band_settings() {
                layers[count] = GpuLayer {
                    kind: KIND_GALAXY_BAND,
                    params: [band.tilt, band.width, band.brightness, band.dust],
                    color_a_opacity: rgb_opacity(band.color_a),
                    color_b: [band.color_b[0], band.color_b[1], band.color_b[2], 0.0],
                    ..common
                };
            } else if let Some(sun) = layer.sun_settings() {
                layers[count] = GpuLayer {
                    kind: KIND_SUN,
                    params: [sun.azimuth, sun.elevation, sun.size, sun.brightness],
                    color_a_opacity: rgb_opacity(sun.color),
                    color_b: [sun.glare, 0.0, 0.0, 0.0],
                    ..common
                };
            } else {
                continue;
            }
            count += 1;
        }

        let (basis, mode) = match projection {
            Projection::Basis(basis) => (basis, 0),
            Projection::Equirectangular => (
                FaceBasis {
                    origin: [0.0; 3],
                    right: [0.0; 3],
                    down: [0.0; 3],
                },
                1,
            ),
        };
        let pad = |v: [f32; 3]| [v[0], v[1], v[2], 0.0];
        SkyUniforms {
            origin: pad(basis.origin),
            right: pad(basis.right),
            down: pad(basis.down),
            info: [count as u32, mode, 0, 0],
            pixel: [pixel_angle, 0.0, 0.0, 0.0],
            band: [0.0, 1.0, 0.0, 0.0],
            layers,
        }
    }
}

impl SkyUniforms {
    /// Restricts the pass to the rows between start and end, as fractions of the image height.
    pub fn with_band(mut self, start: f32, end: f32) -> SkyUniforms {
        self.band = [start, end, 0.0, 0.0];
        self
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// The compiled sky pipeline for one render target format.
pub struct SkyPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl SkyPipeline {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> SkyPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sky.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(target_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        SkyPipeline {
            pipeline,
            bind_group_layout,
        }
    }

    /// A buffer and bind group holding one `SkyUniforms`; update it with `queue.write_buffer`.
    pub fn create_uniform_binding(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky uniforms"),
            size: std::mem::size_of::<SkyUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky uniforms"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, bind_group)
    }

    /// Draws the whole sky into a render pass.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, bind_group: &wgpu::BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
