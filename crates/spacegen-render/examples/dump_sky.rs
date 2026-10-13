//! Renders a sample star field sky to PNG files, for eyeballing the look while tuning
//! the constants at the top of `sky.wgsl`.
//!
//! `cargo run -p spacegen-render --example dump_sky -- <output dir> [face size]`

use spacegen_core::{EngineConvention, ImageFormat, LayerKind, Project, Seed};
use spacegen_render::{ExportOptions, Renderer, export_skybox};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: dump_sky <output dir> [face size]");
    let face_size: u32 = args.next().map_or(512, |s| s.parse().expect("face size"));

    let renderer = Renderer::gpu().expect("no GPU adapter");
    let mut project = Project::new(Seed(2024));
    project.add_layer(LayerKind::Nebula);
    project.add_layer(LayerKind::StarField);
    let options = ExportOptions {
        name: "sky".to_string(),
        face_size,
        format: ImageFormat::Png,
        convention: EngineConvention::Unity,
        faces: true,
        panorama: true,
        hdri: false,
        jpeg_quality: 95,
    };
    let written = export_skybox(&renderer, &project, dir.as_ref(), &options, |_, _| {})
        .expect("export failed");
    for path in written {
        println!("{}", path.display());
    }
}
