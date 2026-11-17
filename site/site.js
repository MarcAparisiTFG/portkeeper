// Light / dark: the system decides unless the toggle saved a choice. The demo app follows.
(function () {
  const root = document.documentElement;
  const media = matchMedia("(prefers-color-scheme: dark)");
  const app = document.getElementById("app");
  const mode = () => root.dataset.theme || (media.matches ? "dark" : "light");
  function sync() {
    const m = mode();
    root.dataset.mode = m;
    if (app.contentWindow && app.src) app.contentWindow.postMessage({ portkeeper: "theme", value: m }, "*");
  }
  // Load the demo already in the right theme (no flash).
  app.src = app.dataset.src + "&theme=" + mode();
  app.addEventListener("load", sync);
  media.addEventListener("change", sync);
  document.getElementById("theme").addEventListener("click", () => {
    const next = mode() === "dark" ? "light" : "dark";
    // Back to following the system when the choice matches it.
    if (next === (media.matches ? "dark" : "light")) delete root.dataset.theme;
    else root.dataset.theme = next;
    try {
      if (root.dataset.theme) localStorage.setItem("theme", root.dataset.theme);
      else localStorage.removeItem("theme");
    } catch (e) { /* private mode */ }
    sync();
  });
  sync();
})();

// The embedded app reports its height; size the glass to it.
window.addEventListener("message", (e) => {
  const d = e.data;
  if (d && d.portkeeper === "height" && Number.isFinite(d.value)) {
    const g = document.getElementById("glass");
    g.style.setProperty("--h", Math.min(Math.max(d.value, 60), 600) + "px");
    if (d.theme) g.dataset.theme = d.theme; // light: white panel, like the app
  }
});

// Highlight the visitor's platform, show the latest version and the star count.
(function () {
  const REPO = "MarcAparisiTFG/portkeeper";
  const ua = navigator.userAgent;
  const os = /Windows/i.test(ua) ? "win" : /Mac/i.test(navigator.platform || ua) ? "mac"
    : /Linux/i.test(ua) ? (/aarch64|arm64|armv8/i.test(ua + " " + (navigator.platform || "")) ? "linux-arm" : "linux") : null;
  const here = os && document.getElementById(os);
  if (here) here.classList.add("here");

  // Installer links use releases/latest/download/<version-free name>; only the label needs the version.
  fetch("https://api.github.com/repos/" + REPO + "/releases/latest")
    .then((r) => (r.ok ? r.json() : null))
    .then((rel) => {
      if (!rel || !rel.tag_name) return;
      document.getElementById("ver").textContent = rel.tag_name;
      renderNews(rel.tag_name, rel.body, rel.html_url);
    })
    .catch(() => {});
  // Star count.
  fetch("https://api.github.com/repos/" + REPO)
    .then((r) => (r.ok ? r.json() : null))
    .then((repo) => {
      if (!repo || typeof repo.stargazers_count !== "number") return;
      const el = document.getElementById("stars");
      const n = repo.stargazers_count;
      el.textContent = n >= 1000 ? (n / 1000).toFixed(1).replace(/\.0$/, "") + "k" : String(n);
      el.hidden = false;
    })
    .catch(() => {});
})();

/**
 * "What's new": the latest release notes (the CHANGELOG.md section the release workflow
 * publishes). Only text is used, never HTML: "### Group" lines become small labels,
 * "- item" lines become bullets, everything from the "---" separator on is skipped.
 * Releases without bullet lines (the ones before the changelog existed) show nothing.
 */
function renderNews(tag, body, url) {
  const plain = (s) => s.replace(/`([^`]*)`/g, "$1").replace(/\*\*([^*]*)\*\*/g, "$1").replace(/\[([^\]]*)\]\([^)]*\)/g, "$1").trim();
  const items = [];
  for (const raw of String(body || "").split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "---") break;
    if (/^###\s+/.test(line)) items.push({ group: plain(line.replace(/^###\s+/, "")) });
    else if (/^[-*]\s+/.test(line)) items.push({ text: plain(line.replace(/^[-*]\s+/, "")) });
  }
  if (!items.some((i) => i.text)) return;
  const list = document.getElementById("news-list");
  list.replaceChildren();
  for (const i of items) {
    const li = document.createElement("li");
    if (i.group) li.className = "group";
    li.textContent = i.group || i.text;
    list.appendChild(li);
  }
  document.getElementById("news-ver").textContent = tag;
  if (url) document.getElementById("news-release").href = url;
  document.getElementById("news").hidden = false;
}

// Copy an install command.
for (const b of document.querySelectorAll(".cli .copy")) {
  b.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(b.previousElementSibling.textContent);
      b.textContent = "Copied";
      setTimeout(() => (b.textContent = "Copy"), 1500);
    } catch { /* clipboard blocked: the text is selectable */ }
  });
}
