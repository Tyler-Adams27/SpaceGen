//! The six Faces and the panorama are the same sky seen two ways. Look up each sampled
//! panorama pixel's direction in the Face it falls on, using the Vulkan cube map face
//! selection formulas, and check both renders show the same light.

use spacegen_core::{Face, LayerKind, NebulaSettings, Project, Seed};
use spacegen_render::{FaceImage, Renderer};

const PANORAMA_WIDTH: u32 = 256;
const FACE_SIZE: u32 = 128;

/// (face, u, v) for a direction, from the Vulkan "Cube Map Face Selection" table.
fn locate(d: [f32; 3]) -> (Face, f32, f32) {
    let [x, y, z] = d;
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let (face, major, s, t) = if ax >= ay && ax >= az {
        if x > 0.0 {
            (Face::PositiveX, ax, -z, -y)
        } else {
            (Face::NegativeX, ax, z, -y)
        }
    } else if ay >= az {
        if y > 0.0 {
            (Face::PositiveY, ay, x, z)
        } else {
            (Face::NegativeY, ay, x, -z)
        }
    } else if z > 0.0 {
        (Face::PositiveZ, az, x, -y)
    } else {
        (Face::NegativeZ, az, -x, -y)
    };
    (face, 0.5 * (s / major + 1.0), 0.5 * (t / major + 1.0))
}

/// The direction an equirectangular pixel looks toward: centre at +Z, +X a quarter turn right.
fn panorama_direction(u: f32, v: f32) -> [f32; 3] {
    let (lon, theta) = (u * std::f32::consts::TAU, v * std::f32::consts::PI);
    [lon.sin() * theta.sin(), theta.cos(), -lon.cos() * theta.sin()]
}

/// The mean difference per Face between the panorama and that Face's own render.
fn faces_and_panorama_agree(project: &Project) -> Option<Vec<(String, f32)>> {
    let renderer = Renderer::software().ok()?;
    let panorama = renderer.render_equirectangular(project, PANORAMA_WIDTH).unwrap();
    let faces: Vec<(Face, FaceImage)> = [
        Face::PositiveX,
        Face::NegativeX,
        Face::PositiveY,
        Face::NegativeY,
        Face::PositiveZ,
        Face::NegativeZ,
    ]
    .into_iter()
    .map(|face| (face, renderer.render_face(project, face, FACE_SIZE).unwrap()))
    .collect();

    let mut lit_per_face = std::collections::HashMap::new();
    let mut totals: std::collections::HashMap<String, (f32, u32)> = std::collections::HashMap::new();
    for y in (2..panorama.height()).step_by(5) {
        for x in (1..panorama.width()).step_by(3) {
            let u = (x as f32 + 0.5) / panorama.width() as f32;
            let v = (y as f32 + 0.5) / panorama.height() as f32;
            let (face, fu, fv) = locate(panorama_direction(u, v));
            let image = &faces.iter().find(|(f, _)| *f == face).unwrap().1;
            let px = ((fu * FACE_SIZE as f32) as u32).min(FACE_SIZE - 1);
            let py = ((fv * FACE_SIZE as f32) as u32).min(FACE_SIZE - 1);
            let from_face = image.pixels()[(py * FACE_SIZE + px) as usize];
            let from_panorama = panorama.pixels()[(y * panorama.width() + x) as usize];
            let entry = totals.entry(format!("{face:?}")).or_default();
            entry.0 += (0..3).map(|c| (from_face[c] - from_panorama[c]).abs()).sum::<f32>() / 3.0;
            entry.1 += 1;
            if from_panorama[0] > 0.05 {
                *lit_per_face.entry(format!("{face:?}")).or_insert(0) += 1;
            }
        }
    }
    for face in ["PositiveX", "NegativeX", "PositiveY", "NegativeY", "PositiveZ", "NegativeZ"] {
        let lit = lit_per_face.get(face).copied().unwrap_or(0);
        assert!(lit >= 20, "{face} is nearly black ({lit} lit samples); the check would prove nothing there");
    }
    let mut means: Vec<(String, f32)> = totals
        .into_iter()
        .map(|(face, (total, count))| (face, total / count as f32))
        .collect();
    means.sort_by(|a, b| a.0.cmp(&b.0));
    Some(means)
}

#[test]
fn the_six_faces_and_the_panorama_show_the_same_sky() {
    let mut project = Project::new(Seed(5));
    let nebula = project.add_layer(LayerKind::Nebula);
    project
        .set_nebula_settings(
            nebula,
            NebulaSettings {
                coverage: 1.0,
                scale: 0.9,
                ..NebulaSettings::default()
            },
        )
        .unwrap();

    let Some(means) = faces_and_panorama_agree(&project) else {
        eprintln!("SKIPPED: no software adapter");
        return;
    };

    for (face, mean) in &means {
        assert!(
            *mean < 0.02,
            "{face} and the panorama disagree: mean difference {mean}"
        );
    }
}