# Changelog

What changed for people using Portkeeper, newest first. The section for a version becomes
the notes of its GitHub release and the text the app shows when it offers that update.
Add your change under **Unreleased** in the same pull request that makes it.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed
- Opening the widget checks for updates again when the last check was more than an hour ago, instead of waiting up to 6 hours.

## [0.5.0] - 2026-10-16

### Added
- **What's new**: when an update is available, click the message to read what changed before installing.
- **Forget all** under the Closed list, to clear it in one click instead of one row at a time.
- One-command install for macOS, Linux and Windows from the website, which now also shows what's new in the latest version.

### Changed
- The widget opens centred under the tray icon on setups with screens of different scale (a Retina laptop plus a 1x monitor), and in the right place right after launch.
- Rows and cards no longer jump on hover.
- Linux: the widget opens in the top-right corner on desktops with a top panel (GNOME, Ubuntu, elementary), instead of the bottom-right.
- Windows: with the taskbar on the left or right, the widget opens beside it instead of on top of it.
- Portkeeper is now fully open source under the MIT license.

### Security
- **Start again** runs the saved command as separate arguments, never through a shell, and shows exactly what will run.
- Stopping a process only works for processes shown in the list; nothing else can be signalled.
- Opening a port only ever opens `http://localhost:<port>` in your browser.
- Passwords inside connection URLs (`postgres://user:password@…`) are masked and never saved.
- A content security policy for the widget, and stricter checks on Docker container names.
- The install scripts refuse to run a half-downloaded copy, pin HTTPS, swap the app in safely, and check the release signature when `minisign` is installed.
- Releases are built by pinned, reviewed CI steps, with the signing key only available in a protected environment.

## [0.4.1] - 2026-10-15

### Fixed
- macOS no longer reports the downloaded app as "damaged": the whole bundle is signed, so Gatekeeper offers **Open Anyway** instead.

## [0.4.0] - 2026-10-15

### Added
- In-app updates: Portkeeper checks GitHub for new signed releases and installs them with one click. A **Check for updates** setting turns this off.

## [0.3.0] - 2026-10-15

### Added
- Stop progress: rows show "stopping…" until the process or container is really gone.
- Drawn action icons instead of text glyphs, which rendered hairline-thin on Windows.
- Buy me a coffee link in Settings.

### Changed
- Windows: opaque window with native rounded corners, no glass effect.
- Docker actions no longer prompt.

## [0.2.0] - 2026-10-15

### Added
- First public release: every listening port and Docker container, grouped by project, with the logo of the agent or IDE that started it. Open, stop or kill with one click. Closed tab to start projects again. Project locks. Five languages.

[Unreleased]: https://github.com/MarcAparisiTFG/portkeeper/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/MarcAparisiTFG/portkeeper/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/MarcAparisiTFG/portkeeper/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/MarcAparisiTFG/portkeeper/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MarcAparisiTFG/portkeeper/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v0.2.0
