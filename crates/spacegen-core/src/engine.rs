#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
}

/// Bits per channel of exported PNG files. 16-bit avoids banding in dark gradients; 8-bit is for
/// engines and importers that mishandle 16-bit PNGs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PngBitDepth {
    Sixteen,
    Eight,
}

/// File format of the floating-point HDRI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HdriFormat {
    OpenExr,
    Radiance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
