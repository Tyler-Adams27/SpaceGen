//! The Preview: the same sky shader the exports use, drawn straight into the window.

use eframe::egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::{egui, egui_wgpu, wgpu};
use spacegen_render::{SkyPipeline, SkyUniforms};

use crate::scaled_sky::ScaledSky;

/// GPU objects kept alive between frames, stored in egui's callback resources.
pub struct PreviewResources {
    pipeline: SkyPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    scaled: ScaledSky,
}

impl PreviewResources {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> PreviewResources {
        let pipeline = SkyPipeline::new(device, target_format);
        let (uniform_buffer, bind_group) = pipeline.create_uniform_binding(device);
        PreviewResources {
            pipeline,
            uniform_buffer,
            bind_group,
            scaled: ScaledSky::new(device, target_format),
        }
    }
}

/// One frame of the Preview: upload the uniforms, then draw the sky into the panel.
pub struct PreviewCallback {
    pub uniforms: SkyUniforms,
    /// Draw the sky at this size in pixels and stretch it over the panel, instead of at full size.
    pub reduced_size: Option<[u32; 2]>,
}

impl PreviewCallback {
    pub fn paint_callback(self, rect: egui::Rect) -> egui::PaintCallback {
        egui_wgpu::Callback::new_paint_callback(rect, self)
    }
}

impl CallbackTrait for PreviewCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(preview) = resources.get_mut::<PreviewResources>() {
            queue.write_buffer(&preview.uniform_buffer, 0, self.uniforms.as_bytes());
            if let Some(size) = self.reduced_size {
                preview.scaled.render(
                    device,
                    encoder,
                    &preview.pipeline,
                    &preview.bind_group,
                    size,
                );
            }
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        if let Some(preview) = resources.get::<PreviewResources>() {
            if self.reduced_size.is_some() {
                preview.scaled.blit(pass);
            } else {
                preview.pipeline.draw(pass, &preview.bind_group);
            }
        }
    }
}
