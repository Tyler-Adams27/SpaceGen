use serde::{Deserialize, Serialize};

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seed(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LayerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    StarField,
    Nebula,
    GalaxyBand,
    Sun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    Additive,
    Screen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Layer {
    id: LayerId,
    kind: LayerKind,
    blend: BlendMode,
    opacity: f32,
    visible: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    format_version: u32,
    seed: Seed,
    next_layer_id: u64,
    layers: Vec<Layer>,
}

#[derive(Debug)]
pub enum ProjectError {
    Json(serde_json::Error),
    UnsupportedVersion { found: u32, supported: u32 },
    InvalidOpacity { layer: LayerId, opacity: f32 },
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Json(error) => write!(f, "invalid project file: {error}"),
            ProjectError::UnsupportedVersion { found, supported } => write!(
                f,
                "project file uses format version {found}, but this version of SpaceGen only supports up to format version {supported}"
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

#[derive(Deserialize)]
struct VersionHeader {
    format_version: u32,
}

impl Project {
    pub fn new(seed: Seed) -> Project {
        Project {
            format_version: FORMAT_VERSION,
            seed,
            next_layer_id: 0,
            layers: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, kind: LayerKind) -> LayerId {
        let id = LayerId(self.next_layer_id);
        self.next_layer_id += 1;
        self.layers.push(Layer {
            id,
            kind,
            blend: BlendMode::Additive,
            opacity: 1.0,
            visible: true,
        });
        id
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a Project always serializes")
    }

    pub fn from_json(json: &str) -> Result<Project, ProjectError> {
        let header: VersionHeader = serde_json::from_str(json).map_err(ProjectError::Json)?;
        if header.format_version > FORMAT_VERSION {
            return Err(ProjectError::UnsupportedVersion {
                found: header.format_version,
                supported: FORMAT_VERSION,
            });
        }
        let project: Project = serde_json::from_str(json).map_err(ProjectError::Json)?;
        for layer in &project.layers {
            if !(0.0..=1.0).contains(&layer.opacity) {
                return Err(ProjectError::InvalidOpacity {
                    layer: layer.id,
                    opacity: layer.opacity,
                });
            }
        }
        Ok(project)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineConvention {
    Unity,
    Unreal,
    Godot,
}

impl EngineConvention {
    pub fn face_file_name(self, name: &str, face: Face, format: ImageFormat) -> String {
        let label = match (self, face) {
            (EngineConvention::Godot, Face::PositiveX) => "px",
            (EngineConvention::Godot, Face::NegativeX) => "nx",
            (EngineConvention::Godot, Face::PositiveY) => "py",
            (EngineConvention::Godot, Face::NegativeY) => "ny",
            (EngineConvention::Godot, Face::PositiveZ) => "pz",
            (EngineConvention::Godot, Face::NegativeZ) => "nz",
            (EngineConvention::Unreal, Face::PositiveX) => "PositiveX",
            (EngineConvention::Unreal, Face::NegativeX) => "NegativeX",
            (EngineConvention::Unreal, Face::PositiveY) => "PositiveY",
            (EngineConvention::Unreal, Face::NegativeY) => "NegativeY",
            (EngineConvention::Unreal, Face::PositiveZ) => "PositiveZ",
            (EngineConvention::Unreal, Face::NegativeZ) => "NegativeZ",
            (EngineConvention::Unity, Face::PositiveX) => "Left",
            (EngineConvention::Unity, Face::NegativeX) => "Right",
            (EngineConvention::Unity, Face::PositiveY) => "Up",
            (EngineConvention::Unity, Face::NegativeY) => "Down",
            (EngineConvention::Unity, Face::PositiveZ) => "Front",
            (EngineConvention::Unity, Face::NegativeZ) => "Back",
        };
        let extension = match format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
        };
        format!("{name}_{label}.{extension}")
    }
}

impl Seed {
    pub fn layer_seed(self, layer: LayerId) -> Seed {
        const GAMMA: u64 = 0x9E3779B97F4A7C15;
        let state = self
            .0
            .wrapping_add(layer.0.wrapping_mul(GAMMA))
            .wrapping_add(GAMMA);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        Seed(z ^ (z >> 31))
    }
}
