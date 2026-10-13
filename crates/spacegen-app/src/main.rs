mod app;
mod preview;
mod seed_source;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("SpaceGen"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "SpaceGen",
        options,
        Box::new(|cc| Ok(Box::new(app::SpacegenApp::new(cc)))),
    )
}
