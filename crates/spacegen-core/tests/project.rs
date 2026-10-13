use spacegen_core::{LayerId, LayerKind, Project, Seed};

#[test]
fn a_project_round_trips_through_json() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    project.add_layer(LayerKind::Nebula);

    let loaded = Project::from_json(&project.to_json()).unwrap();

    assert_eq!(loaded, project);
}

#[test]
fn layer_ids_are_distinct_and_increasing_and_continue_after_a_round_trip() {
    let mut project = Project::new(Seed(5));

    let ids = [
        project.add_layer(LayerKind::StarField),
        project.add_layer(LayerKind::Nebula),
        project.add_layer(LayerKind::Sun),
    ];
    assert_eq!(ids, [LayerId(0), LayerId(1), LayerId(2)]);

    let mut loaded = Project::from_json(&project.to_json()).unwrap();
    assert_eq!(loaded.add_layer(LayerKind::GalaxyBand), LayerId(3));
}

#[test]
fn a_layer_with_opacity_outside_zero_to_one_is_rejected_naming_the_layer() {
    let mut project = Project::new(Seed(5));
    project.add_layer(LayerKind::StarField);
    let second = project.add_layer(LayerKind::Nebula);
    assert_eq!(second, LayerId(1));
    let json = project.to_json();
    let marker = "\"opacity\": 1.0";
    let second_layer_opacity = json.rfind(marker).unwrap();
    let bad = format!(
        "{}\"opacity\": 1.5{}",
        &json[..second_layer_opacity],
        &json[second_layer_opacity + marker.len()..]
    );

    let error = Project::from_json(&bad).unwrap_err();

    assert!(
        error.to_string().contains("layer 1"),
        "unexpected message: {error}"
    );
}

#[test]
fn a_project_from_a_newer_format_version_is_rejected_with_a_clear_error() {
    let json = Project::new(Seed(5))
        .to_json()
        .replace("\"format_version\": 1", "\"format_version\": 2");

    let error = Project::from_json(&json).unwrap_err();

    assert!(
        error.to_string().contains("format version 2"),
        "unexpected message: {error}"
    );
}
