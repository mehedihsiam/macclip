//! User configuration (`config.json`) and well-known paths.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "MacClip";
pub const MAX_ITEMS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub poll_interval_ms: u64,
    pub max_text_bytes: usize,
    pub max_image_bytes: usize,
    pub excluded_bundle_ids: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            poll_interval_ms: 300,
            max_text_bytes: 1024 * 1024,
            max_image_bytes: 25 * 1024 * 1024,
            excluded_bundle_ids: vec![
                "com.1password.1password".into(),
                "com.agilebits.onepassword7".into(),
                "com.bitwarden.desktop".into(),
                "com.apple.keychainaccess".into(),
                "com.dashlane.dashlanephonefinal".into(),
            ],
        }
    }
}

impl Config {
    /// Load from `path`; a missing or invalid file yields defaults, and a
    /// missing file is created so users can discover the options.
    pub fn load_or_create(path: &Path) -> Config {
        match std::fs::read_to_string(path) {
            Ok(s) => match serde_json::from_str::<Config>(&s) {
                Ok(mut c) => {
                    c.poll_interval_ms = c.poll_interval_ms.clamp(100, 5000);
                    c
                }
                Err(e) => {
                    log::warn!("invalid config.json ({e}); using defaults");
                    Config::default()
                }
            },
            Err(_) => {
                let c = Config::default();
                if let Ok(s) = serde_json::to_string_pretty(&c) {
                    let _ = std::fs::write(path, s);
                    crate::store::restrict_file(path);
                }
                c
            }
        }
    }
}

/// Data directory: `~/Library/Application Support/MacClip`.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

/// Log directory: `~/Library/Logs/MacClip`.
pub fn log_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Logs")
        .join(APP_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing_and_file_created() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        let c = Config::load_or_create(&p);
        assert_eq!(c, Config::default());
        assert!(p.exists());
    }

    #[test]
    fn partial_override_keeps_other_defaults() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        std::fs::write(&p, r#"{"poll_interval_ms": 500}"#).unwrap();
        let c = Config::load_or_create(&p);
        assert_eq!(c.poll_interval_ms, 500);
        assert_eq!(c.max_text_bytes, Config::default().max_text_bytes);
    }

    #[test]
    fn invalid_file_falls_back() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        std::fs::write(&p, "not json").unwrap();
        assert_eq!(Config::load_or_create(&p), Config::default());
    }
}
