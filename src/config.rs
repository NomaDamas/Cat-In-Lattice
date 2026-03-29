use crate::banner::SlackConfig;
use crate::watcher::WatcherConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    /// Apply runtime-only overrides from a dotenv-style env file.
    /// These values are not meant to be persisted back into config.json.
    pub fn apply_env_file(&mut self, env_path: &std::path::Path) -> std::io::Result<()> {
        let contents = std::fs::read_to_string(env_path)?;
        let vars = Self::parse_env_contents(&contents)?;
        self.apply_env_vars(&vars);
        Ok(())
    }

    /// Save config to the data directory.
    pub fn save(&self) -> std::io::Result<()> {
        self.ensure_data_dir()?;
        let path = Self::config_path(&self.data_dir);
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    fn apply_env_vars(&mut self, vars: &HashMap<String, String>) {
        if let Some(value) = vars.get("CAT_IN_LATTICE_SLACK_WEBHOOK_URL") {
            self.slack.webhook_url = normalize_env_value(value);
        }
        if let Some(value) = vars.get("CAT_IN_LATTICE_SLACK_TOKEN") {
            self.slack.token = normalize_env_value(value);
        }
        if let Some(value) = vars.get("CAT_IN_LATTICE_SLACK_CHANNEL") {
            self.slack.channel = normalize_env_value(value);
        }
    }

    fn parse_env_contents(contents: &str) -> std::io::Result<HashMap<String, String>> {
        let mut vars = HashMap::new();

        for (index, raw_line) in contents.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let line = line.strip_prefix("export ").unwrap_or(line).trim();
            let Some((key, value)) = line.split_once('=') else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid env line {}: {raw_line}", index + 1),
                ));
            };

            let key = key.trim();
            if key.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid env key on line {}", index + 1),
                ));
            }

            let value = strip_wrapping_quotes(value.trim());
            vars.insert(key.to_string(), value.to_string());
        }

        Ok(vars)
    }
}

fn normalize_env_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn strip_wrapping_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let quoted_with_double = value.starts_with('"') && value.ends_with('"');
        let quoted_with_single = value.starts_with('\'') && value.ends_with('\'');
        if quoted_with_double || quoted_with_single {
            return &value[1..value.len() - 1];
        }
    }

    value
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

        let cfg = Config {
            data_dir: tmp.clone(),
            events_per_day: 42,
            active_hours: (9, 17),
            ..Config::default()
        };
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

    #[test]
    fn test_apply_env_file_overrides_slack_settings() {
        let tmp = std::env::temp_dir().join("cil-test-env-override");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let env_path = tmp.join(".env");
        fs::write(
            &env_path,
            r#"
CAT_IN_LATTICE_SLACK_WEBHOOK_URL="https://hooks.slack.com/services/test"
CAT_IN_LATTICE_SLACK_TOKEN=xoxb-test
CAT_IN_LATTICE_SLACK_CHANNEL=C123
"#,
        )
        .unwrap();

        let mut cfg = Config::load(Some(&tmp));
        cfg.apply_env_file(&env_path).unwrap();

        assert_eq!(
            cfg.slack.webhook_url.as_deref(),
            Some("https://hooks.slack.com/services/test")
        );
        assert_eq!(cfg.slack.token.as_deref(), Some("xoxb-test"));
        assert_eq!(cfg.slack.channel.as_deref(), Some("C123"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_env_file_can_clear_slack_setting() {
        let tmp = std::env::temp_dir().join("cil-test-env-clear");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let env_path = tmp.join(".env");
        fs::write(&env_path, "CAT_IN_LATTICE_SLACK_WEBHOOK_URL=\n").unwrap();

        let mut cfg = Config::default();
        cfg.slack.webhook_url = Some("https://hooks.slack.com/services/test".into());
        cfg.apply_env_file(&env_path).unwrap();

        assert_eq!(cfg.slack.webhook_url, None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_apply_env_file_rejects_invalid_line() {
        let tmp = std::env::temp_dir().join("cil-test-env-invalid");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let env_path = tmp.join(".env");
        fs::write(&env_path, "NOT VALID\n").unwrap();

        let mut cfg = Config::default();
        let err = cfg.apply_env_file(&env_path).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);

        let _ = fs::remove_dir_all(&tmp);
    }
}
