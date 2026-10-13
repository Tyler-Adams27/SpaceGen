use spacegen_core::Face;
use spacegen_render::face_basis;

// Expected directions are worked out by hand from the Vulkan "Cube Map Face
// Selection" table (u = (sc/|ma| + 1) / 2, v = (tc/|ma| + 1) / 2), not computed
// by the code under test. (u, v) = (0, 0) is the top-left of the image.

#[test]
fn positive_x_face_corners_follow_the_vulkan_table() {
    let basis = face_basis(Face::PositiveX);

    assert_eq!(basis.direction(0.0, 0.0), [1.0, 1.0, 1.0]);
    assert_eq!(basis.direction(1.0, 0.0), [1.0, 1.0, -1.0]);
    assert_eq!(basis.direction(0.0, 1.0), [1.0, -1.0, 1.0]);
}

#[test]
fn negative_x_face_corners_follow_the_vulkan_table() {
    let basis = face_basis(Face::NegativeX);

    assert_eq!(basis.direction(0.0, 0.0), [-1.0, 1.0, -1.0]);
    assert_eq!(basis.direction(1.0, 0.0), [-1.0, 1.0, 1.0]);
    assert_eq!(basis.direction(0.0, 1.0), [-1.0, -1.0, -1.0]);
}

#[test]
fn positive_z_face_corners_follow_the_vulkan_table() {
    let basis = face_basis(Face::PositiveZ);

    assert_eq!(basis.direction(0.0, 0.0), [-1.0, 1.0, 1.0]);
    assert_eq!(basis.direction(1.0, 0.0), [1.0, 1.0, 1.0]);
    assert_eq!(basis.direction(0.0, 1.0), [-1.0, -1.0, 1.0]);
}

#[test]
fn negative_z_face_corners_follow_the_vulkan_table() {
    let basis = face_basis(Face::NegativeZ);

    assert_eq!(basis.direction(0.0, 0.0), [1.0, 1.0, -1.0]);
    assert_eq!(basis.direction(1.0, 0.0), [-1.0, 1.0, -1.0]);
    assert_eq!(basis.direction(0.0, 1.0), [1.0, -1.0, -1.0]);
}

#[test]
fn positive_y_face_corners_follow_the_vulkan_table() {
    let basis = face_basis(Face::PositiveY);

    assert_eq!(basis.direction(0.0, 0.0), [-1.0, 1.0, -1.0]);
    assert_eq!(basis.direction(1.0, 0.0), [1.0, 1.0, -1.0]);
    assert_eq!(basis.direction(0.0, 1.0), [-1.0, 1.0, 1.0]);
}

#[test]
fn negative_y_face_corners_follow_the_vulkan_table() {
    let basis = face_basis(Face::NegativeY);

    assert_eq!(basis.direction(0.0, 0.0), [-1.0, -1.0, 1.0]);
    assert_eq!(basis.direction(1.0, 0.0), [1.0, -1.0, 1.0]);
    assert_eq!(basis.direction(0.0, 1.0), [-1.0, -1.0, -1.0]);
}
