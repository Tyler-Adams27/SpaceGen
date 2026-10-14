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
        check_unit_range(&fields)
    }
}

/// Every value must lie in 0.0 to 1.0 (NaN fails); names the first field that does not.
fn check_unit_range(fields: &[(&'static str, f32)]) -> Result<(), EditError> {
    for &(field, value) in fields {
        if !(0.0..=1.0).contains(&value) {
            return Err(EditError::InvalidSetting { field, value });
        }
    }
    Ok(())
}

impl NebulaSettings {
    fn validate(&self) -> Result<(), EditError> {
        check_unit_range(&[
            ("coverage", self.coverage),
            ("scale", self.scale),
            ("brightness", self.brightness),
            ("color_a", self.color_a[0]),
            ("color_a", self.color_a[1]),
            ("color_a", self.color_a[2]),
            ("color_b", self.color_b[0]),
            ("color_b", self.color_b[1]),
            ("color_b", self.color_b[2]),
        ])
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
    Nebula(NebulaSettings),
    GalaxyBand(GalaxyBandSettings),
    Sun(SunSettings),
}

impl LayerSettings {
    fn default_for(kind: LayerKind) -> LayerSettings {
        match kind {
            LayerKind::StarField => LayerSettings::StarField(StarFieldSettings::default()),
            LayerKind::Nebula => LayerSettings::Nebula(NebulaSettings::default()),
            LayerKind::GalaxyBand => LayerSettings::GalaxyBand(GalaxyBandSettings::default()),
            LayerKind::Sun => LayerSettings::Sun(SunSettings::default()),
        }
    }

    fn kind(&self) -> LayerKind {
        match self {
            LayerSettings::StarField(_) => LayerKind::StarField,
            LayerSettings::Nebula(_) => LayerKind::Nebula,
            LayerSettings::GalaxyBand(_) => LayerKind::GalaxyBand,
            LayerSettings::Sun(_) => LayerKind::Sun,
        }
    }
}

/// A bright star with a glare, placed by hand. Every field is 0.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SunSettings {
    /// Which way around the horizon the Sun lies: 0.0 is straight ahead (+Z), 0.25 a quarter turn
    /// to the right (+X), 0.5 behind.
    pub azimuth: f32,
    /// How high the Sun sits: 0.0 straight down, 0.5 on the horizon, 1.0 straight up.
    pub elevation: f32,
    /// Size of the bright disc.
    pub size: f32,
    pub brightness: f32,
    /// Strength of the glow and glare around the disc.
    pub glare: f32,
    pub color: [f32; 3],
}

impl Default for SunSettings {
    fn default() -> Self {
        SunSettings {
            azimuth: 0.0,
            elevation: 0.6,
            size: 0.5,
            brightness: 0.5,
            glare: 0.5,
            color: [1.0, 0.95, 0.85],
        }
    }
}

impl SunSettings {
    fn validate(&self) -> Result<(), EditError> {
        check_unit_range(&[
            ("azimuth", self.azimuth),
            ("elevation", self.elevation),
            ("size", self.size),
            ("brightness", self.brightness),
            ("glare", self.glare),
            ("color", self.color[0]),
            ("color", self.color[1]),
            ("color", self.color[2]),
        ])
    }
}

/// A Milky Way-style band of light across the sky. Every field is 0.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GalaxyBandSettings {
    /// Tilt of the band away from the horizon, 0.0 (flat) to 1.0 (vertical).
    pub tilt: f32,
    /// How wide the band is across the sky.
    pub width: f32,
    pub brightness: f32,
    /// How strongly dark dust lanes cut through the band.
    pub dust: f32,
    /// Color of the band's bright core.
    pub color_a: [f32; 3],
    /// Color of the band's outer glow.
    pub color_b: [f32; 3],
}

impl Default for GalaxyBandSettings {
    fn default() -> Self {
        GalaxyBandSettings {
            tilt: 0.3,
            width: 0.4,
            brightness: 0.5,
            dust: 0.5,
            color_a: [1.0, 0.85, 0.6],
            color_b: [0.5, 0.6, 1.0],
        }
    }
}

impl GalaxyBandSettings {
    fn validate(&self) -> Result<(), EditError> {
        check_unit_range(&[
            ("tilt", self.tilt),
            ("width", self.width),
            ("brightness", self.brightness),
            ("dust", self.dust),
            ("color_a", self.color_a[0]),
            ("color_a", self.color_a[1]),
            ("color_a", self.color_a[2]),
            ("color_b", self.color_b[0]),
            ("color_b", self.color_b[1]),
            ("color_b", self.color_b[2]),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NebulaSettings {
    pub coverage: f32,
    pub scale: f32,
    pub brightness: f32,
    pub color_a: [f32; 3],
    pub color_b: [f32; 3],
}

impl Default for NebulaSettings {
    fn default() -> Self {
        NebulaSettings {
            coverage: 0.5,
            scale: 0.5,
            brightness: 0.5,
            color_a: [0.2, 0.4, 1.0],
            color_b: [1.0, 0.3, 0.6],
        }
    }
}

impl Layer {
    pub fn nebula_settings(&self) -> Option<&NebulaSettings> {
        match &self.settings {
            LayerSettings::Nebula(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn sun_settings(&self) -> Option<&SunSettings> {
        match &self.settings {
            LayerSettings::Sun(settings) => Some(settings),
            _ => None,
        }
    }

    pub fn galaxy_band_settings(&self) -> Option<&GalaxyBandSettings> {
        match &self.settings {
            LayerSettings::GalaxyBand(settings) => Some(settings),
            _ => None,
        }
    }

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

    /// Gives every Layer new random colors derived from `seed`; nothing else changes. Star tints
    /// stay close to white; nebula and galaxy band colors are vivid, with the second hue a
    /// little way around the color wheel from the first.
    pub fn randomize_colors(&mut self, seed: Seed) {
        for layer in &mut self.layers {
            let mut picker = ColorPicker(seed.layer_seed(layer.id).0);
            match &mut layer.settings {
                LayerSettings::StarField(settings) => settings.tint = picker.pastel(),
                LayerSettings::Nebula(settings) => {
                    (settings.color_a, settings.color_b) = picker.vivid_pair();
                }
                LayerSettings::GalaxyBand(settings) => {
                    (settings.color_a, settings.color_b) = picker.vivid_pair();
                }
                LayerSettings::Sun(settings) => settings.color = picker.pastel(),
            }
        }
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

    pub fn set_nebula_settings(
        &mut self,
        layer: LayerId,
        settings: NebulaSettings,
    ) -> Result<(), EditError> {
        settings.validate()?;
        let target = self.layer_mut(layer)?;
        if !matches!(target.settings, LayerSettings::Nebula(_)) {
            return Err(EditError::WrongKind {
                layer,
                expected: "nebula",
            });
        }
        target.settings = LayerSettings::Nebula(settings);
        Ok(())
    }

    pub fn set_sun_settings(
        &mut self,
        layer: LayerId,
        settings: SunSettings,
    ) -> Result<(), EditError> {
        settings.validate()?;
        let target = self.layer_mut(layer)?;
        if !matches!(target.settings, LayerSettings::Sun(_)) {
            return Err(EditError::WrongKind {
                layer,
                expected: "sun",
            });
        }
        target.settings = LayerSettings::Sun(settings);
        Ok(())
    }

    pub fn set_galaxy_band_settings(
        &mut self,
        layer: LayerId,
        settings: GalaxyBandSettings,
    ) -> Result<(), EditError> {
        settings.validate()?;
        let target = self.layer_mut(layer)?;
        if !matches!(target.settings, LayerSettings::GalaxyBand(_)) {
            return Err(EditError::WrongKind {
                layer,
                expected: "galaxy band",
            });
        }
        target.settings = LayerSettings::GalaxyBand(settings);
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
            let checked = match &layer.settings {
                LayerSettings::StarField(settings) => settings.validate(),
                LayerSettings::Nebula(settings) => settings.validate(),
                LayerSettings::GalaxyBand(settings) => settings.validate(),
                LayerSettings::Sun(settings) => settings.validate(),
            };
            if let Err(EditError::InvalidSetting { field, value }) = checked {
                return Err(ProjectError::InvalidSetting {
                    layer: layer.id,
                    field,
                    value,
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

/// Bits per channel of exported PNG files. 16-bit avoids banding in dark gradients; 8-bit is for
/// engines and importers that mishandle 16-bit PNGs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngBitDepth {
    Sixteen,
    Eight,
}

/// File format of the floating-point HDRI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdriFormat {
    OpenExr,
    Radiance,
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

/// A small deterministic random color source (SplitMix64) for `Project::randomize_colors`.
struct ColorPicker(u64);

impl ColorPicker {
    /// The next number in 0.0 to 1.0.
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    }

    fn pastel(&mut self) -> [f32; 3] {
        let (hue, saturation) = (self.next(), self.next() * 0.3);
        hsv_to_rgb(hue, saturation, 1.0)
    }

    fn vivid(&mut self, hue: f32) -> [f32; 3] {
        let (saturation, value) = (0.55 + self.next() * 0.45, 0.8 + self.next() * 0.2);
        hsv_to_rgb(hue, saturation, value)
    }

    fn vivid_pair(&mut self) -> ([f32; 3], [f32; 3]) {
        let hue = self.next();
        let second = (hue + 0.15 + self.next() * 0.35).fract();
        (self.vivid(hue), self.vivid(second))
    }
}

/// Hue, saturation and value, each 0.0 to 1.0, as red, green and blue.
fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let sector = hue.fract() * 6.0;
    let (index, fraction) = (sector.floor() as u32 % 6, sector.fract());
    let low = value * (1.0 - saturation);
    let falling = value * (1.0 - saturation * fraction);
    let rising = value * (1.0 - saturation * (1.0 - fraction));
    match index {
        0 => [value, rising, low],
        1 => [falling, value, low],
        2 => [low, value, rising],
        3 => [low, falling, value],
        4 => [rising, low, value],
        _ => [value, low, falling],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragField {
    Opacity,
    StarField,
    Nebula,
    GalaxyBand,
    Sun,
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

    pub fn set_nebula_settings(
        &mut self,
        layer: LayerId,
        settings: NebulaSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::Nebula)), |project| {
            project.set_nebula_settings(layer, settings)
        })
    }

    pub fn set_sun_settings(
        &mut self,
        layer: LayerId,
        settings: SunSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::Sun)), |project| {
            project.set_sun_settings(layer, settings)
        })
    }

    pub fn set_galaxy_band_settings(
        &mut self,
        layer: LayerId,
        settings: GalaxyBandSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::GalaxyBand)), |project| {
            project.set_galaxy_band_settings(layer, settings)
        })
    }

    /// Sets the Seed and gives every Layer new random colors, as one undo step.
    pub fn randomize(&mut self, seed: Seed) {
        self.edit(None, |project| {
            project.set_seed(seed);
            project.randomize_colors(seed);
            Ok(())
        })
        .expect("randomizing cannot fail");
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
