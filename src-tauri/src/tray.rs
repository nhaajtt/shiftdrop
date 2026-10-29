use crate::settings::{self, ACCENTS, THEMES};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Wry,
};
use tauri_plugin_autostart::ManagerExt;

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let s = settings::get(app);
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    let title = MenuItem::with_id(app, "title", "ShiftDrop — Shift + drag a file", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open ShiftDrop…", true, None::<&str>)?;
    let shelf = MenuItem::with_id(app, "shelf", "Shelf", true, None::<&str>)?;
    let help = MenuItem::with_id(app, "welcome", "How to use", true, None::<&str>)?;
    let activity = MenuItem::with_id(app, "activity", "Activity (results and errors)", true, None::<&str>)?;
    let enabled = CheckMenuItem::with_id(app, "enabled", "Enabled", true, s.enabled, None::<&str>)?;
    let explorer = CheckMenuItem::with_id(app, "explorer_only", "Only from File Explorer and desktop", true, s.explorer_only, None::<&str>)?;
    let startup = CheckMenuItem::with_id(app, "autostart", "Start with Windows", true, autostart, None::<&str>)?;

    let accent = Submenu::with_id(app, "accent", "Accent", true)?;
    for a in ACCENTS {
        accent.append(&CheckMenuItem::with_id(app, format!("accent:{a}"), cap(a), true, s.accent == a, None::<&str>)?)?;
    }
    let theme = Submenu::with_id(app, "theme", "Theme", true)?;
    for t in THEMES {
        theme.append(&CheckMenuItem::with_id(app, format!("theme:{t}"), cap(t), true, s.theme == t, None::<&str>)?)?;
    }
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);
    Menu::with_items(app, &[&title, &open, &activity, &shelf, &help, &sep()?, &enabled, &explorer, &startup, &sep()?, &accent, &theme, &sep()?, &quit])
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("icon"))
        .tooltip("ShiftDrop")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, ev| {
            let id = ev.id().as_ref();
            match id {
                "quit" => {
                    crate::log("quit from tray");
                    app.exit(0)
                }
                "open" => {
                    let _ = crate::windows::open_hub_now(app.clone(), vec![], "actions".into(), None);
                }
                "activity" => {
                    let _ = crate::windows::open_hub_now(app.clone(), vec![], "activity".into(), None);
                }
                "welcome" => {
                    let _ = crate::windows::open_hub_now(app.clone(), vec![], "welcome".into(), None);
                }
                "shelf" => {
                    let _ = crate::windows::shelf_add_now(app.clone(), vec![]);
                }
                "enabled" => settings::update(app, |s| s.enabled = !s.enabled),
                "explorer_only" => settings::update(app, |s| s.explorer_only = !s.explorer_only),
                "autostart" => {
                    let al = app.autolaunch();
                    let _ = if al.is_enabled().unwrap_or(false) { al.disable() } else { al.enable() };
                }
                _ => {
                    if let Some(a) = id.strip_prefix("accent:") {
                        settings::update(app, |s| s.accent = a.into());
                    } else if let Some(t) = id.strip_prefix("theme:") {
                        settings::update(app, |s| s.theme = t.into());
                    }
                }
            }
            // rebuild so the check marks match the stored state (radio-like groups)
            if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), build_menu(app)) {
                let _ = tray.set_menu(Some(menu));
            }
        })
        .build(app)?;
    Ok(())
}
