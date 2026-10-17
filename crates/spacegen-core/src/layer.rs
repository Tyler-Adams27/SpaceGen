use serde::{Deserialize, Serialize};

use crate::color::ColorPicker;
use crate::seed::{LayerId, Seed};
use crate::settings::{GalaxyBandSettings, NebulaSettings, StarFieldSettings, SunSettings};

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
    pub(crate) id: LayerId,
    pub(crate) settings: LayerSettings,
    pub(crate) blend: BlendMode,
    pub(crate) opacity: f32,
    pub(crate) visible: bool,
    /// Mixed into the Layer's seed so it can get a new look on its own; 0 means unchanged.
    #[serde(default)]
    pub(crate) variation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum LayerSettings {
    StarField(StarFieldSettings),
    Nebula(NebulaSettings),
    GalaxyBand(GalaxyBandSettings),
    Sun(SunSettings),
}

impl LayerSettings {
    pub(crate) fn default_for(kind: LayerKind) -> LayerSettings {
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

    pub(crate) fn recolor(&mut self, seed: Seed) {
        let mut picker = ColorPicker::new(seed.layer_seed(self.id).0);
        match &mut self.settings {
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

    pub fn variation(&self) -> u64 {
        self.variation
    }

    /// The Seed this Layer draws its randomness from: the Project Seed and the Layer id, with the
    /// Layer's variation mixed in once it has been re-rolled.
    pub fn seed_under(&self, seed: Seed) -> Seed {
        let base = seed.layer_seed(self.id);
        if self.variation == 0 {
            return base;
        }
        let mut z = base.0 ^ self.variation.wrapping_mul(0x9E3779B97F4A7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        Seed(z ^ (z >> 31))
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
