use crate::banner::SlackConfig;
use crate::watcher::WatcherConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application-wide configuration, persisted as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Slack integration settings.
    pub slack: SlackConfig,
    /// File/stdin watcher settings.
    pub watcher: WatcherConfig,
    /// Active hours range (start_hour, end_hour) in 24h format.
    pub active_hours: (u32, u32),
    /// Number of cat events to schedule per day.
    pub events_per_day: u32,
    /// Base directory for persisted data.
    pub data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cat-in-lattice");

        Self {
            slack: SlackConfig::default(),
            watcher: WatcherConfig::default(),
            active_hours: (8, 23),
            events_per_day: 20,
            data_dir,
        }
    }
}

impl Config {
    /// Path to the config JSON file.
    pub fn config_path(data_dir: &std::path::Path) -> PathBuf {
        data_dir.join("config.json")
    }

    /// Ensure the data directory exists.
    pub fn ensure_data_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }

    /// Load config from the given data directory, or return defaults.
    pub fn load(data_dir: Option<&std::path::Path>) -> Self {
        let default_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cat-in-lattice");
        let dir = data_dir.unwrap_or(&default_dir);
        let path = Self::config_path(dir);

        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<Config>(&contents) {
                    Ok(mut cfg) => {
                        cfg.data_dir = dir.to_path_buf();
                        return cfg;
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to parse config: {e}. Using defaults.");
                    }
                },
                Err(e) => {
                    eprintln!("Warning: failed to read config: {e}. Using defaults.");
                }
            }
        }

        Config {
            data_dir: dir.to_path_buf(),
            ..Config::default()
        }
    }

    /// Save config to the data directory.
    pub fn save(&self) -> std::io::Result<()> {
        self.ensure_data_dir()?;
        let path = Self::config_path(&self.data_dir);
        let json = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.active_hours, (8, 23));
        assert_eq!(cfg.events_per_day, 20);
        assert!(cfg.data_dir.ends_with(".cat-in-lattice"));
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let tmp = std::env::temp_dir().join("cil-test-config-missing");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let cfg = Config::load(Some(&tmp));
        assert_eq!(cfg.active_hours, (8, 23));
        assert_eq!(cfg.events_per_day, 20);
        assert_eq!(cfg.data_dir, tmp);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join("cil-test-config-roundtrip");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut cfg = Config::default();
        cfg.data_dir = tmp.clone();
        cfg.events_per_day = 42;
        cfg.active_hours = (9, 17);
        cfg.save().unwrap();

        let loaded = Config::load(Some(&tmp));
        assert_eq!(loaded.events_per_day, 42);
        assert_eq!(loaded.active_hours, (9, 17));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_corrupt_file_returns_defaults() {
        let tmp = std::env::temp_dir().join("cil-test-config-corrupt");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let path = Config::config_path(&tmp);
        fs::write(&path, "not valid json {{{{").unwrap();

        let cfg = Config::load(Some(&tmp));
        // Should fall back to defaults
        assert_eq!(cfg.active_hours, (8, 23));
        assert_eq!(cfg.data_dir, tmp);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_config_path() {
        let dir = PathBuf::from("/some/dir");
        let path = Config::config_path(&dir);
        assert_eq!(path, PathBuf::from("/some/dir/config.json"));
    }
}
