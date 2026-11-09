use partner_core::storage::{write_private_file, DEFAULT_BACKUPS};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub autolock_minutes: u64,
    pub autosave_seconds: u64,
    pub backups: usize,
    pub armor: bool,
    pub check_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            autolock_minutes: 10,
            autosave_seconds: 30,
            backups: DEFAULT_BACKUPS,
            armor: false,
            check_updates: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub recent: Vec<PathBuf>,
    #[serde(default)]
    pub settings: Settings,
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("io.github.Nariod", "Partner", "Partner")
            .map(|d| d.config_dir().join("config.json"))
    }

    pub fn load() -> Self {
        let Some(p) = Self::path() else {
            return Self::default();
        };
        std::fs::read(&p)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            if let Ok(b) = serde_json::to_vec_pretty(self) {
                let _ = write_private_file(&p, &b);
            }
        }
    }

    pub fn add_recent(&mut self, p: PathBuf) {
        self.recent.retain(|r| r != &p);
        self.recent.insert(0, p);
        self.recent.truncate(10);
        self.save();
    }
}
