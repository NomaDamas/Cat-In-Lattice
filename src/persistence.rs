use crate::cat::state::CatState;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = "cat_state.json";

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
        let json = state
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&self.path, json)
    }
}
