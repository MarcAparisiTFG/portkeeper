> [!WARNING]
> **Not verified by Apple or Microsoft yet.** Portkeeper has no developer certificate for now, so macOS and Windows show a one-time warning when you first open it. That is expected and the app is safe: every build comes from the public code in this repository through GitHub Actions. Two clicks get past the warning (see [Install](#install)), or use the one-command install below, which shows no dialog at all. The warning disappears once the app is notarized.

<p align="center">
  <img src="docs/active.png" width="370" alt="Portkeeper: projects, their ports, the agent that started them, and a status dot for each" />
</p>

<h1 align="center">Portkeeper</h1>

<p align="center">
  A tiny desktop widget that answers one question: <b>what is running on my machine right now, who started it, and how do I stop it?</b>
</p>

<p align="center">
  <a href="https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-macOS-AppleSilicon.dmg"><img src="docs/download-macos-arm.svg" height="60" alt="Download for macOS, Apple Silicon" /></a>
  <a href="https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-macOS-Intel.dmg"><img src="docs/download-macos-intel.svg" height="60" alt="Download for macOS, Intel" /></a>
  <br />
  <a href="https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-Windows-x64-setup.exe"><img src="docs/download-windows.svg" height="60" alt="Download for Windows" /></a>
  <a href="https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-Linux-x86_64.AppImage"><img src="docs/download-linux.svg" height="60" alt="Download for Linux" /></a>
</p>

> **Fastest install: one command.** It downloads the latest release from GitHub, installs it and opens it. No "unidentified developer" dialog (the builds aren't signed with an Apple or Microsoft certificate yet, see [Install](#install)).
>
> macOS and Linux, in Terminal:
> ```bash
> curl -fsSL https://portkeeper.pages.dev/install.sh | sh
> ```
> Windows, in PowerShell:
> ```powershell
> irm https://portkeeper.pages.dev/install.ps1 | iex
> ```
> [Read the scripts](site/) before you run them, as with any installer.

<p align="center">
  <a href="https://github.com/MarcAparisiTFG/portkeeper/stargazers"><img src="https://img.shields.io/github/stars/MarcAparisiTFG/portkeeper?style=social" alt="GitHub stars" /></a>
  &nbsp;
  <a href="https://buymeacoffee.com/maparisidef"><img src="https://img.shields.io/badge/Buy_me_a_coffee-FFDD00?style=flat&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee" /></a>
</p>

<p align="center">
  <a href="https://portkeeper.pages.dev">Website</a> ·
  <a href="https://github.com/MarcAparisiTFG/portkeeper/releases/latest">All downloads</a> ·
  <a href="#what-it-shows">What it shows</a> ·
  <a href="#install">Install</a> ·
  <a href="#build-from-source">Build</a>
</p>

---

## Why

If you work with several projects at once, and let AI coding agents start servers for you, your machine fills up with `localhost:3000`, `:5173`, `:8080`, Docker containers and forgotten dev servers. Finding out which one belongs to which project, which agent opened it, and whether it still answers means `lsof`, `docker ps` and a lot of guessing.

Portkeeper puts that in one small panel in your menu bar (macOS) or system tray (Windows): every project with a listening port, grouped by folder, with the logo of the agent or IDE that started it. Click a port to open it. Stop it with one click. Start it again later. Lock the ones a client depends on so nobody stops them by mistake.

## What it shows

<p align="center">
  <img src="docs/expanded.png" width="300" alt="A project with two ports expanded, each with its own label" />
  &nbsp;&nbsp;
  <img src="docs/light.png" width="300" alt="Light theme" />
</p>

- **One row per project.** Processes are grouped by their working directory and merged with the Docker Compose project running from the same folder. A project with several ports expands into one row per port, each with its own label (`Vite`, `dashboard/server.mjs`).
- **Who started it.** The logo is the AI agent or IDE that launched the process, found by walking up the process tree: Claude Code, Codex, Cursor, Gemini CLI, Copilot, Windsurf, VS Code, JetBrains, Zed, Xcode, Cline, Aider, OpenCode. Without one, it shows what serves the port: Docker, Vite, Next, Node, Python, Ollama, Postgres…
- **Does it answer?** The dot is green when the port responds, amber while it is listening but still silent (booting, or hung). Hover it for "Running since…".
- **Sections.** Projects are separated by the agent that started them. Agents first, the rest after.

<p align="center">
  <img src="docs/closed.png" width="300" alt="Closed tab with projects that can be started again" />
  &nbsp;&nbsp;
  <img src="docs/settings.png" width="300" alt="Settings: appearance, language, launch at login, locked projects" />
</p>

- **Stop.** The stop button on a row stops that port, or the whole project, after a one-button confirmation. While it works, the row shows *stopping…*. Portkeeper asks the process politely first and kills it if it ignores the request for 1.5 s, so Stop always stops.
- **Closed, and start again.** Projects that stopped move to *Closed*. Portkeeper remembers the command that started them (`npm start`, `python -m http.server`, a compose project), the folder and the environment, and the play button starts them again. Secrets in command lines (`--token`, `API_KEY=`) are masked and never written to disk.
- **Lock.** 🔒 on a project's row hides stop and remove and makes the backend refuse them too. Made for the project a client is testing on your machine.
- **Settings.** Appearance (Glass on macOS: Liquid Glass on macOS 26, the system blur before; opaque Dark or Light everywhere, with native rounded corners on Windows 11), language (English, Spanish, French, German, Portuguese), launch at login, open on start, show all processes, check for updates.

Portkeeper works entirely on your machine. It reads the process table and talks to the local `docker` command. It connects to `localhost` to see whether a port answers, and to GitHub every few hours to look for a new version (turn this off in Settings). No account, no telemetry.

## Install

Download the file for your system. These links always point to the latest version:

| System | Download |
| --- | --- |
| macOS, Apple Silicon (M1 and later) | [Portkeeper-macOS-AppleSilicon.dmg](https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-macOS-AppleSilicon.dmg) |
| macOS, Intel | [Portkeeper-macOS-Intel.dmg](https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-macOS-Intel.dmg) |
| Windows 10 / 11 | [Portkeeper-Windows-x64-setup.exe](https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-Windows-x64-setup.exe) |
| Linux | [Portkeeper-Linux-x86_64.AppImage](https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-Linux-x86_64.AppImage) |

The `.msi`, `.deb` and `.rpm` packages are on the [releases page](https://github.com/MarcAparisiTFG/portkeeper/releases/latest).

The installers are not signed with an Apple or Microsoft developer certificate yet, so your system warns you once:

- **The easy way, any system:** one command, see the box at the top of this page. It downloads the latest release from GitHub and installs it with no dialog: files fetched by `curl` or PowerShell and unblocked carry no "downloaded from the internet" mark. On Linux it puts the AppImage in `~/.local/bin/portkeeper` and adds a menu entry. Updates then come from inside the app.
- **macOS, with the `.dmg`:** drag Portkeeper to Applications. Open it once: macOS says it *could not verify* the app. Click **Done**, then go to **System Settings → Privacy & Security**, scroll down and click **Open Anyway**. On macOS 14 and older, right-click the app and choose **Open** instead.
  If macOS calls the app *damaged* (it does with version 0.4.0), run this once in Terminal and open it again:

  ```bash
  xattr -dr com.apple.quarantine /Applications/Portkeeper.app
  ```

- **Windows:** when SmartScreen appears, click **More info**, then **Run anyway**.

Containers are listed when the `docker` command is available (Docker Desktop, OrbStack, Colima, Rancher Desktop…). Without it Portkeeper simply lists processes and ports, with no prompts.

## Updates

From v0.4.0, Portkeeper keeps itself up to date. It checks GitHub for a new version when it starts and every six hours. When there is one, a bar at the top of the widget says so; **Update** downloads it, checks its signature, installs it and restarts the app. Updates are signed with the project's own key (separate from the developer certificates above), and the app refuses any file whose signature doesn't match. Turn the check off in Settings with *Check for updates*.

Versions before 0.4.0 can't update themselves: download the latest version once from the links above.

## Use

- Click the menu bar / tray icon to open the widget; the minimise button or `Esc` tucks it away. Right-click the icon to quit.
- Click a port number to open `http://localhost:PORT`. Click a project with several ports to expand it.
- Hover a row for its actions: lock, stop. In *Closed*: start again, forget.

## Build from source

You need [Rust](https://rustup.rs), Node.js 22+, [pnpm](https://pnpm.io/installation) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system.

```bash
pnpm install
pnpm dev      # run the app
pnpm build    # installers in src-tauri/target/release/bundle/
```

- `src-tauri/` is the Rust side: `ports.rs` (listening sockets, process tree, launch commands), `docker.rs`, `probe.rs` (does the port answer), `launch.rs` (Closed list and relaunch), `settings.rs`, `lib.rs` (tray, window, Liquid Glass, native animations).
- `src/` is a plain HTML, CSS and JS front-end with no bundler. Serve it with `python3 -m http.server -d src 4173` to work on the UI without Rust: `src/mock.js` feeds it sample data.
- `bash assets/screenshots.sh` regenerates the screenshots in `docs/` from the UI with sample data (needs Google Chrome); `node assets/make_badges.js` redraws the download buttons.
- `cargo test` in `src-tauri/` runs the unit tests; `cargo test -- --ignored` adds the end-to-end ones that start a throwaway `python3` server, stop it and start it again.
- `portkeeper --dump` prints the raw scan as JSON, useful when reporting a platform-specific bug.

`site/` is the one-page website. Every merge to `main` that changes `site/` or `src/` deploys it to Cloudflare Pages (`.github/workflows/site.yml`), with the app UI copied in as the live demo. To deploy by hand: `pnpm run deploy:site` (run `pnpm dlx wrangler@4.151.0 login` once first).

Releases take two commands:

```bash
scripts/release.sh 0.6.0         # opens the "Release 0.6.0" pull request
scripts/release.sh --tag 0.6.0   # after it is merged: tags main, starts the release
```

The first moves the **Unreleased** notes in [CHANGELOG.md](CHANGELOG.md) under the new version (that section becomes the release notes and the text the app shows when it offers the update) and sets the version in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `Cargo.lock`. The second refuses to tag until that version is merged on `main`. Then approve the `release` deployment in Actions. The workflow signs the update bundles with the `TAURI_SIGNING_PRIVATE_KEY` secret and publishes `latest.json`, which installed copies read to find the update.

## Contributing

Pull requests are welcome: bug fixes, support for more agents and IDEs, translations, platform fixes. Open an issue first for bigger ideas. Every pull request is reviewed by the maintainer before it is merged; see [CONTRIBUTING.md](CONTRIBUTING.md).

## Maintainer

<img src="https://github.com/MarcAparisiTFG.png?size=80" width="40" align="left" style="border-radius:50%" alt="" />

Maintained by **Marc Aparisi Rull**, also behind [Illeta Digital](https://illetadigital.com).<br clear="left" />

If Portkeeper saves you some `lsof` time, you can [buy me a coffee](https://buymeacoffee.com/maparisidef) ☕

[LinkedIn](https://www.linkedin.com/in/marc-aparisi-rull-160bb71bb/) ·
[Instagram](https://www.instagram.com/maparisi_dev/) ·
[YouTube](https://www.youtube.com/@maparisi_dev) ·
[TikTok](https://www.tiktok.com/@maparisi_dev) ·
[Facebook](https://www.facebook.com/profile.php?id=61594776640153) ·
[GitHub](https://github.com/MarcAparisiTFG?tab=repositories) ·
[Buy me a coffee](https://buymeacoffee.com/maparisidef)

## License

Portkeeper is fully open source under the [MIT License](LICENSE). You can use it, change it and share it, including commercially, as long as you keep the copyright notice.

> **A note on the license change:** earlier versions were published under the PolyForm Noncommercial License, which I called "open source" at the time. I didn't fully understand the concept yet: a license that forbids commercial use isn't open source by the [Open Source Definition](https://opensource.org/osd). Portkeeper is now MIT, so it really is open source. Releases published before this change keep the license they shipped with.

Logos come from [Simple Icons](https://simpleicons.org) (CC0) and belong to their respective owners.
