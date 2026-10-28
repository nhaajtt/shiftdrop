mod apps;
mod clipboard;
mod engines;
#[cfg(windows)]
mod explorer;
mod jobs;
mod naming;
mod settings;
mod share;
mod shell;
mod watch;
mod tray;
#[cfg(windows)]
mod win;
mod windows;
#[cfg(windows)]
mod winrt;

use std::sync::Mutex;
use tauri::{Manager, RunEvent};

/// Appends a line to %TEMP%\shiftdrop.log (debug builds only).
pub fn log(msg: &str) {
    if cfg!(debug_assertions) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("shiftdrop.log")) {
            let _ = writeln!(f, "{:?} {msg}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
        }
    }
}

#[tauri::command]
fn ui_log(msg: String) {
    log(&format!("ui: {msg}"));
}

/// Where pdfium.dll (and optionally ffmpeg.exe / ffprobe.exe) live.
fn bin_dir(app: &tauri::App) -> std::path::PathBuf {
    let bundled = app.path().resource_dir().map(|d| d.join("resources").join("bin")).unwrap_or_default();
    if cfg!(debug_assertions) && !bundled.exists() {
        return std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("bin");
    }
    bundled
}

#[cfg(windows)]
fn start_drag_watch(app: tauri::AppHandle) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::Emitter;
    use win::HookEvent;

    let (tx, rx) = std::sync::mpsc::channel();
    win::start_hook(tx);
    let open = AtomicBool::new(false);
    std::thread::spawn(move || {
        let mut last_move = std::time::Instant::now();
        for ev in rx {
            match ev {
                HookEvent::ShiftDrag { ox, oy } => {
                    let s = settings::get(&app);
                    if !s.enabled || win::is_own_window_at(ox, oy) || (s.explorer_only && !win::is_explorer_at(ox, oy)) {
                        continue;
                    }
                    let paths = explorer::selection(win::root_handle_at(ox, oy));
                    log(&format!("shift-drag from {ox},{oy}: {paths:?}"));
                    if paths.is_empty() {
                        continue;
                    }
                    open.store(true, Ordering::SeqCst);
                    windows::show_overlay(&app, ox, oy, &paths);
                }
                HookEvent::Moved { x, y } => {
                    // high-rate mice report up to 8000 moves a second; ~120 updates are plenty
                    if open.load(Ordering::SeqCst) && last_move.elapsed() >= std::time::Duration::from_millis(8) {
                        last_move = std::time::Instant::now();
                        windows::overlay_pointer(&app, x, y, false);
                    }
                }
                HookEvent::Peek { root } => {
                    if settings::get(&app).peek {
                        let paths = explorer::selection(root);
                        if let Some(p) = paths.first() {
                            let _ = windows::open_peek(app.clone(), p.display().to_string());
                        }
                    }
                }
                HookEvent::Key { n } => {
                    if open.load(Ordering::SeqCst) {
                        let _ = app.emit_to("overlay", "overlay:key", n);
                        if n == 0 {
                            open.store(false, Ordering::SeqCst);
                        }
                    }
                }
                HookEvent::Released { x, y } => {
                    if open.swap(false, Ordering::SeqCst) {
                        windows::overlay_pointer(&app, x, y, true);
                    }
                }
            }
        }
    });
}

/// Files passed with `--open` (Explorer's right-click entry). Explorer may start one process per
/// selected file, so paths arriving within a moment are gathered into one hub window.
fn opened_with(app: &tauri::AppHandle, argv: Vec<String>) {
    static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let Some(i) = argv.iter().position(|a| a == "--open") else { return };
    let first = {
        let mut p = PENDING.lock().unwrap();
        let first = p.is_empty();
        p.extend(argv[i + 1..].iter().cloned());
        first
    };
    if first {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(350));
            let paths = std::mem::take(&mut *PENDING.lock().unwrap());
            let _ = windows::open_hub(app, paths, "actions".into(), None);
        });
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|p| log(&format!("panic: {p}"))));
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _| opened_with(app, argv)))
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(settings::Store(Mutex::new(settings::Settings::default())))
        .invoke_handler(tauri::generate_handler![
            ui_log,
            apps::get_apps,
            apps::open_with,
            settings::get_settings,
            settings::update_settings,
            settings::get_autostart,
            settings::set_autostart,
            jobs::run_action,
            jobs::reveal,
            windows::open_editor,
            windows::overlay_hide,
            windows::toast_hide,
            windows::open_hub,
            windows::hub_take,
            engines::security::tools_status,
            engines::security::tools_install,
            engines::security::sandbox_ok,
            engines::office::office_ok,
            engines::pdf::pdf_thumbs,
            shell::trash,
            share::share_lan,
            share::share_r2,
            watch::folder_exists,
            windows::shelf_add,
            windows::shelf_take,
            windows::drag_icon,
            windows::open_peek,
            windows::read_text,
            windows::file_info,
            shell::clipboard_files,
            shell::rename_files,
            shell::context_menu,
            shell::context_menu_on,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            settings::update(&handle, |s| *s = settings::load(&handle));
            engines::tools::set_bin_dir(bin_dir(app));
            app.manage(jobs::start(handle.clone()));
            windows::create(&handle)?;
            tray::create(&handle)?;
            #[cfg(windows)]
            // debug aid: SHIFTDROP_DEMO=<file> shows the menu for that file in the middle of the screen
            #[cfg(debug_assertions)]
            if let Ok(f) = std::env::var("SHIFTDROP_DEMO") {
                let h = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    windows::show_overlay(&h, 1000, 600, &[f.into()]);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    windows::overlay_pointer(&h, 1000, 600 - 100, false);
                    if std::env::var_os("SHIFTDROP_DEMO_DROP").is_some() {
                        std::thread::sleep(std::time::Duration::from_secs(4));
                        windows::overlay_pointer(&h, 1000, 600, true);
                    }
                });
            }
            #[cfg(debug_assertions)]
            if let Ok(v) = std::env::var("SHIFTDROP_HUB") {
                let files = std::env::var("SHIFTDROP_HUB_FILES").map(|f| f.split(';').map(String::from).collect()).unwrap_or_default();
                let _ = windows::open_hub(handle.clone(), files, v, None);
            }
            watch::start(handle.clone());
            opened_with(&handle, std::env::args().collect());
            start_drag_watch(handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start ShiftDrop")
        .run(|_, ev| {
            // keep running in the tray when editor windows close
            if let RunEvent::ExitRequested { code, .. } = &ev {
                log(&format!("exit requested, code {code:?}"));
            }
            if let RunEvent::ExitRequested { api, code: None, .. } = ev {
                api.prevent_exit();
            }
        });
}
