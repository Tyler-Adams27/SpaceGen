use crate::seed::LayerId;

#[derive(Debug)]
pub enum ProjectError {
    Json(serde_json::Error),
    UnsupportedVersion {
        found: u32,
        supported: u32,
    },
    InvalidOpacity {
        layer: LayerId,
        opacity: f32,
    },
    InvalidSetting {
        layer: LayerId,
        field: &'static str,
        value: f32,
    },
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Json(error) => write!(f, "invalid project file: {error}"),
            ProjectError::UnsupportedVersion { found, supported } => write!(
                f,
                "project file uses format version {found}, but this version of SpaceGen only supports up to format version {supported}"
            ),
            ProjectError::InvalidSetting {
                layer,
                field,
                value,
            } => write!(
                f,
                "layer {} has {field} value {value}, which is outside 0.0 to 1.0",
                layer.0
            ),
            ProjectError::InvalidOpacity { layer, opacity } => write!(
                f,
                "layer {} has opacity {opacity}, which is outside 0.0 to 1.0",
                layer.0
            ),
        }
    }
}

impl std::error::Error for ProjectError {}

#[derive(Debug, PartialEq)]
pub enum EditError {
    InvalidOpacity {
        opacity: f32,
    },
    UnknownLayer {
        layer: LayerId,
    },
    IndexOutOfRange {
        index: usize,
        layer_count: usize,
    },
    InvalidSetting {
        field: &'static str,
        value: f32,
    },
    WrongKind {
        layer: LayerId,
        expected: &'static str,
    },
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::WrongKind { layer, expected } => {
                write!(f, "layer {} is not a {expected} layer", layer.0)
            }
            EditError::InvalidSetting { field, value } => {
                write!(f, "{field} value {value} is outside 0.0 to 1.0")
            }
            EditError::IndexOutOfRange { index, layer_count } => write!(
                f,
                "index {index} is out of range for a stack of {layer_count} layers"
            ),
            EditError::UnknownLayer { layer } => write!(f, "there is no layer {}", layer.0),
            EditError::InvalidOpacity { opacity } => {
                write!(f, "opacity {opacity} is outside 0.0 to 1.0")
            }
        }
    }
}

impl std::error::Error for EditError {}
