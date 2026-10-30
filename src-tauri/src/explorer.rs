//! Windows: the files selected in the File Explorer window (or on the desktop) a drag started from.
//! Explorer doesn't hand its drag to the menu window, so we ask it which files are being dragged.

use std::path::PathBuf;
use windows::{
    core::Result,
    Win32::{
        Foundation::HWND,
        System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4},
        System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, IServiceProvider, CLSCTX_ALL, COINIT_APARTMENTTHREADED},
        UI::{
            Shell::{
                IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp, SID_STopLevelBrowser, ShellWindows,
                SIGDN_FILESYSPATH, SVGIO_SELECTION, SWC_DESKTOP, SWFO_NEEDDISPATCH,
            },
            WindowsAndMessaging::{GetClassNameW, IsWindowVisible},
        },
    },
    core::Interface,
};

/// `root` is the top-level window under the point where the drag started.
pub fn selection(root: isize) -> Vec<PathBuf> {
    unsafe { find(HWND(root as _)) }.unwrap_or_else(|e| {
        crate::log(&format!("explorer selection failed: {e}"));
        Vec::new()
    })
}

unsafe fn find(root: HWND) -> Result<Vec<PathBuf>> {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
    let mut buf = [0u16; 64];
    let n = GetClassNameW(root, &mut buf) as usize;
    if matches!(String::from_utf16_lossy(&buf[..n]).as_str(), "Progman" | "WorkerW") {
        let mut hwnd = 0;
        let disp = shell.FindWindowSW(&int(0), &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)?;
        let sp: IServiceProvider = disp.cast()?;
        return selected(&sp.QueryService(&SID_STopLevelBrowser)?);
    }
    for i in 0..shell.Count()? {
        let Ok(disp) = shell.Item(&int(i)) else { continue };
        let Ok(app) = disp.cast::<IWebBrowserApp>() else { continue };
        if app.HWND().map(|h| h.0).ok() != Some(root.0 as isize) {
            continue;
        }
        let sp: IServiceProvider = disp.cast()?;
        let browser: IShellBrowser = sp.QueryService(&SID_STopLevelBrowser)?;
        // Windows 11 tabs share one window; only the active tab is visible
        if browser.GetWindow().is_ok_and(|t| !IsWindowVisible(t).as_bool()) {
            continue;
        }
        return selected(&browser);
    }
    Ok(Vec::new())
}

fn int(v: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(VARIANT_0_0 { vt: VT_I4, Anonymous: VARIANT_0_0_0 { lVal: v }, ..Default::default() }),
        },
    }
}

unsafe fn selected(browser: &IShellBrowser) -> Result<Vec<PathBuf>> {
    let view: IFolderView = browser.QueryActiveShellView()?.cast()?;
    let items: IShellItemArray = view.Items(SVGIO_SELECTION)?;
    let mut out = Vec::new();
    for i in 0..items.GetCount()? {
        let Ok(name) = items.GetItemAt(i).and_then(|it| it.GetDisplayName(SIGDN_FILESYSPATH)) else { continue };
        if let Ok(s) = name.to_string() {
            out.push(PathBuf::from(s));
        }
        CoTaskMemFree(Some(name.0 as _));
    }
    Ok(out)
}
