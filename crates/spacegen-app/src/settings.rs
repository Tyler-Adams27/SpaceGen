//! What SpaceGen remembers between launches: recent Project files and the last export settings,
//! kept as JSON in the user's config directory. A missing or corrupt file just means defaults.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use spacegen_core::{EngineConvention, HdriFormat, ImageFormat, PngBitDepth};

/// How many recent files are kept.
pub const MAX_RECENT: usize = 8;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    pub name: String,
    pub folder: Option<PathBuf>,
    pub face_size: u32,
    pub format: ImageFormat,
    pub convention: EngineConvention,
    pub faces: bool,
    pub panorama: bool,
    pub hdri: bool,
    pub hdri_format: HdriFormat,
    pub png_bit_depth: PngBitDepth,
    pub jpeg_quality: u8,
}

impl Default for ExportSettings {
    fn default() -> Self {
        ExportSettings {
            name: "skybox".to_string(),
            folder: None,
            face_size: 2048,
            format: ImageFormat::Png,
            convention: EngineConvention::Unity,
            faces: true,
            panorama: true,
            hdri: true,
            hdri_format: HdriFormat::OpenExr,
            png_bit_depth: PngBitDepth::Sixteen,
            jpeg_quality: 95,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Most recent first.
    pub recent: Vec<PathBuf>,
    pub export: ExportSettings,
}

impl Settings {
    /// Reads the settings file; a missing, unreadable or corrupt file gives the defaults.
    pub fn load(path: &Path) -> Settings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_json())
    }

    /// Puts `path` at the front of the recent files, without duplicates, keeping `MAX_RECENT`.
    pub fn remember_recent(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(MAX_RECENT);
    }

    pub fn forget_recent(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
    }
}

/// Where the settings file lives: the platform's per-user config directory, if it can be found.
pub fn settings_path() -> Option<PathBuf> {
    let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(windows) {
        env("APPDATA")?
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Application Support")
    } else {
        env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config")))?
    };
    Some(base.join("SpaceGen").join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("spacegen-settings-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("nested").join("settings.json")
    }

    #[test]
    fn saved_settings_come_back_on_the_next_load() {
        let path = temp_file("roundtrip");
        let mut settings = Settings::default();
        settings.export.name = "nebula".to_string();
        settings.export.folder = Some(PathBuf::from("exports"));
        settings.export.face_size = 4096;
        settings.export.format = ImageFormat::Jpeg;
        settings.export.convention = EngineConvention::Godot;
        settings.remember_recent(Path::new("a.spacegen"));

        settings.save(&path).unwrap();
        let loaded = Settings::load(&path);

        assert_eq!(loaded.export.name, "nebula");
        assert_eq!(loaded.export.folder, Some(PathBuf::from("exports")));
        assert_eq!(loaded.export.face_size, 4096);
        assert_eq!(loaded.export.format, ImageFormat::Jpeg);
        assert_eq!(loaded.export.convention, EngineConvention::Godot);
        assert_eq!(loaded.recent, [PathBuf::from("a.spacegen")]);
    }

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let loaded = Settings::load(&temp_file("missing"));

        assert_eq!(loaded.export.name, "skybox");
        assert!(loaded.recent.is_empty());
    }

    #[test]
    fn a_corrupt_file_is_ignored_not_fatal() {
        let path = temp_file("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{ this is not json").unwrap();

        let loaded = Settings::load(&path);

        assert_eq!(loaded.export.face_size, 2048);
    }

    #[test]
    fn a_file_missing_some_fields_keeps_the_defaults_for_those() {
        let path = temp_file("partial");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"export":{"name":"only-this"}}"#).unwrap();

        let loaded = Settings::load(&path);

        assert_eq!(loaded.export.name, "only-this");
        assert_eq!(loaded.export.face_size, 2048);
    }

    #[test]
    fn recent_files_are_newest_first_without_duplicates_and_capped() {
        let mut settings = Settings::default();
        for i in 0..12 {
            settings.remember_recent(Path::new(&format!("{i}.spacegen")));
        }
        settings.remember_recent(Path::new("5.spacegen"));

        assert_eq!(settings.recent.len(), MAX_RECENT);
        assert_eq!(settings.recent[0], PathBuf::from("5.spacegen"));
        assert_eq!(settings.recent[1], PathBuf::from("11.spacegen"));
        assert_eq!(
            settings
                .recent
                .iter()
                .filter(|p| **p == Path::new("5.spacegen"))
                .count(),
            1
        );
    }

    #[test]
    fn a_forgotten_recent_file_is_removed() {
        let mut settings = Settings::default();
        settings.remember_recent(Path::new("a.spacegen"));
        settings.remember_recent(Path::new("b.spacegen"));

        settings.forget_recent(Path::new("a.spacegen"));

        assert_eq!(settings.recent, [PathBuf::from("b.spacegen")]);
    }
}
