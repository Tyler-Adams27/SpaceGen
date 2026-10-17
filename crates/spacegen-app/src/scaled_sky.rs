//! Draws the sky into a smaller offscreen texture and stretches it over the Preview, for when the
//! full-size sky is too slow to follow the user's mouse (see `quality.rs`).

use eframe::wgpu;
use spacegen_render::SkyPipeline;

const SHADER: &str = "
struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    // One triangle covering the whole viewport; uv runs from the top-left corner.
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VsOut;
    out.position = vec4<f32>(corner.x * 2.0 - 1.0, 1.0 - corner.y * 2.0, 0.0, 1.0);
    out.uv = corner;
    return out;
}

@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, in.uv);
}
";

struct Offscreen {
    view: wgpu::TextureView,
    size: [u32; 2],
    bind_group: wgpu::BindGroup,
}

/// An offscreen copy of the sky at a reduced size, and the pass that stretches it over a viewport.
pub struct ScaledSky {
    format: wgpu::TextureFormat,
    blit: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    offscreen: Option<Offscreen>,
}

impl ScaledSky {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> ScaledSky {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scaled sky"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scaled sky"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scaled sky"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let blit = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scaled sky"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        ScaledSky {
            format,
            blit,
            layout,
            sampler,
            offscreen: None,
        }
    }

    /// Draws the sky (uniforms already uploaded to `sky_bind_group`) into the offscreen texture
    /// of `size` pixels, making or resizing the texture first if needed.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        sky: &SkyPipeline,
        sky_bind_group: &wgpu::BindGroup,
        size: [u32; 2],
    ) {
        let size = [size[0].max(1), size[1].max(1)];
        if self.offscreen.as_ref().is_none_or(|o| o.size != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scaled sky"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("scaled sky"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.offscreen = Some(Offscreen {
                view,
                size,
                bind_group,
            });
        }
        let Some(offscreen) = &self.offscreen else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scaled sky"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &offscreen.view,
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
        sky.draw(&mut pass, sky_bind_group);
    }

    /// Stretches the offscreen sky over the pass's whole viewport. Does nothing before `render`.
    pub fn blit(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(offscreen) = &self.offscreen {
            pass.set_pipeline(&self.blit);
            pass.set_bind_group(0, &offscreen.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegen_core::{LayerKind, Project, Seed};
    use spacegen_render::{Projection, SkyUniforms, camera_basis};

    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const SIDE: u32 = 32;

    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: true,
            ..Default::default()
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
    }

    /// A smooth sky with a clear feature (a galaxy band across the view), so a stretched copy should
    /// look like the real thing and a flipped or shifted one should not.
    fn smooth_uniforms(side: u32) -> SkyUniforms {
        let mut project = Project::new(Seed(3));
        project.add_layer(LayerKind::GalaxyBand);
        SkyUniforms::new(
            &project,
            Projection::Basis(camera_basis(0.4, 0.15, 1.2, 1.0)),
            1.2 / side as f32,
        )
    }

    /// Draws into a SIDE x SIDE target with `draw` and reads the RGBA bytes back.
    fn render_target(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draw: impl FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView),
    ) -> Vec<u8> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: SIDE,
                height: SIDE,
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
        let row = SIDE * 4; // 128 bytes; copies need 256
        let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(padded * SIDE),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        draw(&mut encoder, &view);
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(SIDE),
                },
            },
            wgpu::Extent3d {
                width: SIDE,
                height: SIDE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        mapped
            .chunks(padded as usize)
            .flat_map(|r| r[..row as usize].iter().copied())
            .collect()
    }

    fn clear_pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'a wgpu::TextureView,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        })
    }

    fn mean_difference(a: &[u8], b: &[u8]) -> f32 {
        let total: u64 = a
            .iter()
            .zip(b)
            .map(|(x, y)| u64::from(x.abs_diff(*y)))
            .sum();
        total as f32 / a.len() as f32 / 255.0
    }

    #[test]
    fn a_half_size_sky_stretched_over_the_target_looks_like_the_full_size_sky() {
        let Some((device, queue)) = device() else {
            eprintln!("SKIPPED: no software adapter");
            return;
        };
        let sky = SkyPipeline::new(&device, FORMAT);
        let (buffer, bind_group) = sky.create_uniform_binding(&device);
        queue.write_buffer(&buffer, 0, smooth_uniforms(SIDE).as_bytes());
        let direct = render_target(&device, &queue, |encoder, view| {
            let mut pass = clear_pass(encoder, view);
            sky.draw(&mut pass, &bind_group);
        });

        let mut scaled = ScaledSky::new(&device, FORMAT);
        queue.write_buffer(&buffer, 0, smooth_uniforms(SIDE / 2).as_bytes());
        let stretched = render_target(&device, &queue, |encoder, view| {
            scaled.render(&device, encoder, &sky, &bind_group, [SIDE / 2, SIDE / 2]);
            let mut pass = clear_pass(encoder, view);
            scaled.blit(&mut pass);
        });

        assert!(direct.iter().any(|&b| b > 20), "the reference sky is black");
        let difference = mean_difference(&direct, &stretched);
        assert!(difference < 0.01, "stretched sky differs by {difference}");
    }

    #[test]
    fn the_offscreen_texture_follows_the_requested_size() {
        let Some((device, queue)) = device() else {
            eprintln!("SKIPPED: no software adapter");
            return;
        };
        let sky = SkyPipeline::new(&device, FORMAT);
        let (_, bind_group) = sky.create_uniform_binding(&device);
        let mut scaled = ScaledSky::new(&device, FORMAT);
        let mut encoder = device.create_command_encoder(&Default::default());

        scaled.render(&device, &mut encoder, &sky, &bind_group, [16, 8]);
        assert_eq!(scaled.offscreen.as_ref().map(|o| o.size), Some([16, 8]));
        scaled.render(&device, &mut encoder, &sky, &bind_group, [20, 20]);
        assert_eq!(scaled.offscreen.as_ref().map(|o| o.size), Some([20, 20]));
        scaled.render(&device, &mut encoder, &sky, &bind_group, [0, 0]);
        assert_eq!(scaled.offscreen.as_ref().map(|o| o.size), Some([1, 1]));
        queue.submit([encoder.finish()]);
    }
}
