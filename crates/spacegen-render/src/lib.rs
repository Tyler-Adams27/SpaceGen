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
