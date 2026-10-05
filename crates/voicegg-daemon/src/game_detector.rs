//! Background Game Detection Engine.
//!
//! Scans running user processes (native Linux and Proton/Wine) to automatically
//! switch equalizer presets and route game audio to the Game channel.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Embedded fallback games database in case configuration files are missing.
const EMBEDDED_GAMES_TOML: &str = include_str!("../../../presets/games.toml");

/// Metadata for a registered game.
#[derive(Debug, Clone, Deserialize)]
pub struct GameEntry {
    /// Human-friendly display name (e.g. "Counter-Strike 2").
    pub name: String,
    /// Preset ID to apply (e.g. "game_cs2").
    pub preset: String,
    /// Executable binary names to match against (e.g. ["cs2"]).
    pub executables: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GamesConfig {
    games: HashMap<String, GameEntry>,
}

/// Information about a currently detected active game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedGame {
    /// Unique game identifier key (e.g. "cs2", "apex").
    pub id: String,
    /// Display name of the game.
    pub name: String,
    /// Preset identifier to apply to the Game channel.
    pub preset_id: String,
    /// Name of the binary that triggered the match.
    pub matched_binary: String,
}

/// Scans active user processes to detect gaming applications.
pub struct GameDetector {
    games: HashMap<String, GameEntry>,
}

impl Default for GameDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl GameDetector {
    /// Initializes game detector with user configuration or embedded defaults.
    #[must_use]
    pub fn new() -> Self {
        let user_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("voicegg")
            .join("games.toml");

        let content = if user_path.exists() {
            fs::read_to_string(&user_path).unwrap_or_else(|_| EMBEDDED_GAMES_TOML.to_string())
        } else {
            EMBEDDED_GAMES_TOML.to_string()
        };

        let config: GamesConfig =
            toml::from_str(&content).unwrap_or_else(|_| GamesConfig { games: HashMap::new() });

        Self {
            games: config.games,
        }
    }

    /// Matches a process command line or executable name against the games database.
    pub fn match_executable(&self, comm: &str, cmdline: &str) -> Option<DetectedGame> {
        let clean_comm = comm.trim().trim_end_matches('\0');
        let lower_comm = clean_comm.to_ascii_lowercase();

        // Extract binary base name from cmdline if available
        let first_arg = cmdline.split('\0').next().unwrap_or(cmdline);
        let base_name = Path::new(first_arg)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        for (id, game) in &self.games {
            for exe in &game.executables {
                let lower_exe = exe.to_ascii_lowercase();
                if lower_comm == lower_exe
                    || base_name == lower_exe
                    || cmdline.to_ascii_lowercase().contains(&lower_exe)
                {
                    return Some(DetectedGame {
                        id: id.clone(),
                        name: game.name.clone(),
                        preset_id: game.preset.clone(),
                        matched_binary: if !clean_comm.is_empty() {
                            clean_comm.to_string()
                        } else {
                            exe.clone()
                        },
                    });
                }
            }
        }
        None
    }

    /// Scans `/proc` for processes belonging to the current user.
    pub fn scan_processes(&self) -> Option<DetectedGame> {
        let proc_dir = fs::read_dir("/proc").ok()?;

        for entry in proc_dir.flatten() {
            let file_name = entry.file_name();
            let pid_str = file_name.to_str()?;

            // Only examine numeric PID directories
            if !pid_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let path = entry.path();
            let comm_path = path.join("comm");
            let cmdline_path = path.join("cmdline");

            let comm = fs::read_to_string(comm_path).unwrap_or_default();
            let cmdline = fs::read_to_string(cmdline_path).unwrap_or_default();

            if let Some(detected) = self.match_executable(&comm, &cmdline) {
                return Some(detected);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_native_cs2() {
        let detector = GameDetector::new();
        let matched = detector.match_executable("cs2", "/home/user/.steam/steamapps/common/Counter-Strike Global Offensive/game/bin/linuxsteamrt64/cs2");
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "cs2");
        assert_eq!(game.name, "Counter-Strike 2");
        assert_eq!(game.preset_id, "game_cs2");
    }

    #[test]
    fn test_match_proton_apex() {
        let detector = GameDetector::new();
        let matched = detector.match_executable("wine64-preloader", "Z:\\SteamLibrary\\steamapps\\common\\Apex Legends\\r5apex.exe\0-anticheat");
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "apex");
        assert_eq!(game.name, "Apex Legends");
        assert_eq!(game.preset_id, "game_apex");
    }

    #[test]
    fn test_no_match_for_browser() {
        let detector = GameDetector::new();
        let matched = detector.match_executable("firefox", "/usr/lib/firefox/firefox");
        assert!(matched.is_none());
    }
}
