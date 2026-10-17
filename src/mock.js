// Browser-only harness: iterate on the UI without Rust.
// `python3 -m http.server -d src 4173` and open http://localhost:4173.
// Inside the real app `window.__TAURI__` exists, so this file does nothing.
if (!window.__TAURI__) {
  // `?embed`: the live demo on the website. Transparent card, height reported to the page.
  const Q = new URLSearchParams(location.search);
  const EMBED = Q.has("embed");
  document.addEventListener("DOMContentLoaded", () => document.body.classList.add("mock", ...(EMBED ? ["embed"] : [])));
  // `?shot=active|expanded|closed|settings` (+ `&theme=light`): fixed states for the
  // README screenshots, see assets/screenshots.sh.
  // The website's light/dark toggle drives the demo's theme.
  if (EMBED) {
    addEventListener("message", (e) => {
      if (e.data && e.data.portkeeper === "theme" && ["dark", "light"].includes(e.data.value) && settings.theme !== e.data.value) {
        settings.theme = e.data.value;
        if (typeof refresh === "function") refresh(true);
      }
    });
  }
  const SHOT = Q.get("shot");
  if (SHOT) {
    try { localStorage.setItem("tab", SHOT === "closed" ? "closed" : "active"); } catch { /* ignore */ }
    window.addEventListener("load", () => setTimeout(() => {
      if (SHOT === "settings") document.getElementById("gear").click();
      if (SHOT === "expanded") {
        const row = [...document.querySelectorAll("#list .row")].find((r) => r.textContent.includes("video-studio"));
        if (row) row.click();
      }
    }, 150));
  }
  const P = (port, pid, process, cwd, agent = null, label = process, tech = null) => ({
    port, pid, process, address: "127.0.0.1", system: false, agent, cwd, cmd: label, label, tech,
    dev: (cwd.startsWith("~/") && !cwd.startsWith("~/Library/Containers/")) || ["ollama"].includes(tech),
    launch: cwd.startsWith("~/") ? { key: `${cwd}:${label}`, display: label } : null,
    started_at: Math.floor(Date.now() / 1000) - ((port * 7919) % 300000),
  });
  const now = Date.now() / 1000;
  const settings = { theme: Q.get("theme") || "dark", glass: !/Windows/i.test(navigator.userAgent), lang: "", show_on_start: true, show_all: false, launch_at_login: false, check_updates: true, close_on_blur: false, version: "0.4.1", protected: ["dir:~/code/illeta-digital"] };
  let closed = [
    { key: "c1", title: "blog", path: "~/code/blog", display: "npm run dev", items: [{ port: 4321, label: "astro", tech: "astro" }], agent_id: "claude", agent_name: "Claude Code", closed_at: now - 600 },
    { key: "c2", title: "crm", path: "~/code/crm", display: "pnpm dev", items: [{ port: 3001, label: "next", tech: "next" }, { port: 5555, label: "prisma", tech: "node" }], agent_id: null, agent_name: null, closed_at: now - 7200 },
  ];
  const C = (name, project, service, ports, running = true) => ({
    id: name.replace(/\W/g, "").slice(0, 12), name, image: `${service}:latest`, state: running ? "running" : "exited",
    status: running ? "Up 2 days" : "Exited (0) 5 days ago", ports: "", published_ports: ports, running,
    project, service, working_dir: `~/code/${project}`,
    started_at: running ? Math.floor(Date.now() / 1000) - 864000 : null,
  });
  const data = {
    ports: [
      P(3000, 1, "com.docker.backend", "~/Library/Containers/com.docker.docker/Data", null, "com.docker.backend", "docker"),
      P(4321, 20, "node", "~/code/maparisi-portfolio", { id: "claude", name: "Claude Code" }, "astro", "astro"),
      P(5173, 21, "node", "~/code/dashboard", { id: "cursor", name: "Cursor" }, "vite", "vite"),
      P(8000, 22, "Python", "~/code/rag-api", { id: "codex", name: "Codex" }, "uvicorn", "python"),
      P(8001, 29, "Python", "~/code/gemini-lab", { id: "gemini", name: "Gemini CLI" }, "app.py", "python"),
      P(9000, 23, "node", "~/code/video-studio", null, "vite", "vite"),
      P(9100, 24, "node", "~/code/video-studio", null, "dashboard/server.mjs", "node"),
      P(20241, 28, "cloudflared", "~/code/illeta-digital", { id: "claude", name: "Claude Code" }, "cloudflared", "cloudflare"),
      P(11434, 25, "ollama", "/opt/homebrew/var", null, "ollama", "ollama"),
      P(7768, 26, "Spotify", "/"),
      P(52083, 27, "Code Helper", "/", { id: "vscode", name: "VS Code" }),
    ],
    docker: {
      kind: "ok",
      containers: [
        C("illeta-digital-api-1", "illeta-digital", "api", [3000]),
        C("illeta-digital-db-1", "illeta-digital", "db", [3307]),
        C("illeta-digital-web-1", "illeta-digital", "web", [8080]),
        C("billing-postgres-1", "billing", "postgres", [], false),
      ],
    },
  };
  window.__TAURI__ = {
    event: { listen: async () => () => {} },
    core: {
      invoke: async (cmd, args = {}) => {
        switch (cmd) {
          case "list_ports": return data.ports;
          case "list_containers": return data.docker;
          case "get_settings": return { ...settings, protected: [...settings.protected] };
          case "set_setting": settings[args.name] = args.value; return;
          case "set_theme": settings.theme = args.value; return;
          case "set_lang": settings.lang = args.value; return;
          case "toggle_protected": {
            const k = args.key;
            settings.protected = settings.protected.includes(k) ? settings.protected.filter((x) => x !== k) : [...settings.protected, k];
            return settings.protected;
          }
          case "list_closed": return closed;
          case "fit_window": if (EMBED) parent.postMessage({ portkeeper: "height", value: args.height, theme: settings.theme }, "*"); return;
          case "check_update": return Q.has("update") ? { version: "0.5.0", notes: "### Added\n- **Forget all** under the Closed list, to clear it in one click.\n- One-command install for macOS, Linux and Windows.\n\n### Security\n- **Start again** runs the saved command as separate arguments, never through a shell.\n- Opening a port only ever opens `http://localhost:<port>`.\n\n---\nDownload the installer for your platform below." } : null;
          case "install_update": await new Promise((r) => setTimeout(r, 2500)); throw "Demo: nothing to install";
          case "probe_ports": return Object.fromEntries(args.ports.map(([p]) => [p, p !== 8001])); // 8001 "still booting"
          case "forget_closed": closed = closed.filter((c) => c.key !== args.key); return;
          case "forget_all_closed": closed = []; return;
          case "relaunch": {
            const c = closed.find((x) => x.key === args.key);
            setTimeout(() => {
              closed = closed.filter((x) => x !== c);
              c.items.forEach((i, n) => data.ports.push(P(i.port, 900 + n, "node", c.path, c.agent_id ? { id: c.agent_id, name: c.agent_name } : null, i.label, i.tech)));
            }, 1500);
            return "~/Library/Logs/dev.maparisi.portkeeper/mock.log";
          }
          case "kill_process": await new Promise((r) => setTimeout(r, 1500)); data.ports = data.ports.filter((p) => p.pid !== args.pid); return; // like the real 1.5 s grace
          case "docker_action": {
            const c = data.docker.containers.find((x) => x.id === args.id);
            if (args.action === "remove") data.docker.containers = data.docker.containers.filter((x) => x !== c);
            else c.running = args.action !== "stop";
            await new Promise((r) => setTimeout(r, 300));
            return;
          }
          default: return;
        }
      },
    },
  };
}
