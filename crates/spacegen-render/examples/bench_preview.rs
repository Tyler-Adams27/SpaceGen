//! Times Preview-sized frames of the sky shader on every GPU in the machine, with 1, 8 and 16
//! Layers of each kind, at 1080p and 4K. Frame time is submit to GPU-idle, the median of many
//! frames after a warm-up.
//!
//! `cargo run --release -p spacegen-render --example bench_preview`

use std::time::Instant;

use spacegen_core::{LayerKind, Project, Seed};
use spacegen_render::{Projection, SkyPipeline, SkyUniforms, camera_basis};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

fn project(kind: LayerKind, count: usize) -> Project {
    let mut project = Project::new(Seed(7));
    for _ in 0..count {
        project.add_layer(kind);
    }
    project
}

fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    for adapter in adapters {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu {
            continue;
        }
        println!(
            "\n== {} ({:?}, {:?}) ==",
            info.name, info.device_type, info.backend
        );
        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            println!("  could not open");
            continue;
        };
        let pipeline = SkyPipeline::new(&device, FORMAT);
        let (buffer, bind_group) = pipeline.create_uniform_binding(&device);
        for (label, width, height) in [("1080p", 1920u32, 1080u32), ("4K", 3840, 2160)] {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            for kind in [
                LayerKind::StarField,
                LayerKind::Nebula,
                LayerKind::GalaxyBand,
                LayerKind::Sun,
            ] {
                for count in [1usize, 8, 16] {
                    let basis = camera_basis(0.4, 0.15, 1.2, width as f32 / height as f32);
                    let uniforms = SkyUniforms::new(
                        &project(kind, count),
                        Projection::Basis(basis),
                        1.2 / height as f32,
                    );
                    queue.write_buffer(&buffer, 0, uniforms.as_bytes());
                    let mut times = Vec::new();
                    for frame in 0..25 {
                        let start = Instant::now();
                        let mut encoder = device.create_command_encoder(&Default::default());
                        {
                            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: None,
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
                            pipeline.draw(&mut pass, &bind_group);
                        }
                        queue.submit([encoder.finish()]);
                        let _ = device.poll(wgpu::PollType::wait_indefinitely());
                        if frame >= 5 {
                            times.push(start.elapsed().as_secs_f64() * 1000.0);
                        }
                    }
                    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    println!(
                        "  {label:>5} {:>10} x{count:>2}: {:7.2} ms",
                        format!("{kind:?}"),
                        times[times.len() / 2]
                    );
                }
            }
        }
    }
}
