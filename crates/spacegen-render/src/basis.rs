use spacegen_core::Face;

/// Where the pixels of one cubemap Face look: the direction at image position
/// `(u, v)` is `origin + u * right + v * down`, with `(0, 0)` the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceBasis {
    pub origin: [f32; 3],
    pub right: [f32; 3],
    pub down: [f32; 3],
}

impl FaceBasis {
    pub fn direction(&self, u: f32, v: f32) -> [f32; 3] {
        std::array::from_fn(|axis| self.origin[axis] + u * self.right[axis] + v * self.down[axis])
    }
}

/// A perspective camera at the origin. `yaw` turns from +Z toward +X, `pitch` looks up
/// from the horizon, `vertical_fov` is the full vertical field of view, all in radians.
pub fn camera_basis(yaw: f32, pitch: f32, vertical_fov: f32, aspect: f32) -> FaceBasis {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let forward = [sin_yaw * cos_pitch, sin_pitch, cos_yaw * cos_pitch];
    let right = [cos_yaw, 0.0, -sin_yaw];
    let up = [-sin_yaw * sin_pitch, cos_pitch, -cos_yaw * sin_pitch];
    let half_height = (vertical_fov / 2.0).tan();
    let half_width = half_height * aspect;
    FaceBasis {
        origin: std::array::from_fn(|a| forward[a] - half_width * right[a] + half_height * up[a]),
        right: std::array::from_fn(|a| 2.0 * half_width * right[a]),
        down: std::array::from_fn(|a| -2.0 * half_height * up[a]),
    }
}

pub fn face_basis(face: Face) -> FaceBasis {
    match face {
        Face::PositiveX => FaceBasis {
            origin: [1.0, 1.0, 1.0],
            right: [0.0, 0.0, -2.0],
            down: [0.0, -2.0, 0.0],
        },
        Face::NegativeX => FaceBasis {
            origin: [-1.0, 1.0, -1.0],
            right: [0.0, 0.0, 2.0],
            down: [0.0, -2.0, 0.0],
        },
        Face::PositiveY => FaceBasis {
            origin: [-1.0, 1.0, -1.0],
            right: [2.0, 0.0, 0.0],
            down: [0.0, 0.0, 2.0],
        },
        Face::NegativeY => FaceBasis {
            origin: [-1.0, -1.0, 1.0],
            right: [2.0, 0.0, 0.0],
            down: [0.0, 0.0, -2.0],
        },
        Face::PositiveZ => FaceBasis {
            origin: [-1.0, 1.0, 1.0],
            right: [2.0, 0.0, 0.0],
            down: [0.0, -2.0, 0.0],
        },
        Face::NegativeZ => FaceBasis {
            origin: [1.0, 1.0, -1.0],
            right: [-2.0, 0.0, 0.0],
            down: [0.0, -2.0, 0.0],
        },
    }
}
