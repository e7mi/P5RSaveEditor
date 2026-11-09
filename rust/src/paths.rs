//! Filesystem discovery, settings and small Windows helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "P5RSaveEditor";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SLOT_NAMES: [&str; 16] = [
    "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06", "DATA07", "DATA08", "DATA09",
    "DATA10", "DATA11", "DATA12", "DATA13", "DATA14", "DATA15", "DATA16",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub save_dir: String,
    pub auto_backup: bool,
    pub show_reserve: bool,
    pub allow_save_while_running: bool,
    pub advanced: bool,
    pub last_slot: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            save_dir: String::new(),
            auto_backup: true,
            show_reserve: false,
            allow_save_while_running: false,
            advanced: false,
            last_slot: String::new(),
        }
    }
}

pub fn local_app_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
                .join("AppData")
                .join("Local")
        });
    let path = base.join(APP_NAME);
    let _ = fs::create_dir_all(&path);
    path
}

pub fn settings_path() -> PathBuf {
    local_app_dir().join("settings.json")
}

pub fn load_settings() -> Settings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(settings: &Settings) {
    let path = settings_path();
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, text);
    }
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(
            PathBuf::from(appdata)
                .join("SEGA")
                .join("P5R")
                .join("Steam"),
        );
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        let home = PathBuf::from(profile);
        for docs in [
            home.join("Documents"),
            home.join("OneDrive").join("Documents"),
            home.join("OneDrive").join("ドキュメント"),
        ] {
            roots.push(docs.join("Atlus").join("P5R").join("Steam"));
            roots.push(docs.join("Atlus").join("P5R"));
        }
    }
    roots
}

pub fn clean_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

pub fn display_path(path: &Path) -> String {
    clean_path(path).display().to_string()
}

pub fn candidate_save_dirs() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut add = |path: PathBuf| {
        if !path.is_dir() {
            return;
        }
        let resolved = fs::canonicalize(&path)
            .map(|path| clean_path(&path))
            .unwrap_or_else(|_| clean_path(&path));
        if !found.contains(&resolved) {
            found.push(resolved);
        }
    };
    if let Some(env) = std::env::var_os("P5R_SAVE_DIR") {
        add(PathBuf::from(env));
    }
    for root in steam_roots() {
        add(root.join("savedata"));
        if root.is_dir() {
            if let Ok(children) = fs::read_dir(&root) {
                for child in children.flatten() {
                    if child.path().is_dir() {
                        add(child.path().join("savedata"));
                        add(child.path());
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter(|path| path.join("DATA01").is_dir() || path.join("SYSTEM").is_dir())
        .collect()
}

pub fn default_save_dir() -> String {
    candidate_save_dirs()
        .first()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[derive(Clone, Debug)]
pub struct SlotFile {
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
}

pub fn slot_files(save_dir: &Path) -> Vec<SlotFile> {
    SLOT_NAMES
        .iter()
        .map(|name| {
            let path = save_dir.join(name).join("DATA.DAT");
            SlotFile {
                name: (*name).to_string(),
                exists: path.is_file(),
                path,
            }
        })
        .collect()
}

pub fn is_game_running() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let output = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq P5R.exe", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        if let Ok(output) = output {
            return String::from_utf8_lossy(&output.stdout).contains("P5R.exe");
        }
        false
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn open_in_explorer(path: &Path) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer").arg(path).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

pub fn human_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    }
}
