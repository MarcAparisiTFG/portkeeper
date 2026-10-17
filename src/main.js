// Portkeeper front-end: projects with listening ports, grouped by folder.
const { invoke } = window.__TAURI__.core;
const $ = (s) => document.querySelector(s);
const el = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text != null) n.textContent = text;
  return n;
};

// id -> [svg file in src/icons, colour]. Colour null = brand is black/white, drawn white.
const ICONS = {
  // agents / IDEs (src-tauri/src/ports.rs)
  claude: ["claude", "#D97757"], codex: ["codex", "#10A37F"], cursor: ["cursor", null],
  gemini: ["googlegemini", "#8E75B2"], copilot: ["githubcopilot", null], windsurf: ["windsurf", "#4FD1C5"],
  opencode: ["opencode", null], cline: ["cline", null], jetbrains: ["jetbrains", "#FE2857"],
  zed: ["zedindustries", "#4A8CFF"], xcode: ["xcode", "#147EFB"], vscode: ["vscode", "#3B9EFF"],
  // what serves the port
  docker: ["docker", "#2496ED"], node: ["nodedotjs", "#5FA04E"], javascript: ["javascript", "#D9C300"],
  vite: ["vite", "#A78BFA"], next: ["nextdotjs", null], astro: ["astro", "#BC52EE"], nuxt: ["nuxt", "#00DC82"],
  storybook: ["storybook", "#FF4785"], bun: ["bun", "#C9A227"], deno: ["deno", null],
  python: ["python", "#3776AB"], ruby: ["ruby", "#CC342D"], php: ["php", "#7A86B8"], java: ["openjdk", "#E07B00"],
  go: ["go", "#00A8CF"], dotnet: ["dotnet", "#7B5FD9"], rust: ["rust", "#B7410E"], ollama: ["ollama", null],
  postgres: ["postgresql", "#6A8FE8"], mysql: ["mysql", "#5B9BD5"], redis: ["redis", "#FF4438"],
  cloudflare: ["cloudflare", "#F38020"],
};

const state = {
  ports: [],
  docker: { kind: "ok", containers: [] },
  settings: { show_on_start: true, show_all: false, protected: [], launch_at_login: false, version: "" },
  view: "list", // "list" | "settings"
  closed: [],
  tab: (() => { try { return localStorage.getItem("tab") || "active"; } catch { return "active"; } })(),
  starting: new Map(), // closed key -> started at (ms)
  expanded: new Set(),
  responding: new Map(), // port -> bool, from probe_ports
  confirming: null,
  busy: new Set(),
  update: null, // { version, notes } when GitHub has a newer signed release
  showNotes: false, // "What's new" unfolded under the update bar
  updating: false,
  stopping: new Map(), // pid or container id -> when Stop was pressed; kept until it is gone
};

let toastTimer;
function toast(msg, ms = 1500) {
  const t = $("#toast");
  t.textContent = msg;
  t.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.add("hidden"), ms);
}
// Only errors are announced; a row changing is the feedback for success.
async function call(cmd, args) {
  try {
    return await invoke(cmd, args);
  } catch (e) {
    toast(String(e), 4000);
    throw e;
  }
}

// Windows gets an opaque, natively rounded window (see tauri.windows.conf.json).
if (/Windows/i.test(navigator.userAgent)) document.body.classList.add("win");

// ---------- data ----------
let refreshing = false;
let lastSnapshot = "";
async function refresh(force = false) {
  if (refreshing) return;
  refreshing = true;
  try {
    const [ports, docker, settings] = await Promise.all([
      invoke("list_ports").catch(() => state.ports),
      invoke("list_containers").catch((e) => ({ kind: "not_running", message: String(e) })),
      invoke("get_settings").catch(() => state.settings),
    ]);
    // After list_ports, so the tracker has seen this scan.
    const closed = await invoke("list_closed").catch(() => state.closed);
    settleStarting(closed);
    pruneStopping(ports, docker);
    const toProbe = [];
    for (const p of ports) if (p.dev || settings.show_all) toProbe.push([p.port, p.tech]);
    if (docker.kind === "ok") for (const c of docker.containers) if (c.running) for (const port of c.published_ports) toProbe.push([port, "docker"]);
    const responding = await invoke("probe_ports", { ports: toProbe }).catch(() => ({}));
    const snap = JSON.stringify([ports, docker, settings, closed, [...state.starting.keys()], [...state.stopping.keys()], responding]);
    if (force || snap !== lastSnapshot) {
      lastSnapshot = snap;
      state.ports = ports;
      state.docker = docker;
      state.closed = closed;
      state.settings = settings;
      applyTheme(settings);
      setLanguage(settings.lang || "");
      for (const n of document.querySelectorAll("[data-t]")) n.textContent = t(n.dataset.t);
      $("#min").title = t("minimize");
      $("#gear").title = t("settings");
      state.responding = new Map(Object.entries(responding).map(([k, v]) => [Number(k), v]));
      render();
    }
  } finally {
    refreshing = false;
  }
}

// ---------- grouping ----------
const base = (path) => path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || path;
const inProject = (cwd) => !!cwd && cwd.startsWith("~/") && !cwd.startsWith("~/Library/Containers/");

// A group = one project folder (or one lone process / container) with its port items.
function groups() {
  const map = new Map();
  const add = (key, init) => {
    if (!map.has(key)) map.set(key, { key, items: [], ...init });
    return map.get(key);
  };
  const containers = state.docker.kind === "ok" ? state.docker.containers : [];

  const dockerPorts = new Set();
  for (const c of containers.filter((c) => c.running && c.published_ports.length)) {
    const dir = c.working_dir;
    const g = add(dir ? `dir:${dir}` : `d:${c.project || c.name}`, { title: dir ? base(dir) : c.project || c.name, path: dir || c.image });
    for (const port of c.published_ports) {
      g.items.push({ key: `c:${c.id}:${port}`, port, label: c.service || c.name, tech: "docker", container: c, started: c.started_at });
      dockerPorts.add(port);
    }
  }

  for (const p of state.ports) {
    if (dockerPorts.has(p.port)) continue;
    const project = inProject(p.cwd);
    if (!state.settings.show_all && !p.dev) continue;
    const g = add(project ? `dir:${p.cwd}` : `pid:${p.pid}`, { title: project ? base(p.cwd) : p.label, path: project ? p.cwd : "" });
    g.items.push({ key: `p:${p.pid}:${p.port}`, port: p.port, label: p.label, tech: p.tech, pid: p.pid, agent: p.agent, cmd: p.launch?.display || p.cmd, started: p.started_at });
  }

  const list = [...map.values()];
  for (const g of list) {
    const seen = new Set();
    g.items = g.items.sort((a, b) => a.port - b.port).filter((i) => !seen.has(i.port) && seen.add(i.port));
    g.agent = g.items.find((i) => i.agent)?.agent;
    const starts = g.items.map((i) => i.started).filter(Boolean);
    g.started = starts.length ? Math.min(...starts) : null; // the longest-running part
    // Who started it wins (Claude, Codex, Cursor…); otherwise what it is (Docker, Vite, Node…).
    g.icon = g.agent?.id || g.items[0].tech;
  }
  // Agents first (alphabetically), projects without one last, then by port.
  const key = (g) => (g.agent ? "0" + g.agent.name : "1");
  return list.sort((a, b) => key(a).localeCompare(key(b)) || a.items[0].port - b.items[0].port);
}

function stoppedGroups() {
  if (state.docker.kind !== "ok") return [];
  const map = new Map();
  for (const c of state.docker.containers.filter((c) => !c.running)) {
    const key = `s:${c.working_dir || c.project || c.name}`;
    const lockKey = c.working_dir ? `dir:${c.working_dir}` : `d:${c.project || c.name}`;
    if (!map.has(key)) map.set(key, { key, lockKey, title: c.working_dir ? base(c.working_dir) : c.project || c.name, path: c.working_dir || c.image, containers: [] });
    map.get(key).containers.push(c);
  }
  return [...map.values()];
}

// ---------- actions ----------
const open = (port) => call("open_url", { url: `http://localhost:${port}` });

/** Stop: the rows show "stopping…" from the click until the process or container is really gone. */
async function stopItems(items) {
  const containers = [...new Map(items.filter((i) => i.container).map((i) => [i.container.id, i.container])).values()];
  const pids = [...new Set(items.filter((i) => i.pid).map((i) => i.pid))];
  const ids = [...pids, ...containers.map((c) => c.id)];
  ids.forEach((id) => state.stopping.set(id, Date.now()));
  render();
  try {
    for (const pid of pids) await call("kill_process", { pid, force: false });
    for (const c of containers) await call("docker_action", { id: c.id, action: "stop" });
  } catch {
    // The error toast is already shown; bring the row back.
    ids.forEach((id) => state.stopping.delete(id));
    render();
  }
  refresh(true);
}
const STOP_TIMEOUT = 20000;
function pruneStopping(ports, docker) {
  for (const [id, at] of state.stopping) {
    const gone = typeof id === "number"
      ? !ports.some((p) => p.pid === id)
      : !(docker.kind === "ok" && docker.containers.some((c) => c.id === id && c.running));
    if (gone || Date.now() - at > STOP_TIMEOUT) state.stopping.delete(id);
  }
}
const isStopping = (items) => items.some((i) => state.stopping.has(i.container?.id) || state.stopping.has(i.pid));
async function dockerAll(containers, action) {
  containers.forEach((c) => state.busy.add(c.id));
  render();
  try {
    for (const c of containers) await call("docker_action", { id: c.id, action });
  } catch { /* toast already shown */ }
  finally {
    containers.forEach((c) => state.busy.delete(c.id));
    refresh(true);
  }
}
// ---------- closed / relaunch ----------
const STARTUP_TIMEOUT = 30000;
function settleStarting(closed) {
  for (const [key, st] of state.starting) {
    if (!closed.some((c) => c.key === key)) {
      state.starting.delete(key);
    } else if (Date.now() - st.at > STARTUP_TIMEOUT) {
      state.starting.delete(key);
      toast(t("didntStart", { log: st.log }), 6000);
    }
  }
}
async function relaunch(c) {
  try {
    const log = await invoke("relaunch", { key: c.key });
    state.starting.set(c.key, { at: Date.now(), log });
    render();
  } catch (e) {
    toast(String(e), 4000);
  }
}
/** "5m", "3h", "2d": how long something has been running. */
function uptime(since) {
  if (!since) return "";
  const d = Math.max(0, Date.now() / 1000 - since);
  if (d < 60) return t("now");
  if (d < 3600) return `${Math.floor(d / 60)}m`;
  if (d < 86400) return `${Math.floor(d / 3600)}h`;
  return `${Math.floor(d / 86400)}d`;
}

function ago(secs) {
  const d = Math.max(0, Date.now() / 1000 - secs);
  if (d < 60) return "now";
  if (d < 3600) return `${Math.floor(d / 60)}m`;
  if (d < 86400) return `${Math.floor(d / 3600)}h`;
  return `${Math.floor(d / 86400)}d`;
}


// ---------- lock ----------
const lockable = (key) => key.startsWith("dir:") || key.startsWith("d:");
const isLocked = (key) => state.settings.protected.includes(key);
async function toggleLock(key) {
  try {
    state.settings.protected = await invoke("toggle_protected", { key });
  } catch (e) { toast(String(e), 4000); }
  state.confirming = null;
  render();
}
const lockIco = (cls = "lock") => el("i", `ico ${cls}`);

// ---------- rendering ----------
function btn(label, title, fn, cls = "") {
  const b = el("button", cls, label);
  b.title = title;
  b.addEventListener("click", (e) => { e.stopPropagation(); fn(); });
  return b;
}
/** Button with a drawn icon (text glyphs like ■ ▶ ✕ render hairline-thin on Windows). */
function ibtn(ico, title, fn, cls = "") {
  const b = btn("", title, fn, cls);
  b.appendChild(el("i", `ico ${ico}`));
  return b;
}
/** Spinner + "stopping…" / "starting…" in place of a row's ports and actions. */
function doing(row, txt, label) {
  row.classList.add("doing");
  txt.querySelector(".sub")?.replaceChildren(el("span", "doing-txt", label));
  row.appendChild(el("span", "spin"));
}
function glyph(id) {
  const spec = ICONS[id];
  if (!spec) return null;
  const g = el("i", "glyph");
  g.style.setProperty("--m", `url(icons/${spec[0]}.svg)`);
  if (spec[1]) g.style.setProperty("--c", spec[1]); // brand colour; black/white brands follow the text colour
  return g;
}
function tile(id, fallback) {
  const t = el("span", "tile");
  const g = glyph(id);
  t.appendChild(g || el("b", "", (fallback || "•").slice(0, 2).toUpperCase()));
  return t;
}
function portBtn(port) {
  return btn(String(port), t("openUrl", { url: `http://localhost:${port}` }), () => open(port), "p");
}
/** Status dot: solid green when the port answers, pulsing amber while it is listening but silent. */
function dot(ports, since) {
  const ok = ports.every((p) => state.responding.get(p) !== false);
  const d = el("span", ok ? "dot on" : "dot warn");
  d.title = !ok ? t("notResponding") : since ? t("runningSince", { date: new Date(since * 1000).toLocaleString([], { dateStyle: "medium", timeStyle: "short" }), up: uptime(since) }) : "";
  return d;
}
/** Inline confirm: replaces the action buttons of one row. */
/** Boxed confirmation that takes the place of a row: "Stop portfolio?  [Stop]". Click elsewhere or Esc to cancel. */
function confirmBox(prompt, choices) {
  const box = el("div", "cbox");
  box.appendChild(el("span", "q", prompt));
  const acts = el("span", "cacts");
  for (const [label, fn] of choices) acts.appendChild(btn(label, label, () => { state.confirming = null; fn(); }, "primary"));
  box.appendChild(acts);
  state.confirmOpenedAt = Date.now();
  return box;
}
// Clicking anywhere outside the box (or Esc) cancels a pending confirmation.
document.addEventListener("click", (e) => {
  if (!state.confirming || e.target.closest(".cbox") || Date.now() - state.confirmOpenedAt < 150) return;
  state.confirming = null;
  render();
});
/** First letter up for single-word labels (vite -> Vite); paths and dotted names stay as they are. */
const cap = (t) => (/^[a-z][a-z0-9-]*$/.test(t) ? t[0].toUpperCase() + t.slice(1) : t);

function groupEl(g) {
  const wrap = el("li", state.expanded.has(g.key) ? "open" : "");
  const multi = g.items.length > 1;
  const locked = isLocked(g.key);
  const row = el("div", `row${multi ? " multi" : ""}`);
  row.title = [g.path, g.agent ? t("startedBy", { x: g.agent.name }) : "", locked ? t("lockedTip") : ""].filter(Boolean).join("\n");

  const stopping = g.items.every((i) => isStopping([i]));
  if (!stopping && state.confirming === g.key) {
    row.appendChild(confirmBox(t("stopQ", { x: g.title }), [[t("stop"), () => stopItems(g.items)]]));
    wrap.appendChild(row);
    return wrap;
  }

  row.appendChild(tile(g.icon, g.title));
  const txt = el("span", "txt");
  const line = el("span", "line");
  line.appendChild(el("span", "title", g.title));
  if (locked) { const l = lockIco(); l.classList.add("title-lock"); l.title = t("locked"); line.appendChild(l); }
  txt.appendChild(line);
  const sub = el("span", "sub");
  const plist = el("span", "plist");
  if (multi) {
    for (const i of g.items) plist.appendChild(portBtn(i.port));
  } else {
    plist.appendChild(portBtn(g.items[0].port));
    plist.appendChild(el("span", "", cap(g.items[0].label)));
  }
  plist.title = g.items.map((i) => i.port).join("  ");
  sub.appendChild(plist);
  txt.appendChild(sub);
  row.appendChild(txt);
  if (stopping) {
    doing(row, txt, t("stopping"));
    wrap.appendChild(row);
    return wrap;
  }

  const acts = el("span", "act hov");
  if (lockable(g.key)) {
    const lb = btn("", locked ? t("unlock") : t("lock"), () => toggleLock(g.key));
    lb.appendChild(lockIco(locked ? "unlock" : "lock"));
    acts.appendChild(lb);
  }
  if (!locked) acts.appendChild(ibtn("stop", multi ? t("stopAll") : t("stop"), () => { state.confirming = g.key; render(); }, "stop"));
  row.appendChild(acts);
  row.appendChild(dot(g.items.map((i) => i.port), g.started));
  const chev = el("span", "chev");
  if (multi) chev.appendChild(el("i", "ico down"));
  row.appendChild(chev);
  if (multi) {
    row.addEventListener("click", () => {
      if (state.expanded.has(g.key)) state.expanded.delete(g.key);
      else { state.expanded.add(g.key); state.justExpanded = g.key; }
      render();
      state.justExpanded = null;
    });
  }
  wrap.appendChild(row);

  if (multi && state.expanded.has(g.key)) {
    const ul = el("ul", "ports");
    for (const i of g.items) ul.appendChild(portEl(i, locked));
    wrap.appendChild(ul);
  }
  return wrap;
}

function portEl(i, locked) {
  const li = el("li");
  const row = el("div", "row");
  row.title = i.container ? `${i.container.name} · ${i.container.image}` : i.cmd || "";
  if (isStopping([i])) {
    row.classList.add("doing");
    row.appendChild(glyph(i.tech) || el("i", "glyph"));
    row.appendChild(el("span", "p", String(i.port)));
    row.appendChild(el("span", "lbl", t("stopping")));
    row.appendChild(el("span", "spin"));
  } else if (state.confirming === i.key) {
    row.appendChild(confirmBox(t("stopQ", { x: `${cap(i.label)} · ${i.port}` }), [[t("stop"), () => stopItems([i])]]));
  } else {
    row.appendChild(glyph(i.tech) || el("i", "glyph"));
    row.appendChild(portBtn(i.port));
    row.appendChild(el("span", "lbl", cap(i.label)));
    const acts = el("span", "act hov");
    if (!locked) acts.appendChild(ibtn("stop", i.container ? t("stopContainer", { x: i.container.name }) : t("stopX", { x: i.label }), () => { state.confirming = i.key; render(); }, "stop"));
    row.appendChild(acts);
    row.appendChild(dot([i.port], i.started));
    row.appendChild(el("span", "chev"));
  }
  li.appendChild(row);
  return li;
}

function closedRow({ key, icon, title, ports, when, tip, busy, onStart, onRemove, removeLabel, locked }) {
  const li = el("li", "closed");
  const row = el("div", "row");
  row.title = tip || "";
  if (!busy && state.confirming === key) {
    row.appendChild(confirmBox(t(removeLabel === "Forget" ? "forgetQ" : "removeQ", { x: title }), [[t(removeLabel.toLowerCase()), onRemove]]));
  } else {
    row.appendChild(tile(icon, title));
    const txt = el("span", "txt");
    const line = el("span", "line");
    line.appendChild(el("span", "title", title));
    if (locked) { const l = lockIco(); l.classList.add("title-lock"); l.title = t("locked"); line.appendChild(l); }
    txt.appendChild(line);
    const sub = el("span", "sub");
    const plist = el("span", "plist");
    for (const p of ports) plist.appendChild(el("span", "p", String(p)));
    sub.appendChild(plist);
    if (when) sub.appendChild(el("span", "ago", when));
    txt.appendChild(sub);
    row.appendChild(txt);
    if (busy) {
      doing(row, txt, t("starting"));
      li.appendChild(row);
      return li;
    }
    const acts = el("span", "act hov");
    acts.appendChild(ibtn("play", t("startAgain"), onStart));
    if (!locked) acts.appendChild(ibtn("x", t(removeLabel.toLowerCase()), () => { state.confirming = key; render(); }, "stop"));
    row.appendChild(acts);
    row.appendChild(el("span", "dot"));
    row.appendChild(el("span", "chev"));
  }
  li.appendChild(row);
  return li;
}

function closedRows() {
  const rows = state.closed.map((c) => closedRow({
    key: c.key, title: c.title, icon: c.agent_id || c.items[0]?.tech, ports: c.items.map((i) => i.port),
    when: ago(c.closed_at), tip: [c.path, c.display].join("\n"), busy: state.starting.has(c.key),
    onStart: () => relaunch(c),
    onRemove: () => invoke("forget_closed", { key: c.key }).then(() => refresh(true)),
    removeLabel: "Forget",
  }));
  for (const g of stoppedGroups()) {
    rows.push(closedRow({
      key: g.key, title: g.title, icon: "docker", ports: [], when: g.containers.map((c) => c.service || c.name).join(", "),
      tip: g.path, busy: g.containers.some((c) => state.busy.has(c.id)),
      onStart: () => dockerAll(g.containers, "start"),
      onRemove: () => dockerAll(g.containers, "remove"),
      removeLabel: "Remove",
      locked: isLocked(g.lockKey),
    }));
  }
  return rows;
}

/** "Forget all" under the Closed list, so a long list does not need a click per row. */
function forgetAllRow() {
  const li = el("li", "clear");
  if (state.confirming === "forget-all") {
    const row = el("div", "row");
    row.appendChild(confirmBox(t("forgetAllQ", { n: state.closed.length }), [[t("forget"), () => invoke("forget_all_closed").then(() => refresh(true))]]));
    li.appendChild(row);
  } else {
    li.appendChild(btn(t("forgetAll"), t("forgetAll"), () => { state.confirming = "forget-all"; render(); }));
  }
  return li;
}

/** Split rows into sections (by agent or by type); headers only when there are 2+. */
function sectioned(gs) {
  const sectionOf = (g) => g.agent?.name || "";
  const out = [];
  let prev = null;
  for (const g of gs) {
    const k = sectionOf(g);
    if (prev !== null && k !== prev) out.push(el("li", "sep"));
    out.push(groupEl(g));
    prev = k;
  }
  return out;
}

/** Body classes: theme-glass / theme-dark / theme-light, plus "glass" when the window really is translucent. */
function applyTheme(s) {
  const b = document.body.classList;
  b.remove("theme-glass", "theme-dark", "theme-light", "glass");
  const t = s.theme || "glass";
  b.add(`theme-${t === "glass" && !s.glass ? "dark" : t}`);
  if (t === "glass" && s.glass) b.add("glass");
}
function segRow(label, value, options, onPick) {
  const row = el("div", "set");
  row.appendChild(el("span", "", label));
  const seg = el("span", "seg");
  for (const [v, text] of options) {
    const b = el("button", v === value ? "on" : "", text);
    b.addEventListener("click", () => onPick(v));
    seg.appendChild(b);
  }
  row.appendChild(seg);
  return row;
}
function selectRow(label, value, options, onPick) {
  const row = el("div", "set");
  row.appendChild(el("span", "", label));
  const sel = el("select", "sel");
  for (const [v, text] of options) {
    const o = el("option", "", text);
    o.value = v;
    if (v === value) o.selected = true;
    sel.appendChild(o);
  }
  sel.addEventListener("change", () => onPick(sel.value));
  row.appendChild(sel);
  return row;
}
async function setLang(value) {
  try {
    await invoke("set_lang", { value });
    state.settings.lang = value;
  } catch (e) { toast(String(e), 4000); }
  refresh(true);
}
async function setTheme(value) {
  try {
    await invoke("set_theme", { value });
    state.settings.theme = value;
    applyTheme(state.settings);
  } catch (e) { toast(String(e), 4000); }
  render();
}

function toggleRow(label, hint, on, onChange) {
  const row = el("div", "set");
  const txt = el("span", "", label);
  if (hint) txt.appendChild(el("small", "", hint));
  row.appendChild(txt);
  const sw = el("button", `sw${on ? " on" : ""}`);
  sw.title = label;
  sw.addEventListener("click", onChange);
  row.appendChild(sw);
  return row;
}
async function setSetting(name, value) {
  try {
    await invoke("set_setting", { name, value });
    state.settings[name] = value;
  } catch (e) { toast(String(e), 4000); }
  render();
  if (name === "show_all") refresh(true);
}
function renderSettings() {
  const s = state.settings;
  const box = $("#settings");
  box.replaceChildren(
    segRow(t("appearance"), s.glass ? (s.theme || "glass") : (s.theme === "light" ? "light" : "dark"),
      [...(s.glass ? [["glass", t("glass")]] : []), ["dark", t("dark")], ["light", t("light")]], setTheme),
    selectRow(t("language"), s.lang || "", LANGS.map(([v, n]) => [v, v ? n : t("system")]), setLang),
    toggleRow(t("launchAtLogin"), null, s.launch_at_login, () => setSetting("launch_at_login", !s.launch_at_login)),
    toggleRow(t("openOnStart"), t("openOnStartHint"), s.show_on_start, () => setSetting("show_on_start", !s.show_on_start)),
    toggleRow(t("showAll"), t("showAllHint"), s.show_all, () => setSetting("show_all", !s.show_all)),
    toggleRow(t("closeOnBlur"), t("closeOnBlurHint"), s.close_on_blur, () => setSetting("close_on_blur", !s.close_on_blur)),
    toggleRow(t("checkUpdates"), t("checkUpdatesHint"), s.check_updates, async () => {
      await setSetting("check_updates", !s.check_updates);
      checkUpdate();
    }),
  );
  box.appendChild(el("div", "sec", t("lockedProjects")));
  if (!s.protected.length) {
    box.appendChild(el("p", "hint", t("lockedHint")));
  } else {
    const list = el("div", "locks");
    for (const key of s.protected) {
      const lk = el("div", "lk");
      lk.appendChild(lockIco());
      const name = el("span", "", base(key.replace(/^(dir|d):/, "")));
      name.title = key.replace(/^(dir|d):/, "");
      lk.appendChild(name);
      lk.appendChild(btn(t("unlock").toLowerCase(), t("unlock"), () => toggleLock(key)));
      list.appendChild(lk);
    }
    box.appendChild(list);
  }
  const foot = el("div", "foot");
  foot.appendChild(el("span", "", `Portkeeper ${s.version || ""}`.trim()));
  foot.appendChild(btn(`☕ ${t("coffee")}`, "buymeacoffee.com/maparisidef", () => call("open_url", { url: "https://buymeacoffee.com/maparisidef" }), "coffee"));
  foot.appendChild(btn(t("quit"), t("quit"), () => invoke("quit")));
  box.appendChild(foot);
}

function render() {
  $("#min").style.display = state.settings.close_on_blur ? "none" : "";
  const settingsOpen = state.view === "settings";
  $("#settings").classList.toggle("hidden", !settingsOpen);
  $("#list").classList.toggle("hidden", settingsOpen);
  $("#settings-title").classList.toggle("hidden", !settingsOpen);
  for (const t of document.querySelectorAll(".tab")) t.classList.toggle("hidden", settingsOpen);
  $("#gear").classList.toggle("on", settingsOpen);
  if (settingsOpen) renderSettings();

  const gs = groups();
  const closed = closedRows();
  const list = $("#list");
  if (state.tab === "closed") {
    list.replaceChildren(...closed);
    if (!closed.length) list.appendChild(el("li", "empty", t("nothingClosed")));
    if (state.closed.length > 1) list.appendChild(forgetAllRow());
  } else {
    list.replaceChildren(...sectioned(gs));
    if (!gs.length) list.appendChild(el("li", "empty", t("nothingRunning")));
  }
  list.classList.toggle("fits", list.scrollHeight <= list.clientHeight + 1);

  for (const t of document.querySelectorAll(".tab")) t.classList.toggle("on", t.dataset.tab === state.tab);
  $("#n-active").textContent = gs.length;
  $("#n-closed").textContent = closed.length || "";

  invoke("set_tray_count", { count: gs.length }).catch(() => {});
  fit();
}

for (const t of document.querySelectorAll(".tab")) {
  t.addEventListener("click", () => {
    state.tab = t.dataset.tab;
    state.confirming = null;
    try { localStorage.setItem("tab", state.tab); } catch { /* private mode */ }
    render();
  });
}
$("#min").addEventListener("click", () => invoke("hide_window"));
const toggleSettings = (open = state.view !== "settings") => { state.view = open ? "settings" : "list"; state.confirming = null; render(); };
$("#gear").addEventListener("click", () => toggleSettings());

document.addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  if (state.confirming) { state.confirming = null; render(); }
  else if (state.view === "settings") toggleSettings(false);
  else invoke("hide_window");
});

// No requestAnimationFrame here: it never fires while the window is hidden or occluded.
function fit() {
  setTimeout(() => {
    const h = $("#card").getBoundingClientRect().height;
    invoke("fit_window", { height: Math.ceil(h) }).catch(() => {});
  }, 0);
}

// ---------- updates ----------
const UPDATE_EVERY = 6 * 3600 * 1000;
const UPDATE_ON_OPEN = 3600 * 1000; // opening the widget checks again once the last check is this old
let lastUpdateCheck = 0;
async function checkUpdate() {
  if (state.updating) return; // keep the bar (and its spinner) while installing
  lastUpdateCheck = Date.now();
  const info = await invoke("check_update").catch(() => null); // offline or GitHub down: try again later
  state.update = info || null;
  renderUpdate();
}
async function installUpdate() {
  state.updating = true;
  renderUpdate();
  try {
    await invoke("install_update"); // restarts Portkeeper when done
  } catch (e) {
    state.updating = false;
    renderUpdate();
    toast(String(e), 5000);
  }
}
/**
 * Release notes (the CHANGELOG.md section the release workflow publishes) as group labels
 * and bullets. Text only, never HTML; everything from the "---" separator on is skipped.
 * Returns [] for releases without bullet notes (the ones before the changelog existed).
 */
function parseNotes(body) {
  const plain = (s) => s.replace(/`([^`]*)`/g, "$1").replace(/\*\*([^*]*)\*\*/g, "$1").replace(/\[([^\]]*)\]\([^)]*\)/g, "$1").trim();
  const items = [];
  for (const raw of String(body || "").split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "---") break;
    if (/^###\s+/.test(line)) items.push({ group: plain(line.replace(/^###\s+/, "")) });
    else if (/^[-*]\s+/.test(line)) items.push({ text: plain(line.replace(/^[-*]\s+/, "")) });
  }
  return items.some((i) => i.text) ? items : [];
}
/** A slim bar under the header while a newer version is available; "What's new" unfolds the notes. */
function renderUpdate() {
  const bar = $("#update");
  bar.classList.toggle("hidden", !state.update);
  if (!state.update) return fit();
  const { version, notes } = state.update;
  const items = parseNotes(notes);
  const row = el("div", "urow");
  const msg = el("span", "msg", t("updateAvailable", { v: version }));
  if (items.length) {
    // The message itself unfolds the notes (a second button would not fit next to Update).
    msg.classList.add("more");
    msg.title = t(state.showNotes ? "hide" : "whatsNew");
    msg.addEventListener("click", () => { state.showNotes = !state.showNotes; renderUpdate(); });
  } else if (notes) {
    msg.title = notes; // old-style notes: tooltip only
  }
  bar.classList.toggle("open", items.length > 0 && state.showNotes);
  row.appendChild(msg);
  row.appendChild(state.updating ? el("span", "spin") : btn(t("update"), t("update"), installUpdate, "primary"));
  bar.replaceChildren(row);
  if (items.length && state.showNotes) {
    const list = el("ul", "notes");
    for (const i of items) list.appendChild(el("li", i.group ? "group" : "", i.group || i.text));
    bar.appendChild(list);
    bar.appendChild(btn(t("allChanges"), t("allChanges"), () => call("open_url", { url: `https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v${version}` }), "link"));
  }
  fit();
}

(async function init() {
  await refresh(true);
  setInterval(() => refresh(), 3000);
  checkUpdate();
  setInterval(checkUpdate, UPDATE_EVERY);
  window.__TAURI__.event.listen("widget-shown", () => {
    if (Date.now() - lastUpdateCheck > UPDATE_ON_OPEN) checkUpdate();
  });
})();
