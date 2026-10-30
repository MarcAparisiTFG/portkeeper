//! User settings, persisted as JSON in the app data folder.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    /// Open the widget when Portkeeper starts (off is handy with launch at login).
    pub show_on_start: bool,
    /// Also list app-internal ports (Spotify, editor helpers…).
    pub show_all: bool,
    /// Locked projects: no stop / remove from Portkeeper. Keys look like
    /// "dir:~/path/to/project" or "d:compose-project".
    pub protected: Vec<String>,
    /// "glass" (system material), "dark" or "light" (opaque).
    pub theme: String,
    /// UI language code ("en", "es", …); empty = follow the system.
    pub lang: String,
    /// Look for new versions on GitHub (the only request that leaves the machine).
    pub check_updates: bool,
    /// Close the widget when it loses focus, like a menu bar menu.
    pub close_on_blur: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            show_on_start: true,
            show_all: false,
            protected: Vec::new(),
            theme: "glass".into(),
            lang: String::new(),
            check_updates: true,
            close_on_blur: false,
        }
    }
}

pub struct Store {
    pub value: Settings,
    file: PathBuf,
}

impl Store {
    pub fn load(file: PathBuf) -> Self {
        let value = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Store { value, file }
    }

    pub fn save(&self) {
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&self.value) {
            let _ = std::fs::write(&self.file, json);
        }
    }

    pub fn is_protected(&self, key: &str) -> bool {
        self.value.protected.iter().any(|k| k == key)
    }

    pub fn toggle_protected(&mut self, key: &str) {
        if self.is_protected(key) {
            self.value.protected.retain(|k| k != key);
        } else {
            self.value.protected.push(key.to_string());
        }
        self.save();
    }
}

/// Project key of a process, the same one the UI uses for its row.
pub fn key_for_cwd(cwd: &str) -> String {
    format!("dir:{cwd}")
}

/// Project key of a container, the same one the UI uses for its row.
pub fn key_for_container(working_dir: Option<&str>, project: Option<&str>, name: &str) -> String {
    match working_dir {
        Some(dir) => format!("dir:{dir}"),
        None => format!("d:{}", project.unwrap_or(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_the_ui() {
        assert_eq!(key_for_cwd("~/code/app"), "dir:~/code/app");
        assert_eq!(
            key_for_container(Some("~/code/app"), Some("app"), "app-db-1"),
            "dir:~/code/app"
        );
        assert_eq!(key_for_container(None, Some("app"), "app-db-1"), "d:app");
        assert_eq!(key_for_container(None, None, "redis"), "d:redis");
    }

    #[test]
    fn toggles_and_persists() {
        let file =
            std::env::temp_dir().join(format!("portkeeper-settings-{}.json", std::process::id()));
        let mut s = Store::load(file.clone());
        assert!(s.value.show_on_start);
        s.toggle_protected("dir:~/x");
        assert!(Store::load(file.clone()).is_protected("dir:~/x"));
        s.toggle_protected("dir:~/x");
        assert!(!Store::load(file.clone()).is_protected("dir:~/x"));
        let _ = std::fs::remove_file(file);
    }
}
