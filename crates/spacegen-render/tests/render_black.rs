use spacegen_core::{Face, Project, Seed};
use spacegen_render::Renderer;

fn renderer() -> Option<Renderer> {
    match Renderer::software() {
        Ok(renderer) => Some(renderer),
        Err(error) => {
            eprintln!("SKIPPED: no software adapter available: {error}");
            None
        }
    }
}

#[test]
fn a_project_without_star_fields_renders_opaque_black() {
    let Some(renderer) = renderer() else { return };
    let project = Project::new(Seed(5));

    let image = renderer.render_face(&project, Face::PositiveZ, 16).unwrap();

    assert_eq!(image.size(), 16);
    assert_eq!(image.pixels().len(), 16 * 16);
    assert!(image.pixels().iter().all(|pixel| *pixel == [0.0, 0.0, 0.0, 1.0]));
}
