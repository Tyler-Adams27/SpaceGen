use serde::{Deserialize, Serialize};

use crate::error::{EditError, ProjectError};
use crate::layer::{BlendMode, Layer, LayerKind, LayerSettings};
use crate::seed::{LayerId, Seed};
use crate::settings::{GalaxyBandSettings, NebulaSettings, StarFieldSettings, SunSettings};

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    format_version: u32,
    seed: Seed,
    next_layer_id: u64,
    layers: Vec<Layer>,
}

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
            variation: 0,
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
            layer.recolor(seed);
        }
    }

    /// Gives one Layer a new look: a fresh variation, so its stars, clouds or band structure
    /// change, and new colors, both derived from `seed`. Every other Layer, and the Project
    /// Seed, stay exactly as they are.
    pub fn reroll_layer(&mut self, layer: LayerId, seed: Seed) -> Result<(), EditError> {
        let target = self.layer_mut(layer)?;
        target.variation = seed.0.max(1);
        target.recolor(seed);
        Ok(())
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
