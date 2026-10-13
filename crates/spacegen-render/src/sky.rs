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
    pad: u32,
    params: [f32; 4],
    tint_opacity: [f32; 4],
}

/// Mirrors `Globals` in `sky.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SkyUniforms {
    origin: [f32; 4],
    right: [f32; 4],
    down: [f32; 4],
    info: [u32; 4],
    pixel: [f32; 4],
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
            let Some(settings) = layer.star_field_settings() else {
                continue;
            };
            if !layer.visible() || count == MAX_LAYERS {
                continue;
            }
            let seed = project.seed().layer_seed(layer.id()).0;
            layers[count] = GpuLayer {
                seed: [seed as u32, (seed >> 32) as u32],
                blend: match layer.blend() {
                    BlendMode::Additive => 0,
                    BlendMode::Screen => 1,
                },
                pad: 0,
                params: [
                    settings.density,
                    settings.brightness,
                    settings.size,
                    settings.color_variation,
                ],
                tint_opacity: [
                    settings.tint[0],
                    settings.tint[1],
                    settings.tint[2],
                    layer.opacity(),
                ],
            };
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
            layers,
        }
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
