//! Background Game Detection Engine.
//!
//! Scans running user processes (native Linux and Proton/Wine) to automatically
//! switch equalizer presets and route game audio to the Game channel.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

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

        let config: GamesConfig = toml::from_str(&content).unwrap_or_else(|_| GamesConfig {
            games: HashMap::new(),
        });

        Self {
            games: config.games,
        }
    }

    /// Matches a process command line or executable name against the games database in deterministic order.
    pub fn match_executable(&self, comm: &str, cmdline: &str) -> Option<DetectedGame> {
        let clean_comm = comm.trim().trim_end_matches('\0');
        let lower_comm = clean_comm.to_ascii_lowercase();

        // Extract binary base name from cmdline if available
        let first_arg = cmdline.split('\0').next().unwrap_or(cmdline);
        let first_arg = first_arg.split_whitespace().next().unwrap_or(first_arg);
        let normalized_first = first_arg.replace('\\', "/");
        let base_name = normalized_first
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        let mut sorted_games: Vec<(&String, &GameEntry)> = self.games.iter().collect();
        sorted_games.sort_by_key(|(id, _)| *id);

        for (id, game) in sorted_games {
            for exe in &game.executables {
                let lower_exe = exe.to_ascii_lowercase();
                let lower_exe_no_ext = lower_exe.strip_suffix(".exe").unwrap_or(&lower_exe);

                // Direct match on comm (full or stripped .exe)
                let comm_match = lower_comm == lower_exe || lower_comm == lower_exe_no_ext;

                // Direct match on base_name
                let base_match = base_name == lower_exe || base_name == lower_exe_no_ext;

                // Proton/Wine match inside command line args:
                // Check if any arg (split by \0 or whitespace) has an exact file component matching exe
                let cmdline_match = cmdline.split(|c| c == '\0' || c == ' ').any(|arg| {
                    let trimmed = arg.trim();
                    if trimmed.is_empty() {
                        return false;
                    }
                    let lower_arg = trimmed.to_ascii_lowercase();
                    let norm = lower_arg.replace('\\', "/");
                    let file = norm.rsplit('/').next().unwrap_or(&norm);
                    file == lower_exe || (lower_exe.ends_with(".exe") && file == lower_exe_no_ext)
                });

                if comm_match || base_match || cmdline_match {
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

    /// Scans `/proc` deterministically for processes belonging strictly to the current user's UID.
    pub fn scan_processes(&self) -> Option<DetectedGame> {
        let my_uid = fs::metadata("/proc/self").map(|m| m.uid()).ok();
        let proc_dir = fs::read_dir("/proc").ok()?;

        let mut pids: Vec<u32> = Vec::new();
        for entry in proc_dir.flatten() {
            let file_name = entry.file_name();
            if let Some(pid_str) = file_name.to_str() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    // Filter to current user's UID
                    if let Some(uid) = my_uid {
                        if let Ok(meta) = entry.metadata() {
                            if meta.uid() != uid {
                                continue;
                            }
                        }
                    }
                    pids.push(pid);
                }
            }
        }

        // Sort PIDs ascending for deterministic scan order
        pids.sort_unstable();

        for pid in pids {
            let proc_path = PathBuf::from(format!("/proc/{pid}"));
            let comm_path = proc_path.join("comm");
            let cmdline_path = proc_path.join("cmdline");

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
        let matched = detector.match_executable(
            "wine64-preloader",
            "Z:\\SteamLibrary\\steamapps\\common\\Apex Legends\\r5apex.exe\0-anticheat",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "apex");
        assert_eq!(game.name, "Apex Legends");
        assert_eq!(game.preset_id, "game_apex");
    }

    #[test]
    fn test_match_rainbow_six() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "RainbowSix.exe",
            "Z:\\SteamLibrary\\steamapps\\common\\Rainbow Six Siege\\RainbowSix.exe",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "rainbowsix");
        assert_eq!(game.preset_id, "game_r6");
    }

    #[test]
    fn test_match_tarkov() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "EscapeFromTarkov.exe",
            "C:\\Battlestate Games\\BsgLauncher\\EscapeFromTarkov.exe",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "tarkov");
        assert_eq!(game.preset_id, "game_tarkov");
    }

    #[test]
    fn test_match_warzone() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "cod.exe",
            "C:\\Program Files (x86)\\Call of Duty\\cod.exe",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "warzone");
        assert_eq!(game.preset_id, "game_warzone");
    }

    #[test]
    fn test_match_the_finals() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "Discovery.exe",
            "C:\\Steam\\steamapps\\common\\The Finals\\Discovery.exe",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "finals");
        assert_eq!(game.name, "The Finals");
        assert_eq!(game.preset_id, "game_finals");
    }

    #[test]
    fn test_match_rust() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "RustClient.exe",
            "C:\\Steam\\steamapps\\common\\Rust\\RustClient.exe",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "rust");
        assert_eq!(game.name, "Rust");
        assert_eq!(game.preset_id, "game_rust");
    }

    #[test]
    fn test_match_dota2() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "dota2",
            "/home/user/.steam/steamapps/common/dota 2 beta/game/bin/linuxsteamrt64/dota2",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "dota2");
        assert_eq!(game.name, "Dota 2");
        assert_eq!(game.preset_id, "game_dota2");
    }

    #[test]
    fn test_match_league() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "League of Legends.exe",
            "C:\\Riot Games\\League of Legends\\League of Legends.exe",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "league");
        assert_eq!(game.preset_id, "game_league");
    }

    #[test]
    fn test_match_minecraft() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "javaw.exe",
            "C:\\Program Files\\Java\\bin\\javaw.exe -jar minecraft.jar",
        );
        assert!(matched.is_some());
        let game = matched.unwrap();
        assert_eq!(game.id, "minecraft");
        assert_eq!(game.preset_id, "game_minecraft");
    }

    #[test]
    fn test_no_match_for_browser() {
        let detector = GameDetector::new();
        let matched = detector.match_executable("firefox", "/usr/lib/firefox/firefox");
        assert!(matched.is_none());
    }

    #[test]
    fn test_no_match_for_ide_or_hash_containing_cod() {
        let detector = GameDetector::new();
        let matched = detector.match_executable(
            "antigravity",
            "/opt/antigravity/antigravity --enable-crash-reporter=0a46ae67-280f-4ca8-a13f-8c5f6fb --lang=en-US",
        );
        assert!(matched.is_none());

        let matched_vscode = detector.match_executable("code", "/usr/share/code/code");
        assert!(matched_vscode.is_none());
    }
}
