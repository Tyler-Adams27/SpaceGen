use spacegen_render::camera_basis;

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
}

#[test]
fn a_camera_with_no_rotation_looks_down_positive_z() {
    let basis = camera_basis(0.0, 0.0, 90f32.to_radians(), 1.0);

    assert!(close(basis.direction(0.5, 0.5), [0.0, 0.0, 1.0]));
}

#[test]
fn the_image_edges_sit_half_the_field_of_view_away_from_the_centre() {
    // 90 degree vertical field of view, square: tan(45 degrees) = 1.
    let basis = camera_basis(0.0, 0.0, 90f32.to_radians(), 1.0);

    assert!(close(basis.direction(1.0, 0.5), [1.0, 0.0, 1.0])); // right edge
    assert!(close(basis.direction(0.5, 0.0), [0.0, 1.0, 1.0])); // top edge
    assert!(close(basis.direction(0.0, 1.0), [-1.0, -1.0, 1.0])); // bottom-left corner
}

#[test]
fn a_wider_aspect_ratio_widens_only_the_horizontal_field() {
    let basis = camera_basis(0.0, 0.0, 90f32.to_radians(), 2.0);

    assert!(close(basis.direction(1.0, 0.5), [2.0, 0.0, 1.0]));
    assert!(close(basis.direction(0.5, 0.0), [0.0, 1.0, 1.0]));
}

#[test]
fn yaw_turns_toward_positive_x_and_pitch_looks_up() {
    let quarter = std::f32::consts::FRAC_PI_2;

    let right = camera_basis(quarter, 0.0, 1.0, 1.0);
    assert!(close(right.direction(0.5, 0.5), [1.0, 0.0, 0.0]));

    let up = camera_basis(0.0, quarter, 1.0, 1.0);
    assert!(close(up.direction(0.5, 0.5), [0.0, 1.0, 0.0]));
}
