use spacegen_core::{EngineConvention, Face, ImageFormat};

#[test]
fn unity_names_the_positive_x_face_left() {
    assert_eq!(
        EngineConvention::Unity.face_file_name("sky", Face::PositiveX, ImageFormat::Png),
        "sky_Left.png"
    );
}

#[test]
fn unity_names_all_six_faces_with_its_skybox_slot_labels() {
    let name = |face| EngineConvention::Unity.face_file_name("sky", face, ImageFormat::Png);

    assert_eq!(name(Face::PositiveX), "sky_Left.png");
    assert_eq!(name(Face::NegativeX), "sky_Right.png");
    assert_eq!(name(Face::PositiveY), "sky_Up.png");
    assert_eq!(name(Face::NegativeY), "sky_Down.png");
    assert_eq!(name(Face::PositiveZ), "sky_Front.png");
    assert_eq!(name(Face::NegativeZ), "sky_Back.png");
}

#[test]
fn godot_names_all_six_faces_in_layer_order_with_signed_axis_suffixes() {
    let name = |face| EngineConvention::Godot.face_file_name("sky", face, ImageFormat::Png);

    assert_eq!(name(Face::PositiveX), "sky_px.png");
    assert_eq!(name(Face::NegativeX), "sky_nx.png");
    assert_eq!(name(Face::PositiveY), "sky_py.png");
    assert_eq!(name(Face::NegativeY), "sky_ny.png");
    assert_eq!(name(Face::PositiveZ), "sky_pz.png");
    assert_eq!(name(Face::NegativeZ), "sky_nz.png");
}

#[test]
fn unreal_names_all_six_faces_in_cubemap_face_order() {
    let name = |face| EngineConvention::Unreal.face_file_name("sky", face, ImageFormat::Png);

    assert_eq!(name(Face::PositiveX), "sky_PositiveX.png");
    assert_eq!(name(Face::NegativeX), "sky_NegativeX.png");
    assert_eq!(name(Face::PositiveY), "sky_PositiveY.png");
    assert_eq!(name(Face::NegativeY), "sky_NegativeY.png");
    assert_eq!(name(Face::PositiveZ), "sky_PositiveZ.png");
    assert_eq!(name(Face::NegativeZ), "sky_NegativeZ.png");
}

#[test]
fn the_file_extension_follows_the_image_format() {
    assert_eq!(
        EngineConvention::Godot.face_file_name("sky", Face::PositiveY, ImageFormat::Jpeg),
        "sky_py.jpg"
    );
}
