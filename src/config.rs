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

        let mut cfg = Config::default();
        cfg.data_dir = dir.to_path_buf();
        cfg
    }

    /// Save config to the data directory.
    pub fn save(&self) -> std::io::Result<()> {
        self.ensure_data_dir()?;
        let path = Self::config_path(&self.data_dir);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }
}
