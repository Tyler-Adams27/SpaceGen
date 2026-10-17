mod app;
mod icon;
mod layer_cap;
mod preview;
mod preview_view;
mod quality;
mod scaled_sky;
mod seed_source;
mod settings;
mod unsaved;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("SpaceGen")
            .with_icon(icon::app_icon()),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let result = eframe::run_native(
        "SpaceGen",
        options,
        Box::new(|cc| Ok(Box::new(app::SpacegenApp::new(cc)))),
    );
    if let Err(error) = &result {
        // Usually no usable graphics card or driver. A window would not open, so say why in a dialog.
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("SpaceGen could not start")
            .set_description(format!(
                "SpaceGen could not open its window. It needs a graphics card (GPU) that supports Vulkan, DirectX 12 or Metal. Update your graphics driver and try again.

Details: {error}"
            ))
            .show();
    }
    result
}
