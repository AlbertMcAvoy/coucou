// Preferences, stored as plain JSON in settings.json under platform::config_dir().
// No secret ever lands here — API keys live in the OS keychain (see secrets.rs).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub sound_enabled: bool,
    pub sound_volume: f64,
    pub auto_close_interval: f64,
    pub absence_interval: f64,
    pub active_integrations: Vec<String>,
    /// "primary" = the main display, "cursor" = whichever display the mouse is on.
    pub screen: String,
    pub autostart: bool,
    /// Where along the top edge the island rests, 0..=1: 0 flush left, 0.5
    /// centred, 1 flush right. A fraction, so a drag lands anywhere while the
    /// presets in the settings still hit the edges exactly.
    #[serde(default = "default_notch_position")]
    pub notch_position: f64,
    pub hooks_installed: bool,
    /// Claude model used by the chat. Changeable in the settings window.
    /// Defaulted explicitly so a settings.json written by an older build still loads.
    #[serde(default = "default_model")]
    pub model: String,
    /// The compact island stays on screen instead of tucking away after a minute.
    #[serde(default = "default_true")]
    pub keep_visible: bool,
    /// WSL distros whose Claude Code has Coucou's hooks, as last written or seen.
    /// Owned by Rust: the windows never send it (see `save_settings`).
    #[serde(default)]
    pub wsl_hooks: Vec<String>,
    /// The first-launch offer to set up WSL has been made. Shown once, ever.
    #[serde(default)]
    pub wsl_prompted: bool,
    /// Where Mochi's chat goes: "api" (the API key), "windows" (Claude Code on
    /// Windows) or "wsl:<distro>" (Claude Code inside that distro).
    #[serde(default = "default_backend")]
    pub chat_backend: String,
    /// The session Mochi's chat is in, kept across restarts. Owned by Rust.
    #[serde(default)]
    pub mochi_session: Option<crate::local_claude::ActiveSession>,
    /// Who lives in the island: "mochi" (drawn here) or "blobatar" (the
    /// blobatar library's character, a prototype).
    #[serde(default = "default_character")]
    pub character: String,
    /// The string blobatar generates its character from; empty for the classic
    /// round white one.
    #[serde(default)]
    pub character_seed: String,
}

fn default_true() -> bool {
    true
}

fn default_character() -> String {
    "mochi".into()
}

fn default_backend() -> String {
    "api".into()
}

fn default_notch_position() -> f64 {
    0.5
}

fn default_model() -> String {
    crate::claude::DEFAULT_MODEL.to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            sound_volume: 0.12,
            auto_close_interval: 15.0,
            absence_interval: 180.0,
            active_integrations: vec![
                "integration_resend".into(),
                "integration_n8n".into(),
                "integration_vercel".into(),
                "integration_github".into(),
            ],
            screen: "primary".into(),
            autostart: false,
            notch_position: default_notch_position(),
            hooks_installed: false,
            model: default_model(),
            keep_visible: true,
            wsl_hooks: Vec::new(),
            wsl_prompted: false,
            chat_backend: default_backend(),
            mochi_session: None,
            character: default_character(),
            character_seed: String::new(),
        }
    }
}

pub use crate::platform::{config_dir, local_dir};

pub fn hook_exe_path() -> PathBuf {
    local_dir().join("bin").join(crate::platform::HOOK_EXE)
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> Settings {
    match std::fs::read(settings_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let dir = config_dir();
    crate::platform::ensure_private_dir(&dir)?;
    let json = serde_json::to_vec_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(settings_path(), json)
}
