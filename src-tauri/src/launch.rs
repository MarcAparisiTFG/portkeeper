//! Remember how projects were started and start them again from the Closed tab.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ports::PortInfo;

const MAX_CLOSED: usize = 30;
const MAX_AGE_SECS: u64 = 7 * 24 * 3600;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct PortItem {
    pub port: u16,
    pub label: String,
    pub tech: Option<String>,
}

/// One launch command in one folder: "npm start" in ~/code/my-app.
/// Every field defaults so a file written by an older version still loads.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Entry {
    pub key: String,
    pub title: String,
    pub path: String,
    /// The arguments to run again, as captured. Never sent to the UI.
    pub argv: Vec<String>,
    /// Masked command line for display. Not stored: rebuilt from `argv` on load, so the
    /// file can never show one command and run another.
    #[serde(skip)]
    pub display: String,
    /// The command carries a secret: kept in memory only, never written to disk.
    #[serde(default)]
    pub secret: bool,
    pub items: Vec<PortItem>,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub closed_at: u64,
    pub cwd_abs: String,
    #[serde(skip)]
    pub pid: u32,
}

/// What the UI gets for a closed project: no raw command line.
#[derive(Serialize, Clone, Debug)]
pub struct ClosedView {
    pub key: String,
    pub title: String,
    pub path: String,
    pub display: String,
    pub items: Vec<PortItem>,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub closed_at: u64,
}

#[derive(Default)]
pub struct Tracker {
    active: HashMap<String, Entry>,
    closed: Vec<Entry>,
    /// Captured environments, memory only (they can hold secrets).
    envs: HashMap<String, Vec<(String, String)>>,
    file: Option<PathBuf>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn base(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_string()
}

impl Tracker {
    pub fn load(file: PathBuf) -> Self {
        let mut closed = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<Entry>>(&b).ok())
            .unwrap_or_default();
        // Entries from before argv was stored have nothing safe to run: drop them.
        closed.retain(|e| !e.argv.is_empty() && !e.cwd_abs.is_empty());
        for e in &mut closed {
            e.display = crate::ports::display_of(&e.argv);
        }
        let mut t = Tracker {
            closed,
            file: Some(file),
            ..Default::default()
        };
        t.prune();
        t
    }

    fn save(&self) {
        if let Some(f) = &self.file {
            if let Some(dir) = f.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let keep: Vec<&Entry> = self.closed.iter().filter(|e| !e.secret).collect();
            if let Ok(json) = serde_json::to_vec_pretty(&keep) {
                let _ = std::fs::write(f, json);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o600));
                }
            }
        }
    }

    fn prune(&mut self) {
        let cutoff = now().saturating_sub(MAX_AGE_SECS);
        self.closed.retain(|e| e.closed_at >= cutoff);
        self.closed.sort_by_key(|e| std::cmp::Reverse(e.closed_at));
        self.closed.truncate(MAX_CLOSED);
    }

    /// Feed one scan. Launches that disappeared since the previous scan become closed;
    /// closed launches that are running again leave the Closed list.
    pub fn update(&mut self, ports: &[PortInfo]) {
        let mut current: HashMap<String, Entry> = HashMap::new();
        for p in ports.iter().filter(|p| p.dev) {
            let Some(l) = &p.launch else { continue };
            let path = p.cwd.clone().unwrap_or_default();
            let e = current.entry(l.key.clone()).or_insert_with(|| Entry {
                key: l.key.clone(),
                title: base(&path),
                path,
                argv: l.argv.clone(),
                display: l.display.clone(),
                secret: l.secret,
                items: Vec::new(),
                agent_id: None,
                agent_name: None,
                closed_at: 0,
                cwd_abs: l.cwd_abs.clone(),
                pid: l.pid,
            });
            if !e.items.iter().any(|i| i.port == p.port) {
                e.items.push(PortItem {
                    port: p.port,
                    label: p.label.clone(),
                    tech: p.tech.map(str::to_string),
                });
                e.items.sort_by_key(|i| i.port);
            }
            if e.agent_id.is_none() {
                if let Some(a) = &p.agent {
                    e.agent_id = Some(a.id.to_string());
                    e.agent_name = Some(a.name.to_string());
                }
            }
        }

        let mut changed = false;
        for (key, e) in &current {
            if !self.envs.contains_key(key) {
                self.envs.insert(key.clone(), crate::ports::environ(e.pid));
            }
            let before = self.closed.len();
            self.closed.retain(|c| &c.key != key);
            changed |= before != self.closed.len();
        }
        let gone: Vec<Entry> = self
            .active
            .drain()
            .filter(|(k, _)| !current.contains_key(k))
            .map(|(_, mut e)| {
                e.closed_at = now();
                e
            })
            .collect();
        for e in gone {
            self.closed.retain(|c| c.key != e.key);
            self.closed.push(e);
            changed = true;
        }
        self.active = current;
        if changed {
            self.prune();
            self.save();
        }
    }

    pub fn closed(&self) -> Vec<ClosedView> {
        self.closed
            .iter()
            .map(|e| ClosedView {
                key: e.key.clone(),
                title: e.title.clone(),
                path: e.path.clone(),
                display: e.display.clone(),
                items: e.items.clone(),
                agent_id: e.agent_id.clone(),
                agent_name: e.agent_name.clone(),
                closed_at: e.closed_at,
            })
            .collect()
    }

    pub fn forget(&mut self, key: &str) {
        self.closed.retain(|c| c.key != key);
        self.envs.remove(key);
        self.save();
    }

    /// Start a closed launch again. Returns the log file it writes to.
    pub fn relaunch(&self, key: &str, log_dir: &Path) -> Result<PathBuf, String> {
        let e = self
            .closed
            .iter()
            .find(|c| c.key == key)
            .ok_or("not in the closed list")?;
        let safe: String = e
            .title
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        std::fs::create_dir_all(log_dir).map_err(|e| e.to_string())?;
        let log = log_dir.join(format!("{safe}.log"));
        spawn_detached(
            &e.argv,
            &e.cwd_abs,
            self.envs.get(key).map(Vec::as_slice),
            &log,
        )?;
        Ok(log)
    }
}

/// Run `argv` in `cwd`, detached from Portkeeper, output to `log`.
///
/// No shell ever parses the command. On Unix the user's own login shell is only used to
/// rebuild PATH (nvm, pyenv…) and then `exec "$@"` runs the words as given, so `;`, `|`
/// or `$(…)` inside a captured argument are just characters. With a captured environment
/// it is reused as is. On Windows the executable is started directly, without cmd.exe.
pub fn spawn_detached(
    argv: &[String],
    cwd: &str,
    env: Option<&[(String, String)]>,
    log: &Path,
) -> Result<u32, String> {
    let (exe, rest) = argv.split_first().ok_or("nothing to run")?;
    let out = std::fs::File::create(log).map_err(|e| e.to_string())?;
    let err = out.try_clone().map_err(|e| e.to_string())?;
    let env = env.filter(|e| !e.is_empty());

    #[cfg(unix)]
    let mut cmd = {
        use std::os::unix::process::CommandExt;
        // Portkeeper's own shell, never one named by the process being relaunched.
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut c = Command::new(shell);
        c.arg(if env.is_some() { "-c" } else { "-ilc" })
            .arg("exec \"$@\"")
            .arg("portkeeper") // $0
            .arg(exe)
            .args(rest);
        c.process_group(0); // survives Portkeeper quitting
        c
    };
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        // std refuses .bat/.cmd targets with arguments cmd.exe could misparse (CVE-2024-24576).
        let mut c = Command::new(exe);
        c.args(rest);
        c.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
        c
    };

    if let Some(env) = env {
        cmd.env_clear().envs(env.iter().cloned());
    }
    let mut child = cmd
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .map_err(|e| e.to_string())?;
    let pid = child.id();
    std::thread::spawn(move || {
        let _ = child.wait(); // reap, avoid a zombie
    });
    Ok(pid)
}

#[cfg(test)]
mod tests {
    use super::Tracker;
    use crate::ports::{Launch, PortInfo};

    fn port(port: u16, key: &str, secret: bool) -> PortInfo {
        PortInfo {
            port,
            pid: 0, // pid 0: environ() finds nothing, which is fine here
            process: "node".into(),
            address: "127.0.0.1".into(),
            system: false,
            agent: None,
            cwd: Some("~/code/app".into()),
            cmd: "node".into(),
            label: "vite".into(),
            tech: Some("vite"),
            dev: true,
            launch: Some(Launch {
                key: key.into(),
                display: "npm run dev".into(),
                argv: if secret {
                    vec!["tool".into(), "--token".into(), "abc".into()]
                } else {
                    vec!["npm".into(), "run".into(), "dev".into()]
                },
                secret,
                pid: 0,
                cwd_abs: "/tmp".into(),
            }),
            started_at: None,
        }
    }

    /// Relaunch without a captured environment goes through the login shell, but the
    /// arguments are never parsed by it: shell syntax inside one is plain text.
    #[cfg(unix)]
    #[test]
    fn relaunch_does_not_parse_arguments() {
        let dir = std::env::temp_dir().join(format!("portkeeper-spawn-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("pwned");
        let log = dir.join("out.log");
        let argv: Vec<String> = [
            "printf",
            "%s",
            &format!("x; touch {}; echo $(id)", marker.display()),
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        super::spawn_detached(&argv, dir.to_str().unwrap(), None, &log).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::fs::read_to_string(&log).map_or(true, |s| s.is_empty()) {
            assert!(std::time::Instant::now() < deadline, "printf never ran");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        let out = std::fs::read_to_string(&log).unwrap();
        assert!(out.contains("x; touch"), "argument passed verbatim: {out}");
        assert!(out.contains("$(id)"), "no command substitution: {out}");
        assert!(!marker.exists(), "an argument must never be executed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn closes_and_reopens() {
        let file =
            std::env::temp_dir().join(format!("portkeeper-closed-{}.json", std::process::id()));
        let mut t = Tracker::load(file.clone());
        t.update(&[
            port(5173, "a", false),
            port(5174, "a", false),
            port(9000, "b", true),
        ]);
        assert!(t.closed().is_empty(), "nothing closed while running");

        t.update(&[]);
        let closed = t.closed();
        assert_eq!(closed.len(), 2);
        let a = closed.iter().find(|c| c.key == "a").unwrap();
        assert_eq!(
            a.items.iter().map(|i| i.port).collect::<Vec<_>>(),
            vec![5173, 5174]
        );
        assert_eq!(a.title, "app");

        // Only the entry without a secret reaches the disk.
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.contains("\"npm\"") && saved.contains("\"dev\""));
        assert!(!saved.contains("--token"));
        // The display is not stored: a fresh load rebuilds it from the argv that will run.
        assert!(!saved.contains("display"));
        let reloaded = Tracker::load(file.clone()).closed();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].display, "npm run dev");

        t.update(&[port(5173, "a", false)]);
        assert_eq!(
            t.closed()
                .iter()
                .map(|c| c.key.as_str())
                .collect::<Vec<_>>(),
            vec!["b"]
        );

        t.forget("b");
        assert!(t.closed().is_empty());
        let _ = std::fs::remove_file(file);
    }
}
