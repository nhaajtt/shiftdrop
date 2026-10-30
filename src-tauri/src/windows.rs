use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Overlay (radial menu) size in logical pixels.
const OVERLAY: f64 = 460.0;
const TOAST: (f64, f64) = (380.0, 112.0);
const MARGIN: f64 = 8.0;

fn floating(app: &AppHandle, label: &str, page: &str, size: (f64, f64)) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title("ShiftDrop")
        .inner_size(size.0, size.1)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        // never accept the drop itself: releasing over the menu cancels Explorer's move
        .disable_drag_drop_handler()
        .build()
}

/// The overlay and the toast are created once, hidden, so they appear instantly.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    floating(app, "overlay", "overlay.html", (OVERLAY, OVERLAY))?;
    floating(app, "toast", "toast.html", TOAST)?;
    Ok(())
}

/// Monitor rectangle (physical) and scale for the point.
fn monitor_at(w: &WebviewWindow, x: i32, y: i32) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)> {
    let ms = w.available_monitors().ok()?;
    let m = ms
        .iter()
        .find(|m| {
            let (p, s) = (m.position(), m.size());
            x >= p.x && y >= p.y && x < p.x + s.width as i32 && y < p.y + s.height as i32
        })
        .or(ms.first())?;
    Some((*m.position(), *m.size(), m.scale_factor()))
}

#[cfg(windows)]
fn place(w: &WebviewWindow, x: i32, y: i32, width: i32, height: i32) {
    if let Ok(h) = w.hwnd() {
        crate::win::show_no_activate(h.0 as _, x, y, width, height);
    }
    // let Tauri know too, otherwise the webview stays hidden and never receives the drop
    let _ = w.show();
}

#[cfg(not(windows))]
fn place(w: &WebviewWindow, x: i32, y: i32, width: i32, height: i32) {
    let _ = w.set_size(PhysicalSize::new(width as u32, height as u32));
    let _ = w.set_position(PhysicalPosition::new(x, y));
    let _ = w.show();
}

#[derive(serde::Serialize, Clone)]
struct Show<'a> {
    paths: &'a [PathBuf],
    /// data: URL of what's behind the menu, for the frosted-glass background
    backdrop: Option<String>,
    /// files that pretend to be something else (e.g. a program named like a PDF)
    warnings: Vec<String>,
}

#[cfg(windows)]
fn backdrop(left: i32, top: i32, size: i32) -> Option<String> {
    use base64::Engine;
    let jpg = crate::win::capture(left, top, size, size, 4)?;
    Some(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpg)))
}

#[cfg(not(windows))]
fn backdrop(_: i32, _: i32, _: i32) -> Option<String> {
    None
}

/// Top-left of the overlay on screen (physical), to turn cursor positions into menu coordinates.
static ORIGIN: Mutex<(i32, i32)> = Mutex::new((0, 0));

/// Shows the radial menu for `paths` centred on (x, y) (physical screen coordinates), kept on screen.
pub fn show_overlay(app: &AppHandle, x: i32, y: i32, paths: &[PathBuf]) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let (mp, ms, scale) = monitor_at(&w, x, y).unwrap_or((PhysicalPosition::new(0, 0), PhysicalSize::new(1920, 1080), 1.0));
    let s = (OVERLAY * scale).round() as i32;
    let left = (x - s / 2).clamp(mp.x, mp.x + ms.width as i32 - s);
    let top = (y - s / 2).clamp(mp.y, mp.y + ms.height as i32 - s);
    *ORIGIN.lock().unwrap() = (left, top);
    let warnings = paths.iter().take(50).filter_map(|p| crate::engines::security::disguise(p)).collect();
    let _ = w.emit("overlay:show", Show { paths, backdrop: backdrop(left, top, s), warnings });
    place(&w, left, top, s, s);
    #[cfg(windows)]
    crate::win::set_menu_open(true);
}

/// Cursor moved (`drop` = button released) during a drag that has the menu open.
pub fn overlay_pointer(app: &AppHandle, x: i32, y: i32, drop: bool) {
    let (left, top) = *ORIGIN.lock().unwrap();
    let event = if drop { "overlay:drop" } else { "overlay:move" };
    let _ = app.emit_to("overlay", event, (x - left, y - top));
}

pub fn hide_overlay(app: &AppHandle) {
    #[cfg(windows)]
    crate::win::set_menu_open(false);
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.emit("overlay:hide", ());
        let _ = w.hide();
    }
}

#[tauri::command]
pub async fn overlay_hide(app: AppHandle) {
    hide_overlay(&app);
}

/// Height of the toast in logical pixels: compact for progress, taller when it explains a failure.
static TOAST_H: AtomicU32 = AtomicU32::new(TOAST.1 as u32);

/// Bottom-right corner of the monitor where the wheel was last opened (primary as fallback).
pub fn show_toast(app: &AppHandle) {
    let Some(w) = app.get_webview_window("toast") else { return };
    let (ox, oy) = *ORIGIN.lock().unwrap_or_else(|e| e.into_inner());
    let at = w.available_monitors().ok().and_then(|ms| {
        ms.into_iter().find(|m| {
            let (p, s) = (m.position(), m.size());
            ox >= p.x && oy >= p.y && ox < p.x + s.width as i32 && oy < p.y + s.height as i32
        })
    });
    let Some(m) = at.or_else(|| w.primary_monitor().ok().flatten()) else { return };
    let scale = m.scale_factor();
    let area = m.work_area();
    let (tw, th) = ((TOAST.0 * scale) as i32, (TOAST_H.load(Ordering::Relaxed) as f64 * scale) as i32);
    let pad = (MARGIN * scale) as i32;
    let x = area.position.x + area.size.width as i32 - tw - pad;
    let y = area.position.y + area.size.height as i32 - th - pad;
    place(&w, x, y, tw, th);
}

/// The toast grows to fit an error with its explanation, and shrinks back for progress.
#[tauri::command]
pub async fn toast_height(app: AppHandle, height: u32) {
    TOAST_H.store(height.clamp(TOAST.1 as u32, 320), Ordering::Relaxed);
    if app.get_webview_window("toast").is_some_and(|w| w.is_visible().unwrap_or(false)) {
        show_toast(&app);
    }
}

#[tauri::command]
pub async fn toast_show(app: AppHandle) {
    show_toast(&app);
}

#[tauri::command]
pub async fn toast_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window("toast") {
        let _ = w.hide();
    }
}

/// Opens an editor (crop, add-bg, redact, trim) for one file.
/// Windows can't create a window inside a synchronous command (the UI thread would wait on
/// itself), so the command is async and the tray and background threads call `open_editor_now`.
#[tauri::command]
pub async fn open_editor(app: AppHandle, path: String, tool: String) -> Result<(), String> {
    open_editor_now(app, path, tool)
}

pub fn open_editor_now(app: AppHandle, path: String, tool: String) -> Result<(), String> {
    static N: AtomicU32 = AtomicU32::new(0);
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err("file not found".into());
    }
    // Let the editor webview read only this one file.
    app.asset_protocol_scope().allow_file(&p).map_err(|e| e.to_string())?;
    let query: String = tauri::Url::parse_with_params("x://e/", [("tool", tool.as_str()), ("path", path.as_str())])
        .map(|u| u.query().unwrap_or_default().to_string())
        .map_err(|e| e.to_string())?;
    let label = format!("editor-{}", N.fetch_add(1, Ordering::Relaxed));
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    WebviewWindowBuilder::new(&app, label, WebviewUrl::App(format!("editor.html?{query}").into()))
        .title(format!("{name} — ShiftDrop"))
        .inner_size(900.0, 680.0)
        .min_inner_size(560.0, 480.0)
        .center()
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// What the hub window should show next: a view and, for "more", the dragged files.
#[derive(serde::Serialize, Clone, Default)]
pub struct HubOpen {
    view: String,
    paths: Vec<String>,
    /// the menu entry that needs more input (password, confirmation…)
    item: Option<serde_json::Value>,
}

static HUB: Mutex<Option<HubOpen>> = Mutex::new(None);

/// The main window: every action for the files ("More…"), the shelf, history and settings.
/// Windows can't create a window inside a synchronous command (the UI thread would wait on
/// itself), so the command is async and the tray and background threads call `open_hub_now`.
#[tauri::command]
pub async fn open_hub(app: AppHandle, paths: Vec<String>, view: String, item: Option<serde_json::Value>) -> Result<(), String> {
    open_hub_now(app, paths, view, item)
}

pub fn open_hub_now(app: AppHandle, paths: Vec<String>, view: String, item: Option<serde_json::Value>) -> Result<(), String> {
    let req = HubOpen { view, paths, item };
    *HUB.lock().unwrap() = Some(req.clone());
    if let Some(w) = app.get_webview_window("hub") {
        let _ = w.emit("hub:open", req);
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, "hub", WebviewUrl::App("hub.html".into()))
        .title("ShiftDrop")
        .inner_size(820.0, 600.0)
        .min_inner_size(640.0, 460.0)
        .center()
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Called by the hub once loaded, so a request made before it existed isn't lost.
#[tauri::command]
pub fn hub_take() -> Option<HubOpen> {
    HUB.lock().unwrap().take()
}

static SHELF: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// A small always-on-top shelf at the right edge: collect files from anywhere, drag them out later.
/// Windows can't create a window inside a synchronous command (the UI thread would wait on
/// itself), so the command is async and the tray and background threads call `shelf_add_now`.
#[tauri::command]
pub async fn shelf_add(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    shelf_add_now(app, paths)
}

pub fn shelf_add_now(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    SHELF.lock().unwrap_or_else(|e| e.into_inner()).extend(paths.iter().cloned());
    if let Some(w) = app.get_webview_window("shelf") {
        // a shelf still loading picks the files up itself with shelf_take; an event could be missed
        if SHELF_READY.load(Ordering::SeqCst) {
            let _ = w.emit("shelf:add", std::mem::take(&mut *SHELF.lock().unwrap_or_else(|e| e.into_inner())));
        }
        let _ = w.show();
        return Ok(());
    }
    SHELF_READY.store(false, Ordering::SeqCst);
    let (w, h) = (300.0, 460.0);
    let mut b = WebviewWindowBuilder::new(&app, "shelf", WebviewUrl::App("shelf.html".into()))
        .title("ShiftDrop shelf")
        .inner_size(w, h)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .focused(false);
    if let Some(m) = app.primary_monitor().ok().flatten() {
        let s = m.scale_factor();
        let a = m.work_area();
        let x = (a.position.x + a.size.width as i32) as f64 / s - w - MARGIN;
        let y = a.position.y as f64 / s + (a.size.height as f64 / s - h) / 2.0;
        b = b.position(x, y);
    }
    b.build().map_err(|e| e.to_string())?;
    Ok(())
}

/// Files added before the shelf finished loading.
#[tauri::command]
pub fn shelf_take() -> Vec<String> {
    let mut list = SHELF.lock().unwrap_or_else(|e| e.into_inner());
    SHELF_READY.store(true, Ordering::SeqCst);
    std::mem::take(&mut *list)
}

/// The shelf page is listening for "shelf:add".
static SHELF_READY: AtomicBool = AtomicBool::new(false);

/// A small PNG for the cursor while dragging files out of the app.
#[tauri::command]
pub fn drag_icon() -> Result<String, String> {
    let p = std::env::temp_dir().join("shiftdrop").join("drag.png");
    if !p.exists() {
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        std::fs::write(&p, include_bytes!("../icons/32x32.png")).map_err(|e| e.to_string())?;
    }
    Ok(p.display().to_string())
}

/// Quick look at a file (Ctrl+Space in Explorer): image, video, audio, PDF, text or details.
/// Windows can't create a window inside a synchronous command (the UI thread would wait on
/// itself), so the command is async and the tray and background threads call `open_peek_now`.
#[tauri::command]
pub async fn open_peek(app: AppHandle, path: String) -> Result<(), String> {
    open_peek_now(app, path)
}

pub fn open_peek_now(app: AppHandle, path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("file not found".into());
    }
    if p.is_file() {
        app.asset_protocol_scope().allow_file(&p).map_err(|e| e.to_string())?;
    }
    if let Some(w) = app.get_webview_window("peek") {
        let _ = w.emit("peek", &path);
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let url: String = tauri::Url::parse_with_params("x://e/", [("path", path.as_str())]).map(|u| u.query().unwrap_or_default().to_string()).map_err(|e| e.to_string())?;
    WebviewWindowBuilder::new(&app, "peek", WebviewUrl::App(format!("peek.html?{url}").into()))
        .title("ShiftDrop preview")
        .inner_size(960.0, 680.0)
        .center()
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// The start of a text file (for previews), as UTF-8.
#[tauri::command]
pub async fn read_text(path: String, max: usize) -> Result<String, String> {
    use std::io::Read;
    let mut buf = Vec::new();
    std::fs::File::open(&path).and_then(|f| f.take(max.min(4 << 20) as u64).read_to_end(&mut buf)).map_err(|e| e.to_string())?;
    let utf16 = |b: &[u8], be: bool| {
        let u: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| if be { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) }).collect();
        String::from_utf16_lossy(&u)
    };
    Ok(match buf.as_slice() {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, false),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, true),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        b => String::from_utf8_lossy(b).into_owned(),
    })
}

#[derive(serde::Serialize)]
pub struct FileInfo {
    size: u64,
    modified: u64,
    is_dir: bool,
    real_type: Option<String>,
    warning: Option<String>,
    items: Option<usize>,
}

#[tauri::command]
pub async fn file_info(path: String) -> Result<FileInfo, String> {
    let p = PathBuf::from(&path);
    let m = std::fs::metadata(&p).map_err(|e| e.to_string())?;
    let modified = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    Ok(FileInfo {
        size: m.len(),
        modified,
        is_dir: m.is_dir(),
        real_type: if m.is_file() { crate::engines::security::sniff(&p).map(|(_, d)| d) } else { None },
        warning: if m.is_file() { crate::engines::security::disguise(&p) } else { None },
        items: if m.is_dir() { std::fs::read_dir(&p).ok().map(|r| r.count()) } else { None },
    })
}
