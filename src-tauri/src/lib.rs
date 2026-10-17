mod docker;
mod launch;
mod placement;
mod ports;
mod probe;
mod settings;
mod system;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Monitor, PhysicalPosition, Rect,
    WebviewWindow,
};
use tauri_plugin_autostart::ManagerExt as _;

const WIDTH: f64 = 290.0;
const MAX_HEIGHT: f64 = 520.0;

/// True when the widget sits above the tray icon (Windows taskbar at the bottom),
/// so it must grow upwards when its content gets taller.
struct GrowUp(AtomicBool);

#[derive(serde::Serialize)]
struct SettingsDto {
    /// The window has a translucent system backdrop (Liquid Glass / blur / acrylic).
    glass: bool,
    theme: String,
    lang: String,
    show_on_start: bool,
    show_all: bool,
    protected: Vec<String>,
    launch_at_login: bool,
    check_updates: bool,
    version: String,
}

type SettingsState = Mutex<settings::Store>;

/// PIDs in the last `list_ports` scan: the only ones the web view may stop.
#[derive(Default)]
struct ScannedPids(Mutex<std::collections::HashSet<u32>>);

const LOCKED: &str = "This project is locked in Portkeeper. Unlock it to stop it.";

// ---------- commands (called from the web view) ----------

/// A newer version published on GitHub (see `plugins.updater` in tauri.conf.json).
#[derive(serde::Serialize)]
struct UpdateInfo {
    version: String,
    notes: Option<String>,
}

/// Ask GitHub for a newer signed release. `None` when up to date or turned off.
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let enabled = app
        .state::<SettingsState>()
        .lock()
        .unwrap()
        .value
        .check_updates;
    if !enabled {
        return Ok(None);
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo {
        version: u.version,
        notes: u.body,
    }))
}

/// Download, verify the signature, install and restart.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Portkeeper is up to date")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

#[tauri::command]
async fn list_ports(app: AppHandle) -> Result<Vec<ports::PortInfo>, String> {
    let list = ports::list()?;
    *app.state::<ScannedPids>().0.lock().unwrap() = list.iter().map(|p| p.pid).collect();
    if let Some(t) = app.try_state::<Mutex<launch::Tracker>>() {
        t.lock().unwrap().update(&list);
    }
    Ok(list)
}

/// Projects Portkeeper saw running that have since closed (newest first).
#[tauri::command]
fn list_closed(app: AppHandle) -> Vec<launch::ClosedView> {
    app.try_state::<Mutex<launch::Tracker>>()
        .map(|t| t.lock().unwrap().closed())
        .unwrap_or_default()
}

/// Start a closed project again with the command, folder and environment it had.
#[tauri::command]
fn relaunch(app: AppHandle, key: String) -> Result<String, String> {
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    let t = app.state::<Mutex<launch::Tracker>>();
    let log = t.lock().unwrap().relaunch(&key, &log_dir)?;
    Ok(system::tilde(&log.to_string_lossy()))
}

#[tauri::command]
fn forget_closed(app: AppHandle, key: String) {
    app.state::<Mutex<launch::Tracker>>()
        .lock()
        .unwrap()
        .forget(&key);
}

/// Empty the Closed list in one go ("Forget all").
#[tauri::command]
fn forget_all_closed(app: AppHandle) {
    app.state::<Mutex<launch::Tracker>>()
        .lock()
        .unwrap()
        .forget_all();
}

/// True when the process runs from a locked project folder.
fn pid_is_locked(store: &settings::Store, pid: u32) -> bool {
    ports::cwd_of(pid).is_some_and(|cwd| store.is_protected(&settings::key_for_cwd(&cwd)))
}

#[tauri::command]
async fn kill_process(app: AppHandle, pid: u32, force: bool) -> Result<(), String> {
    if !app.state::<ScannedPids>().0.lock().unwrap().contains(&pid) {
        return Err("That process is no longer in the list.".into());
    }
    // Enforced here too, not only in the UI.
    if pid_is_locked(&app.state::<SettingsState>().lock().unwrap(), pid) {
        return Err(LOCKED.into());
    }
    ports::kill(pid, force)
}

#[tauri::command]
async fn list_containers() -> docker::DockerStatus {
    docker::list()
}

#[tauri::command]
async fn docker_action(app: AppHandle, id: String, action: String) -> Result<(), String> {
    if action != "start" {
        if let Some(key) = docker::project_key(&id) {
            if app
                .state::<SettingsState>()
                .lock()
                .unwrap()
                .is_protected(&key)
            {
                return Err(LOCKED.into());
            }
        }
    }
    docker::action(&id, &action)
}

#[tauri::command]
async fn docker_logs(name: String) -> Result<(), String> {
    docker::open_logs(&name)
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !is_allowed_url(&url) {
        return Err("only local http://localhost:<port> URLs can be opened".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
}

const DONATE_URL: &str = "https://buymeacoffee.com/maparisidef";
const RELEASE_URL: &str = "https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v";

/// The UI only ever opens a local port, the donation page or one of Portkeeper's own
/// release pages; anything else is refused, so a compromised web view cannot send the
/// browser to an arbitrary site.
fn is_allowed_url(url: &str) -> bool {
    if url == DONATE_URL {
        return true;
    }
    if let Some(version) = url.strip_prefix(RELEASE_URL) {
        // Only a version after "tag/v": no "..", query or fragment that could point elsewhere.
        return !version.is_empty()
            && version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'));
    }
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let Some((host, port)) = authority.rsplit_once(':') else {
        return false;
    };
    matches!(host, "localhost" | "127.0.0.1" | "[::1]") && port.parse::<u16>().is_ok()
}

#[cfg(test)]
mod url_tests {
    use super::is_allowed_url;

    #[test]
    fn only_local_ports_and_the_donation_page() {
        for ok in [
            "http://localhost:3000",
            "http://localhost:3000/",
            "http://127.0.0.1:8080/path?x=1",
            "http://[::1]:5173",
            super::DONATE_URL,
            "https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v0.5.0",
            "https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v1.0.0-beta.1",
        ] {
            assert!(is_allowed_url(ok), "{ok}");
        }
        for bad in [
            "https://evil.example",
            "http://localhost.evil.example:80",
            "http://localhost:3000@evil.example/",
            "http://localhost",
            "http://localhost:abc",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://buymeacoffee.com/someone-else",
            "https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v",
            "https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v0.5.0/../../../evil",
            "https://github.com/MarcAparisiTFG/portkeeper/releases/tag/v0.5.0?x=1",
            "https://github.com/MarcAparisiTFG/portkeeper/releases",
            "https://github.com/someone-else/portkeeper/releases/tag/v0.5.0",
        ] {
            assert!(!is_allowed_url(bad), "{bad}");
        }
    }
}

/// Which of these ports answer right now. `ports` = [[port, tech], …].
#[tauri::command]
async fn probe_ports(ports: Vec<(u16, Option<String>)>) -> std::collections::HashMap<u16, bool> {
    probe::probe_all(&ports)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> SettingsDto {
    let s = app.state::<SettingsState>().lock().unwrap().value.clone();
    SettingsDto {
        glass: GLASS.load(Ordering::Relaxed),
        theme: s.theme,
        lang: s.lang,
        show_on_start: s.show_on_start,
        show_all: s.show_all,
        protected: s.protected,
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
        check_updates: s.check_updates,
        version: app.package_info().version.to_string(),
    }
}

/// Change one setting from the settings panel.
#[tauri::command]
fn set_setting(app: AppHandle, name: String, value: bool) -> Result<(), String> {
    if name == "launch_at_login" {
        let l = app.autolaunch();
        return if value { l.enable() } else { l.disable() }.map_err(|e| e.to_string());
    }
    let state = app.state::<SettingsState>();
    let mut store = state.lock().unwrap();
    match name.as_str() {
        "show_on_start" => store.value.show_on_start = value,
        "show_all" => store.value.show_all = value,
        "check_updates" => store.value.check_updates = value,
        _ => return Err(format!("unknown setting {name}")),
    }
    store.save();
    Ok(())
}

#[tauri::command]
fn set_theme(app: AppHandle, value: String) -> Result<(), String> {
    if !["glass", "dark", "light"].contains(&value.as_str()) {
        return Err(format!("unknown theme {value}"));
    }
    let state = app.state::<SettingsState>();
    let mut store = state.lock().unwrap();
    store.value.theme = value;
    store.save();
    Ok(())
}

#[tauri::command]
fn set_lang(app: AppHandle, value: String) -> Result<(), String> {
    if !["", "en", "es", "fr", "de", "pt"].contains(&value.as_str()) {
        return Err(format!("unknown language {value}"));
    }
    let state = app.state::<SettingsState>();
    let mut store = state.lock().unwrap();
    store.value.lang = value;
    store.save();
    Ok(())
}

/// Lock / unlock a project (by its row key).
#[tauri::command]
fn toggle_protected(app: AppHandle, key: String) -> Vec<String> {
    let state = app.state::<SettingsState>();
    let mut store = state.lock().unwrap();
    store.toggle_protected(&key);
    store.value.protected.clone()
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// The page reports its content height; the window shrinks or grows to fit it.
/// Anchored above a bottom tray, it keeps its bottom edge in place.
#[tauri::command]
fn fit_window(app: AppHandle, height: f64) -> Result<(), String> {
    let win = app.get_webview_window("main").ok_or("no window")?;
    let h = height.clamp(40.0, MAX_HEIGHT);
    refresh_shadow_later(&app);
    let grow_up = app.state::<GrowUp>().0.load(Ordering::Relaxed);
    let before = (win.outer_position(), win.outer_size());
    win.set_size(LogicalSize::new(WIDTH, h))
        .map_err(|e| e.to_string())?;
    if let (true, (Ok(pos), Ok(old)), Ok(new)) = (grow_up, before, win.outer_size()) {
        let y = pos.y + old.height as i32 - new.height as i32;
        let _ = win.set_position(PhysicalPosition::new(pos.x, y));
    }
    Ok(())
}

/// Hide the widget into the menu bar / tray icon.
#[tauri::command]
fn hide_window(app: AppHandle) {
    hide(&app);
}

/// Show the number of running projects next to the menu bar icon (macOS) / in the tooltip.
#[tauri::command]
fn set_tray_count(app: AppHandle, count: u32) {
    if let Some(tray) = app.tray_by_id("main") {
        #[cfg(target_os = "macos")]
        let _ = tray.set_title(Some(if count > 0 {
            count.to_string()
        } else {
            String::new()
        }));
        let _ = tray.set_tooltip(Some(format!("Portkeeper · {count} running")));
    }
}

// ---------- window plumbing ----------

/// True once the window has a Liquid Glass background (macOS 26+).
static GLASS: AtomicBool = AtomicBool::new(false);

/// macOS 26: put an NSGlassEffectView (Liquid Glass) behind the web view.
/// Returns false on older systems, where the blur material is used instead.
#[cfg(target_os = "macos")]
fn liquid_glass(win: &WebviewWindow) -> bool {
    use objc2::runtime::AnyClass;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSAutoresizingMaskOptions, NSGlassEffectView, NSGlassEffectViewStyle, NSWindow,
        NSWindowOrderingMode,
    };
    if AnyClass::get(c"NSGlassEffectView").is_none() {
        return false;
    }
    let (Ok(ptr), Some(mtm)) = (win.ns_window(), MainThreadMarker::new()) else {
        return false;
    };
    // SAFETY: live NSWindow from Tauri, main thread.
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    let Some(content) = ns.contentView() else {
        return false;
    };
    let glass: objc2::rc::Retained<NSGlassEffectView> =
        unsafe { objc2::msg_send![NSGlassEffectView::alloc(mtm), initWithFrame: content.bounds()] };
    glass.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    glass.setCornerRadius(14.0);
    glass.setStyle(NSGlassEffectViewStyle::Regular);
    content.addSubview_positioned_relativeTo(&glass, NSWindowOrderingMode::Below, None);
    // The panel must be see-through outside the rounded glass, or its square
    // frame shows at the corners and casts a square shadow.
    ns.setOpaque(false);
    ns.setBackgroundColor(Some(&objc2_app_kit::NSColor::clearColor()));
    content.setWantsLayer(true);
    unsafe {
        let layer: *mut objc2::runtime::AnyObject = objc2::msg_send![&*content, layer];
        if !layer.is_null() {
            let _: () = objc2::msg_send![layer, setCornerRadius: 14.0f64];
            let _: () = objc2::msg_send![layer, setMasksToBounds: true];
        }
    }
    ns.invalidateShadow();
    true
}

fn apply_glass(win: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
        if liquid_glass(win) {
            GLASS.store(true, Ordering::Relaxed);
            return;
        }
        let _ = apply_vibrancy(
            win,
            NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            Some(16.0),
        );
    }
    // Windows: no glass. The window is opaque (tauri.windows.conf.json) and
    // Windows 11 rounds it natively; see `windows_frame`.
    #[cfg(target_os = "windows")]
    windows_frame(win);
    #[cfg(target_os = "linux")]
    {
        let _ = win;
    }
}

/// Windows 11: round the borderless window natively and drop the 1px white
/// border Windows draws around undecorated windows that have a shadow.
/// Both calls are ignored on Windows 10.
#[cfg(target_os = "windows")]
fn windows_frame(win: &WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };
    const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
    let Ok(hwnd) = win.hwnd() else { return };
    let corner = DWMWCP_ROUND;
    // SAFETY: valid HWND owned by this process; pointers to live locals of the stated size.
    unsafe {
        DwmSetWindowAttribute(
            hwnd.0 as _,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );
        DwmSetWindowAttribute(
            hwnd.0 as _,
            DWMWA_BORDER_COLOR as u32,
            &DWMWA_COLOR_NONE as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(WidgetPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })
}

/// macOS: a non-activating NSPanel. It opens on the desktop you are on (also over
/// full-screen apps) and stays on that desktop instead of following you around.
#[cfg(target_os = "macos")]
fn make_panel(win: &WebviewWindow) -> tauri::Result<()> {
    use tauri_nspanel::{CollectionBehavior, StyleMask, WebviewWindowExt};
    let panel = win.to_panel::<WidgetPanel>()?;
    panel.set_level(25); // NSStatusWindowLevel
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .move_to_active_space()
            .full_screen_auxiliary()
            .ignores_cycle()
            .into(),
    );
    let _ = panel.add_style_mask(StyleMask::empty().nonactivating_panel().into());
    panel.set_hides_on_deactivate(false);
    Ok(())
}

fn is_visible(app: &AppHandle) -> bool {
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(p) = app.get_webview_panel("main") {
            return p.as_panel().isVisible();
        }
    }
    app.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

// ---------- open / close, with the platform's own window animation ----------

/// macOS: recompute the window shadow from what is actually drawn (rounded glass).
#[cfg(target_os = "macos")]
fn refresh_shadow_later(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        let a = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(w) = a.get_webview_window("main") {
                if let Ok(ptr) = w.ns_window() {
                    let ns: &objc2_app_kit::NSWindow =
                        unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
                    ns.invalidateShadow();
                }
            }
        });
    });
}
#[cfg(not(target_os = "macos"))]
fn refresh_shadow_later(_app: &AppHandle) {}

/// macOS: Core Animation moves the panel's alpha and frame (NSAnimationContext),
/// the same engine the system uses for its popovers.
#[cfg(target_os = "macos")]
fn native_show(app: &AppHandle, win: &WebviewWindow) -> bool {
    use objc2::msg_send;
    use objc2_app_kit::{NSAnimationContext, NSWindow};
    use tauri_nspanel::ManagerExt;
    let Ok(ptr) = win.ns_window() else {
        return false;
    };
    let Ok(panel) = app.get_webview_panel("main") else {
        return false;
    };
    // SAFETY: Tauri hands us the live NSWindow; we are on the main thread.
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    let target = ns.frame();
    let mut start = target;
    // Start a few points towards the icon: up when under the menu bar, down above a taskbar.
    let up = app.state::<GrowUp>().0.load(Ordering::Relaxed);
    start.origin.y += if up { -6.0 } else { 6.0 };
    ns.setAlphaValue(0.0);
    ns.setFrame_display(start, false);
    panel.show();
    unsafe {
        NSAnimationContext::beginGrouping();
        NSAnimationContext::currentContext().setDuration(anim_secs(true));
        let animator: *mut NSWindow = msg_send![ns, animator];
        (*animator).setAlphaValue(1.0);
        (*animator).setFrame_display(target, true);
        NSAnimationContext::endGrouping();
    }
    refresh_shadow_later(app);
    true
}

#[cfg(target_os = "macos")]
fn native_hide(app: &AppHandle, win: &WebviewWindow) -> bool {
    use objc2::msg_send;
    use objc2_app_kit::{NSAnimationContext, NSWindow};
    let Ok(ptr) = win.ns_window() else {
        return false;
    };
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    let secs = anim_secs(false);
    unsafe {
        NSAnimationContext::beginGrouping();
        NSAnimationContext::currentContext().setDuration(secs);
        let animator: *mut NSWindow = msg_send![ns, animator];
        (*animator).setAlphaValue(0.0);
        NSAnimationContext::endGrouping();
    }
    // Take it off screen once the fade is done, then restore alpha for the next show.
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs_f64(secs + 0.02));
        let a = app.clone();
        let _ = app.run_on_main_thread(move || {
            use tauri_nspanel::ManagerExt;
            if let Ok(p) = a.get_webview_panel("main") {
                p.hide();
                p.set_alpha_value(1.0);
            }
        });
    });
    true
}

/// Windows: the system's own AnimateWindow blend.
#[cfg(target_os = "windows")]
fn native_show(_app: &AppHandle, win: &WebviewWindow) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{AnimateWindow, AW_ACTIVATE, AW_BLEND};
    let Ok(hwnd) = win.hwnd() else { return false };
    let ms = (anim_secs(true) * 1000.0) as u32;
    // SAFETY: valid HWND owned by this process.
    unsafe { AnimateWindow(hwnd.0 as _, ms, AW_BLEND | AW_ACTIVATE) != 0 }
}

#[cfg(target_os = "windows")]
fn native_hide(_app: &AppHandle, win: &WebviewWindow) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{AnimateWindow, AW_BLEND, AW_HIDE};
    let Ok(hwnd) = win.hwnd() else { return false };
    let ms = (anim_secs(false) * 1000.0) as u32;
    unsafe { AnimateWindow(hwnd.0 as _, ms, AW_BLEND | AW_HIDE) != 0 }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn native_show(_app: &AppHandle, _win: &WebviewWindow) -> bool {
    false
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn native_hide(_app: &AppHandle, _win: &WebviewWindow) -> bool {
    false
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn anim_secs(show: bool) -> f64 {
    // PORTKEEPER_ANIM_MS slows animations down, handy to inspect them.
    std::env::var("PORTKEEPER_ANIM_MS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|ms| ms / 1000.0)
        .unwrap_or(if show { 0.22 } else { 0.14 })
}

fn hide(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if native_hide(app, &win) {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(p) = app.get_webview_panel("main") {
            p.hide();
            return;
        }
    }
    let _ = win.hide();
}

/// Place the widget next to the tray icon (below it on macOS, above it on a bottom taskbar),
/// or in a corner when there is no icon position. The geometry lives in `placement.rs`.
fn place_near(app: &AppHandle, win: &WebviewWindow, icon: Option<Rect>) {
    let monitors = app.available_monitors().unwrap_or_default();
    let screens: Vec<placement::Screen> = monitors.iter().map(screen_of).collect();
    let size = window_size(win);
    let found = icon.and_then(|r| placement::icon_screen(|s| icon_area(&r, s), &screens));
    let (p, scale) = match found {
        Some((i, icon)) => (
            placement::near_icon(icon, &screens[i], size),
            screens[i].scale,
        ),
        None => {
            let Some(m) = win.primary_monitor().ok().flatten() else {
                return;
            };
            let screen = screen_of(&m);
            (placement::corner(&screen, size, bar_on_top()), screen.scale)
        }
    };
    app.state::<GrowUp>().0.store(p.grow_up, Ordering::Relaxed);
    move_to(win, LogicalPosition::new(p.x, p.y), scale);
}

/// The tray rect is on a real screen (not macOS's placeholder right after launch).
fn icon_on_a_screen(app: &AppHandle, r: &Rect) -> bool {
    let monitors = app.available_monitors().unwrap_or_default();
    let screens: Vec<placement::Screen> = monitors.iter().map(screen_of).collect();
    placement::icon_screen(|s| icon_area(r, s), &screens).is_some()
}

/// The tray icon rect in logical points, converted with scale `s`.
fn icon_area(r: &Rect, s: f64) -> placement::Area {
    let pos = r.position.to_logical::<f64>(s);
    let size = r.size.to_logical::<f64>(s);
    placement::Area::new(pos.x, pos.y, size.width, size.height)
}

/// A monitor's bounds and work area (minus menu bar, taskbar, panels) in logical points.
fn screen_of(m: &Monitor) -> placement::Screen {
    let s = m.scale_factor();
    let (pos, size) = (
        m.position().to_logical::<f64>(s),
        m.size().to_logical::<f64>(s),
    );
    let work = m.work_area();
    let (wpos, wsize) = (
        work.position.to_logical::<f64>(s),
        work.size.to_logical::<f64>(s),
    );
    placement::Screen {
        bounds: placement::Area::new(pos.x, pos.y, size.width, size.height),
        work: placement::Area::new(wpos.x, wpos.y, wsize.width, wsize.height),
        scale: s,
    }
}

/// The desktop keeps its bar at the top even when the work area does not show it: the
/// macOS menu bar, and GNOME-style desktops (on Wayland the work area is the whole screen).
fn bar_on_top() -> bool {
    if cfg!(target_os = "macos") {
        return true;
    }
    if !cfg!(target_os = "linux") {
        return false;
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    ["gnome", "unity", "pantheon"]
        .iter()
        .any(|d| desktop.contains(d))
}

fn window_size(win: &WebviewWindow) -> (f64, f64) {
    let scale = win.scale_factor().unwrap_or(1.0);
    win.outer_size()
        .map(|s| s.to_logical::<f64>(scale))
        .map(|s| (s.width, s.height))
        .unwrap_or((WIDTH, 200.0))
}

/// macOS: move the panel now, in points. Tauri's `set_position` lands later on the main
/// queue, after `native_show` has already read the frame to animate to, and a physical
/// position would be divided by the scale of the screen the window is on now.
#[cfg(target_os = "macos")]
fn move_to(win: &WebviewWindow, p: LogicalPosition<f64>, _scale: f64) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSScreen, NSWindow};
    use objc2_foundation::NSPoint;
    let (Ok(ptr), Some(mtm)) = (win.ns_window(), MainThreadMarker::new()) else {
        return;
    };
    // Cocoa counts y up from the bottom of the primary screen (the one with the menu bar).
    let Some(primary) = NSScreen::screens(mtm).firstObject() else {
        return;
    };
    // SAFETY: Tauri hands us the live NSWindow; MainThreadMarker proved the thread.
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    ns.setFrameTopLeftPoint(NSPoint::new(p.x, primary.frame().size.height - p.y));
}

/// Windows and Linux position windows in pixels of the target screen.
#[cfg(not(target_os = "macos"))]
fn move_to(win: &WebviewWindow, p: LogicalPosition<f64>, scale: f64) {
    let _ = win.set_position(p.to_physical::<i32>(scale));
}

fn show(app: &AppHandle, icon: Option<Rect>) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    place_near(app, &win, icon);
    // Lets the widget re-check for updates when it has been a while (see main.js).
    let _ = app.emit_to("main", "widget-shown", ());
    if native_show(app, &win) {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(p) = app.get_webview_panel("main") {
            p.show();
            return;
        }
    }
    let _ = win.show();
}

fn toggle(app: &AppHandle, icon: Option<Rect>) {
    if is_visible(app) {
        hide(app);
    } else {
        show(app, icon);
    }
}

/// `portkeeper --dump`: print the scan as JSON (for debugging platform differences).
pub fn dump() {
    let ports = ports::list();
    let docker = docker::list();
    println!(
        "{}",
        serde_json::json!({ "ports": ports, "docker": docker })
    );
}

/// Portkeeper used to be called Portero (bundle id `dev.maparisi.portero`).
/// On the first start with the new id, copy settings.json and closed.json
/// from the old data folder so locks and the Closed list survive the rename.
fn migrate_from_portero(data_dir: &std::path::Path) {
    let old = match data_dir.parent() {
        Some(parent) => parent.join("dev.maparisi.portero"),
        None => return,
    };
    if !old.is_dir() {
        return;
    }
    for name in ["settings.json", "closed.json"] {
        let (from, to) = (old.join(name), data_dir.join(name));
        if from.is_file() && !to.exists() {
            let _ = std::fs::create_dir_all(data_dir);
            let _ = std::fs::copy(&from, &to);
        }
    }
}

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(GrowUp(AtomicBool::new(false)))
        .manage(ScannedPids::default())
        .invoke_handler(tauri::generate_handler![
            list_ports,
            kill_process,
            list_containers,
            docker_action,
            docker_logs,
            open_url,
            get_settings,
            fit_window,
            hide_window,
            set_tray_count,
            list_closed,
            relaunch,
            forget_closed,
            forget_all_closed,
            set_setting,
            set_theme,
            set_lang,
            probe_ports,
            check_update,
            install_update,
            toggle_protected,
            quit,
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            migrate_from_portero(&data_dir);
            app.manage(Mutex::new(launch::Tracker::load(
                data_dir.join("closed.json"),
            )));
            let store = settings::Store::load(data_dir.join("settings.json"));
            let show_on_start = store.value.show_on_start;
            app.manage(Mutex::new(store));

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let toggle_item = MenuItem::with_id(app, "toggle", "Show / hide", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Portkeeper", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&toggle_item, &PredefinedMenuItem::separator(app)?, &quit],
            )?;

            // Left click toggles the widget; right click opens the menu.
            let tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Portkeeper")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        let rect = app.tray_by_id("main").and_then(|t| t.rect().ok().flatten());
                        toggle(app, rect);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        toggle(tray.app_handle(), Some(rect));
                    }
                });

            #[cfg(target_os = "macos")]
            let tray = tray
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray.png"
                ))?)
                .icon_as_template(true);
            // Windows / Linux: a full-size coloured tile; the app icon has macOS
            // margins and shrinks to a few pixels in the tray.
            #[cfg(not(target_os = "macos"))]
            let tray = tray.icon(tauri::image::Image::from_bytes(include_bytes!(
                "../icons/tray-color.png"
            ))?);
            let tray = tray.build(app)?;

            if let Some(win) = app.get_webview_window("main") {
                apply_glass(&win);
                #[cfg(target_os = "macos")]
                make_panel(&win)?;
                // Open on launch (unless turned off), under the icon. The icon gets
                // its position a moment after launch, so wait for it (max ~2 s).
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    if !show_on_start {
                        return;
                    }
                    let mut rect = None;
                    for _ in 0..20 {
                        rect = tray
                            .rect()
                            .ok()
                            .flatten()
                            .filter(|r| icon_on_a_screen(&handle, r));
                        if rect.is_some() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    let h2 = handle.clone();
                    let _ = handle.run_on_main_thread(move || show(&h2, rect));
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Portkeeper");
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrates_old_portero_files_once() {
        let root = std::env::temp_dir().join(format!("pk-migrate-{}", std::process::id()));
        let old = root.join("dev.maparisi.portero");
        let new = root.join("dev.maparisi.portkeeper");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("settings.json"), "{\"theme\":\"dark\"}").unwrap();
        super::migrate_from_portero(&new);
        assert_eq!(
            std::fs::read_to_string(new.join("settings.json")).unwrap(),
            "{\"theme\":\"dark\"}"
        );
        // Existing files in the new folder are never overwritten.
        std::fs::write(new.join("settings.json"), "{}").unwrap();
        super::migrate_from_portero(&new);
        assert_eq!(
            std::fs::read_to_string(new.join("settings.json")).unwrap(),
            "{}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// A locked project's process is refused; spawns `python3`, so ignored by default.
    #[test]
    #[ignore]
    fn lock_blocks_kill() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let mut child = std::process::Command::new("python3")
            .args(["-c", "import time; time.sleep(30)"])
            .current_dir(dir)
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let file =
            std::env::temp_dir().join(format!("portkeeper-lock-{}.json", std::process::id()));
        let mut store = crate::settings::Store::load(file.clone());
        let key = crate::settings::key_for_cwd(&crate::system::tilde(dir));
        assert!(!super::pid_is_locked(&store, child.id()));
        store.toggle_protected(&key);
        let locked = super::pid_is_locked(&store, child.id());
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(file);
        assert!(locked, "process in a locked folder must be refused");
    }
}
