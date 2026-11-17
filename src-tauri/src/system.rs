//! Small cross-platform helpers: PATH handling, hidden consoles, terminals.

use std::path::PathBuf;
use std::process::Command;

/// GUI apps on macOS start with a minimal PATH; add the usual places where
/// Homebrew / Docker Desktop put their binaries.
pub fn extended_path() -> String {
    let mut parts: Vec<String> = Vec::new();
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        parts.extend([
            "/usr/local/bin".into(),
            "/opt/homebrew/bin".into(),
            format!("{home}/.docker/bin"),
            "/usr/bin".into(),
            "/bin".into(),
            "/usr/sbin".into(),
            "/sbin".into(),
        ]);
    }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        parts.extend([
            "/usr/local/bin".into(),
            format!("{home}/.docker/bin"),
            "/usr/bin".into(),
            "/bin".into(),
            "/snap/bin".into(),
        ]);
    }
    #[cfg(target_os = "windows")]
    {
        parts.push(r"C:\Program Files\Docker\Docker\resources\bin".into());
    }
    if let Ok(p) = std::env::var("PATH") {
        parts.push(p);
    }
    parts.join(if cfg!(windows) { ";" } else { ":" })
}

/// Locate the `docker` CLI without relying on the (possibly minimal) inherited PATH.
pub fn find_docker() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "docker.exe"
    } else {
        "docker"
    };
    std::env::split_paths(&extended_path())
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

/// On Windows, keep helper commands from flashing a console window.
pub fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// Open the user's terminal and run `command` in it (used for `docker logs -f`).
pub fn open_in_terminal(command: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
        let script =
            format!("tell application \"Terminal\"\nactivate\ndo script \"{escaped}\"\nend tell");
        let out = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "Portkeeper logs", "cmd", "/K", command]);
        cmd.env("PATH", extended_path());
        hide_console(&mut cmd);
        cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        let candidates: [(&str, Vec<&str>); 4] = [
            ("x-terminal-emulator", vec!["-e", "sh", "-c", command]),
            ("gnome-terminal", vec!["--", "sh", "-c", command]),
            ("konsole", vec!["-e", "sh", "-c", command]),
            ("xterm", vec!["-e", "sh", "-c", command]),
        ];
        for (bin, args) in candidates {
            if Command::new(bin).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }
        Err("No terminal emulator found (tried x-terminal-emulator, gnome-terminal, konsole, xterm)".into())
    }
}

/// The user's home folder, or "" if unknown.
pub fn home() -> String {
    std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default()
}

/// Replace the home directory prefix with `~` for display.
pub fn tilde(path: &str) -> String {
    tilde_with(path, &home())
}

fn tilde_with(path: &str, home: &str) -> String {
    let home = home.trim_end_matches(['/', '\\']);
    // Only the home folder itself: "/Users/marc2" is not under "/Users/marc".
    match path.strip_prefix(home) {
        Some(rest) if !home.is_empty() && (rest.is_empty() || rest.starts_with(['/', '\\'])) => {
            format!("~{rest}")
        }
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tilde_only_shortens_the_home_folder() {
        use super::tilde_with;
        assert_eq!(
            tilde_with("/Users/marc/code/app", "/Users/marc"),
            "~/code/app"
        );
        assert_eq!(tilde_with("/Users/marc", "/Users/marc"), "~");
        assert_eq!(
            tilde_with("/Users/marc2/app", "/Users/marc"),
            "/Users/marc2/app"
        );
        assert_eq!(tilde_with("/srv/app", "/Users/marc"), "/srv/app");
        assert_eq!(tilde_with("/srv/app", ""), "/srv/app");
        assert_eq!(tilde_with("C:\\Users\\m\\app", "C:\\Users\\m"), "~\\app");
    }
}
