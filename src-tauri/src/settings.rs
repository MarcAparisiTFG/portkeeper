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
    /// Extra folders whose processes count as projects, besides the home folder
    /// ("/srv", "/Volumes/Work"…). Stored with the home folder shortened to "~".
    pub project_folders: Vec<String>,
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
            project_folders: Vec::new(),
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

/// Most folders the project list can hold.
pub const MAX_PROJECT_FOLDERS: usize = 20;

/// `path` is `folder` itself or somewhere inside it. Either separator counts, so
/// Windows paths work too; "/srv" does not contain "/srv2".
pub fn is_under(path: &str, folder: &str) -> bool {
    let folder = folder.trim_end_matches(['/', '\\']);
    match path.strip_prefix(folder) {
        Some(rest) => rest.is_empty() || rest.starts_with(['/', '\\']),
        None => false,
    }
}

/// Does a process running in `cwd` (home shortened to "~") belong to a project? Anything
/// under the home folder, the home folder itself included, except app sandboxes; and
/// anything under one of the user's extra project folders.
pub fn is_project_dir(cwd: &str, extra: &[String]) -> bool {
    if is_under(cwd, "~") {
        return !is_under(cwd, "~/Library/Containers") && !is_under(cwd, "~\\Library\\Containers");
    }
    extra.iter().any(|f| is_under(cwd, f))
}

/// Check a folder typed in Settings and return it the way it is stored: absolute (or
/// starting with "~"), an existing directory, never the root of a drive, home shortened.
pub fn normalize_project_folder(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() || raw.len() > 1024 || raw.chars().any(char::is_control) {
        return Err("Type the full path of a folder.".into());
    }
    let home = crate::system::home();
    let expanded = match raw.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => format!("{home}{rest}"),
        _ => raw.to_string(),
    };
    let path = std::path::Path::new(&expanded);
    if !path.is_absolute() {
        return Err("Type the full path of a folder, like /srv or ~/work.".into());
    }
    if path.parent().is_none() {
        return Err("The root folder is too broad to be a project folder.".into());
    }
    if !path.is_dir() {
        return Err(format!("{raw} is not a folder."));
    }
    let shown = crate::system::tilde(&expanded);
    Ok(shown.trim_end_matches(['/', '\\']).to_string())
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
    fn project_dirs() {
        let none: Vec<String> = Vec::new();
        assert!(is_project_dir("~/code/app", &none));
        assert!(is_project_dir("~", &none), "a server started from ~ itself");
        assert!(!is_project_dir(
            "~/Library/Containers/com.docker.docker/Data",
            &none
        ));
        assert!(!is_project_dir("/srv/app", &none));
        assert!(!is_project_dir("/", &none));
        assert!(!is_project_dir("~other/x", &none));
        let extra = vec!["/srv".to_string(), "/Volumes/Work/".to_string()];
        assert!(is_project_dir("/srv", &extra));
        assert!(is_project_dir("/srv/www/site", &extra));
        assert!(is_project_dir("/Volumes/Work/api", &extra));
        assert!(!is_project_dir("/srv2/app", &extra));
        assert!(!is_project_dir("/var/www", &extra));
        assert!(is_project_dir("D:\\code\\app", &["D:\\code".to_string()]));
    }

    #[test]
    fn project_folders_are_checked() {
        let tmp = std::env::temp_dir();
        let tmp = tmp.to_string_lossy();
        assert!(normalize_project_folder(&tmp).is_ok());
        assert!(normalize_project_folder("").is_err());
        assert!(normalize_project_folder("relative/path").is_err());
        assert!(normalize_project_folder("/").is_err());
        assert!(normalize_project_folder("/definitely/not/here/pk").is_err());
        assert!(normalize_project_folder("/tmp\n/x").is_err());
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
