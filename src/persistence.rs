use crate::cat::state::CatState;
use crate::games::GameRecord;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const STATE_FILE: &str = "cat_state.json";
const GAME_RECORDS_FILE: &str = "game_records.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredGameRecords {
    items: Vec<GameRecord>,
}

/// Manages save/load of cat state to disk.
pub struct Persistence {
    path: PathBuf,
}

impl Persistence {
    /// Create a new Persistence pointing at the given data directory.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(STATE_FILE),
        }
    }

    /// Load cat state from disk, or return a fresh default if the file is
    /// missing or corrupt.
    pub fn load(&self) -> CatState {
        if !self.path.exists() {
            return CatState::new();
        }
        match std::fs::read_to_string(&self.path) {
            Ok(json) => CatState::from_json(&json).unwrap_or_else(|e| {
                eprintln!("Warning: corrupt cat state file, starting fresh: {e}");
                CatState::new()
            }),
            Err(e) => {
                eprintln!("Warning: could not read cat state file: {e}");
                CatState::new()
            }
        }
    }

    /// Save cat state to disk. Creates the parent directory if needed.
    pub fn save(&self, state: &CatState) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = state.to_json().map_err(std::io::Error::other)?;
        std::fs::write(&self.path, json)
    }

    pub fn load_game_records(&self) -> Vec<GameRecord> {
        let path = self.records_path();
        if !path.exists() {
            return Vec::new();
        }

        match std::fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str::<StoredGameRecords>(&json)
                .map(|records| records.items)
                .unwrap_or_else(|e| {
                    eprintln!("Warning: corrupt game records file, starting fresh: {e}");
                    Vec::new()
                }),
            Err(e) => {
                eprintln!("Warning: could not read game records file: {e}");
                Vec::new()
            }
        }
    }

    pub fn save_game_records(&self, records: &[GameRecord]) -> std::io::Result<()> {
        let path = self.records_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&StoredGameRecords {
            items: records.to_vec(),
        })
        .map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    fn records_path(&self) -> PathBuf {
        self.path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(GAME_RECORDS_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::state::{Accessory, Mood};

    #[test]
    fn test_load_missing_file_returns_fresh_state() {
        let tmp = std::env::temp_dir().join("cil-test-persist-missing");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let p = Persistence::new(&tmp);
        let state = p.load();
        assert_eq!(state.mood, Mood::Neutral);
        assert!((state.affinity - 50.0).abs() < f64::EPSILON);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let tmp = std::env::temp_dir().join("cil-test-persist-roundtrip");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let p = Persistence::new(&tmp);

        let mut state = CatState::new();
        state.affinity = 88.5;
        state.hunger = 12.0;
        state.add_accessory(Accessory::Hat);
        state.total_pets = 42;
        p.save(&state).unwrap();

        let loaded = p.load();
        assert!((loaded.affinity - 88.5).abs() < f64::EPSILON);
        assert!((loaded.hunger - 12.0).abs() < f64::EPSILON);
        assert_eq!(loaded.total_pets, 42);
        assert!(loaded.accessories.contains(&Accessory::Hat));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_corrupt_file_returns_fresh_state() {
        let tmp = std::env::temp_dir().join("cil-test-persist-corrupt");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join(STATE_FILE), "{{not json}}").unwrap();

        let p = Persistence::new(&tmp);
        let state = p.load();
        assert_eq!(state.mood, Mood::Neutral);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_save_creates_parent_directory() {
        let tmp = std::env::temp_dir().join("cil-test-persist-mkdir/nested/deep");
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("cil-test-persist-mkdir"));

        let p = Persistence::new(&tmp);
        let state = CatState::new();
        p.save(&state).unwrap();

        assert!(tmp.join(STATE_FILE).exists());

        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("cil-test-persist-mkdir"));
    }

    #[test]
    fn test_save_load_game_records_roundtrip() {
        let tmp = std::env::temp_dir().join("cil-test-game-records-roundtrip");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let p = Persistence::new(&tmp);
        let records = vec![
            GameRecord::new("Breakout", 120, "Win"),
            GameRecord::new("Snake", 8, "Game Over"),
        ];

        p.save_game_records(&records).unwrap();
        let loaded = p.load_game_records();
        assert_eq!(loaded, records);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
