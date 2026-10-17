use serde::{Deserialize, Serialize};

use crate::error::EditError;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StarFieldSettings {
    pub density: f32,
    pub brightness: f32,
    pub size: f32,
    pub color_variation: f32,
    pub tint: [f32; 3],
}

impl StarFieldSettings {
    pub(crate) fn validate(&self) -> Result<(), EditError> {
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
    pub(crate) fn validate(&self) -> Result<(), EditError> {
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
    pub(crate) fn validate(&self) -> Result<(), EditError> {
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
    pub(crate) fn validate(&self) -> Result<(), EditError> {
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
