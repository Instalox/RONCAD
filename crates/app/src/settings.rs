use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

const APP_SETTINGS_SCHEMA_VERSION: u32 = 1;
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppSettings {
    pub recent_files: Vec<PathBuf>,
    pub last_project: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredAppSettings {
    schema_version: u32,
    recent_files: Vec<PathBuf>,
    last_project: Option<PathBuf>,
}

impl Default for StoredAppSettings {
    fn default() -> Self {
        Self {
            schema_version: APP_SETTINGS_SCHEMA_VERSION,
            recent_files: Vec::new(),
            last_project: None,
        }
    }
}

impl From<StoredAppSettings> for AppSettings {
    fn from(value: StoredAppSettings) -> Self {
        Self {
            recent_files: value.recent_files,
            last_project: value.last_project,
        }
    }
}

impl From<&AppSettings> for StoredAppSettings {
    fn from(value: &AppSettings) -> Self {
        Self {
            schema_version: APP_SETTINGS_SCHEMA_VERSION,
            recent_files: value.recent_files.clone(),
            last_project: value.last_project.clone(),
        }
    }
}

pub fn load_app_settings() -> Result<AppSettings> {
    load_app_settings_from_path(&settings_file_path()?)
}

pub fn save_app_settings(settings: &AppSettings) -> Result<()> {
    save_app_settings_to_path(settings, &settings_file_path()?)
}

fn load_app_settings_from_path(path: &Path) -> Result<AppSettings> {
    if !path.is_file() {
        return Ok(AppSettings::default());
    }

    let stored: StoredAppSettings = serde_json::from_str(
        &fs::read_to_string(path)
            .with_context(|| format!("failed to read app settings {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse app settings {}", path.display()))?;

    if stored.schema_version != APP_SETTINGS_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported app settings schema version {}; expected {}",
            stored.schema_version,
            APP_SETTINGS_SCHEMA_VERSION
        );
    }

    Ok(stored.into())
}

fn save_app_settings_to_path(settings: &AppSettings, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create settings directory {}", parent.display()))?;
    }

    fs::write(
        path,
        serde_json::to_string_pretty(&StoredAppSettings::from(settings))
            .context("failed to serialize app settings")?,
    )
    .with_context(|| format!("failed to write app settings {}", path.display()))?;
    Ok(())
}

fn settings_file_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("dev", "roncad", "RONCAD")
        .context("platform settings directory is unavailable")?;
    Ok(dirs.config_dir().join(SETTINGS_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::{load_app_settings_from_path, save_app_settings_to_path, AppSettings};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn settings_round_trip() {
        let path = temp_settings_path("round_trip");
        let settings = AppSettings {
            recent_files: vec![
                PathBuf::from("/tmp/a.roncad.json"),
                PathBuf::from("/tmp/b.roncad.json"),
            ],
            last_project: Some(PathBuf::from("/tmp/b.roncad.json")),
        };

        save_app_settings_to_path(&settings, &path).expect("save settings");
        let loaded = load_app_settings_from_path(&path).expect("load settings");
        assert_eq!(loaded, settings);

        let _ = std::fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    #[test]
    fn missing_file_returns_defaults() {
        let path = temp_settings_path("missing");
        let loaded = load_app_settings_from_path(&path).expect("load default settings");
        assert_eq!(loaded, AppSettings::default());
    }

    fn temp_settings_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("roncad_settings_{label}_{unique}"))
            .join("settings.json")
    }
}
