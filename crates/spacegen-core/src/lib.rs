use serde::{Deserialize, Serialize};

const FORMAT_VERSION: u32 = 1;
const MAX_UNDO_STEPS: usize = 100;

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
pub struct Layer {
    id: LayerId,
    settings: LayerSettings,
    blend: BlendMode,
    opacity: f32,
    visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StarFieldSettings {
    pub density: f32,
    pub brightness: f32,
    pub size: f32,
    pub color_variation: f32,
    pub tint: [f32; 3],
}

impl StarFieldSettings {
    fn validate(&self) -> Result<(), EditError> {
        let fields = [
            ("density", self.density),
            ("brightness", self.brightness),
            ("size", self.size),
            ("color_variation", self.color_variation),
            ("tint", self.tint[0]),
            ("tint", self.tint[1]),
            ("tint", self.tint[2]),
        ];
        for (field, value) in fields {
            if !(0.0..=1.0).contains(&value) {
                return Err(EditError::InvalidSetting { field, value });
            }
        }
        Ok(())
    }
}

impl Default for StarFieldSettings {
    fn default() -> Self {
        StarFieldSettings {
            density: 0.5,
            brightness: 0.5,
            size: 0.5,
            color_variation: 0.5,
            tint: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum LayerSettings {
    StarField(StarFieldSettings),
    Nebula,
    GalaxyBand,
    Sun,
}

impl LayerSettings {
    fn default_for(kind: LayerKind) -> LayerSettings {
        match kind {
            LayerKind::StarField => LayerSettings::StarField(StarFieldSettings::default()),
            LayerKind::Nebula => LayerSettings::Nebula,
            LayerKind::GalaxyBand => LayerSettings::GalaxyBand,
            LayerKind::Sun => LayerSettings::Sun,
        }
    }

    fn kind(&self) -> LayerKind {
        match self {
            LayerSettings::StarField(_) => LayerKind::StarField,
            LayerSettings::Nebula => LayerKind::Nebula,
            LayerSettings::GalaxyBand => LayerKind::GalaxyBand,
            LayerSettings::Sun => LayerKind::Sun,
        }
    }
}

impl Layer {
    pub fn star_field_settings(&self) -> Option<&StarFieldSettings> {
        match &self.settings {
            LayerSettings::StarField(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn id(&self) -> LayerId {
        self.id
    }

    pub fn kind(&self) -> LayerKind {
        self.settings.kind()
    }

    pub fn blend(&self) -> BlendMode {
        self.blend
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn visible(&self) -> bool {
        self.visible
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            ProjectError::InvalidSetting { layer, field, value } => write!(
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
    InvalidOpacity { opacity: f32 },
    UnknownLayer { layer: LayerId },
    IndexOutOfRange { index: usize, layer_count: usize },
    InvalidSetting { field: &'static str, value: f32 },
    WrongKind { layer: LayerId, expected: &'static str },
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
            settings: LayerSettings::default_for(kind),
            blend: BlendMode::Additive,
            opacity: 1.0,
            visible: true,
        });
        id
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn set_seed(&mut self, seed: Seed) {
        self.seed = seed;
    }

    pub fn set_opacity(&mut self, layer: LayerId, opacity: f32) -> Result<(), EditError> {
        if !(0.0..=1.0).contains(&opacity) {
            return Err(EditError::InvalidOpacity { opacity });
        }
        self.layer_mut(layer)?.opacity = opacity;
        Ok(())
    }

    pub fn set_blend(&mut self, layer: LayerId, blend: BlendMode) -> Result<(), EditError> {
        self.layer_mut(layer)?.blend = blend;
        Ok(())
    }

    pub fn set_star_field_settings(
        &mut self,
        layer: LayerId,
        settings: StarFieldSettings,
    ) -> Result<(), EditError> {
        settings.validate()?;
        let target = self.layer_mut(layer)?;
        if !matches!(target.settings, LayerSettings::StarField(_)) {
            return Err(EditError::WrongKind {
                layer,
                expected: "star field",
            });
        }
        target.settings = LayerSettings::StarField(settings);
        Ok(())
    }

    pub fn set_visible(&mut self, layer: LayerId, visible: bool) -> Result<(), EditError> {
        self.layer_mut(layer)?.visible = visible;
        Ok(())
    }

    pub fn remove_layer(&mut self, layer: LayerId) -> Result<(), EditError> {
        let index = self.layer_index(layer)?;
        self.layers.remove(index);
        Ok(())
    }

    pub fn move_layer(&mut self, layer: LayerId, new_index: usize) -> Result<(), EditError> {
        let index = self.layer_index(layer)?;
        if new_index >= self.layers.len() {
            return Err(EditError::IndexOutOfRange {
                index: new_index,
                layer_count: self.layers.len(),
            });
        }
        let moved = self.layers.remove(index);
        self.layers.insert(new_index, moved);
        Ok(())
    }

    pub fn duplicate_layer(&mut self, layer: LayerId) -> Result<LayerId, EditError> {
        let index = self.layer_index(layer)?;
        let id = LayerId(self.next_layer_id);
        self.next_layer_id += 1;
        let mut copy = self.layers[index].clone();
        copy.id = id;
        self.layers.insert(index + 1, copy);
        Ok(id)
    }

    fn layer_index(&self, id: LayerId) -> Result<usize, EditError> {
        self.layers
            .iter()
            .position(|l| l.id == id)
            .ok_or(EditError::UnknownLayer { layer: id })
    }

    fn layer_mut(&mut self, id: LayerId) -> Result<&mut Layer, EditError> {
        self.layers
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or(EditError::UnknownLayer { layer: id })
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
            if let LayerSettings::StarField(settings) = &layer.settings {
                if let Err(EditError::InvalidSetting { field, value }) = settings.validate() {
                    return Err(ProjectError::InvalidSetting {
                        layer: layer.id,
                        field,
                        value,
                    });
                }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragField {
    Opacity,
    StarField,
}

pub struct History {
    project: Project,
    undo_stack: Vec<Project>,
    redo_stack: Vec<Project>,
    last_drag: Option<(LayerId, DragField)>,
}

impl History {
    pub fn new(project: Project) -> History {
        History {
            project,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_drag: None,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn add_layer(&mut self, kind: LayerKind) -> LayerId {
        self.edit(None, |project| Ok(project.add_layer(kind)))
            .expect("adding a layer cannot fail")
    }

    pub fn set_opacity(&mut self, layer: LayerId, opacity: f32) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::Opacity)), |project| {
            project.set_opacity(layer, opacity)
        })
    }

    pub fn set_blend(&mut self, layer: LayerId, blend: BlendMode) -> Result<(), EditError> {
        self.edit(None, |project| project.set_blend(layer, blend))
    }

    pub fn set_visible(&mut self, layer: LayerId, visible: bool) -> Result<(), EditError> {
        self.edit(None, |project| project.set_visible(layer, visible))
    }

    pub fn set_star_field_settings(
        &mut self,
        layer: LayerId,
        settings: StarFieldSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::StarField)), |project| {
            project.set_star_field_settings(layer, settings)
        })
    }

    pub fn set_seed(&mut self, seed: Seed) {
        self.edit(None, |project| {
            project.set_seed(seed);
            Ok(())
        })
        .expect("setting the seed cannot fail");
    }

    pub fn remove_layer(&mut self, layer: LayerId) -> Result<(), EditError> {
        self.edit(None, |project| project.remove_layer(layer))
    }

    pub fn move_layer(&mut self, layer: LayerId, new_index: usize) -> Result<(), EditError> {
        self.edit(None, |project| project.move_layer(layer, new_index))
    }

    pub fn duplicate_layer(&mut self, layer: LayerId) -> Result<LayerId, EditError> {
        self.edit(None, |project| project.duplicate_layer(layer))
    }

    /// Runs `change` and, only if it succeeds, records the prior state as an undo step.
    /// An edit whose `drag_of` names the same Layer and field as the previous edit's joins that
    /// edit's undo step instead of starting a new one.
    fn edit<T>(
        &mut self,
        drag_of: Option<(LayerId, DragField)>,
        change: impl FnOnce(&mut Project) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        let before = self.project.clone();
        let result = change(&mut self.project)?;
        let continues_drag = drag_of.is_some() && drag_of == self.last_drag;
        if !continues_drag {
            self.undo_stack.push(before);
            if self.undo_stack.len() > MAX_UNDO_STEPS {
                self.undo_stack.remove(0);
            }
        }
        self.redo_stack.clear();
        self.last_drag = drag_of;
        Ok(result)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.project, previous);
            self.redo_stack.push(current);
            self.last_drag = None;
        }
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.project, next);
            self.undo_stack.push(current);
            self.last_drag = None;
        }
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
