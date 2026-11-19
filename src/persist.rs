//! UI state persisted to `<config_dir>/podlogs/state.json` (replaces Zustand `persist`).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    Light,
    #[default]
    Dark,
    System,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct PersistedState {
    pub theme: ThemePref,
    pub sidebar_collapsed: bool,
    pub refresh_interval_ms: u32,
    pub context: String,
    pub namespace: String,
    pub deployment: Option<String>,
    pub last_namespace_by_cluster: BTreeMap<String, String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            theme: ThemePref::Dark,
            sidebar_collapsed: false,
            refresh_interval_ms: 5000,
            context: String::new(),
            namespace: "default".into(),
            deployment: None,
            last_namespace_by_cluster: BTreeMap::new(),
        }
    }
}

/// Missing or corrupt file → defaults.
pub fn load(path: &Path) -> PersistedState {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Atomic write: temp file + rename, creating parent dirs.
pub fn save(path: &Path, s: &PersistedState) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(&tmp, path)
}

pub fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("podlogs").join("state.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_save_fallbacks() {
        let dir = std::env::temp_dir().join(format!("podlogs-test-{}", std::process::id()));
        assert_eq!(load(&dir.join("missing.json")), PersistedState::default());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.json"), "{nope").unwrap();
        assert_eq!(load(&dir.join("bad.json")), PersistedState::default());
        let s = PersistedState { context: "prod".into(), ..Default::default() };
        save(&dir.join("a/state.json"), &s).unwrap();
        assert_eq!(load(&dir.join("a/state.json")), s);
        std::fs::write(dir.join("partial.json"), r#"{"context":"x"}"#).unwrap();
        assert_eq!(load(&dir.join("partial.json")).refresh_interval_ms, 5000);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
