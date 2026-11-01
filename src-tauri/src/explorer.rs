//! Windows: the files selected in the File Explorer window (or on the desktop) a drag started from.
//! Explorer doesn't hand its drag to the menu window, so we ask it which files are being dragged.

use std::path::PathBuf;
use windows::{
    core::Result,
    Win32::{
        Foundation::{HWND, POINT},
        System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4},
        System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider, CLSCTX_ALL, COINIT_APARTMENTTHREADED},
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationElement, UIA_DataItemControlTypeId, UIA_ListControlTypeId, UIA_ListItemControlTypeId, UIA_TreeControlTypeId,
                UIA_TreeItemControlTypeId,
            },
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

/// COM is set up once per thread and released when the thread ends.
struct Com(bool);

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

thread_local! {
    static COM: Com = Com(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok());
    static UIA: Option<IUIAutomation> = unsafe {
        let uia: Option<IUIAutomation> = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL).ok();
        // a busy Explorer mustn't hold up the menu: give up quickly and let it open
        if let Some(u2) = uia.as_ref().and_then(|u| u.cast::<IUIAutomation2>().ok()) {
            let _ = u2.SetConnectionTimeout(400);
            let _ = u2.SetTransactionTimeout(400);
        }
        uia
    };
}

fn com() {
    COM.with(|_| ());
}

/// True when the press at (x, y) landed on a file or folder rather than on empty space.
/// Holding Shift and dragging from empty space draws a selection box, which mustn't open the menu.
/// If Windows can't tell, it counts as a file, so the menu still works.
pub fn on_item(x: i32, y: i32) -> bool {
    com();
    UIA.with(|uia| {
        let Some(uia) = uia else { return true };
        match unsafe { uia.ElementFromPoint(POINT { x, y }) } {
            Ok(el) => is_item(uia, el),
            Err(_) => true,
        }
    })
}

/// Whether the element is a file or folder entry, or part of one (its name, icon or a column).
fn is_item(uia: &IUIAutomation, mut el: IUIAutomationElement) -> bool {
    unsafe {
        let Ok(walker) = uia.ControlViewWalker() else { return true };
        for _ in 0..10 {
            match el.CurrentControlType() {
                Ok(t) if t == UIA_ListItemControlTypeId || t == UIA_DataItemControlTypeId || t == UIA_TreeItemControlTypeId => return true,
                // the empty space of the file list or the folder tree
                Ok(t) if t == UIA_ListControlTypeId || t == UIA_TreeControlTypeId => return false,
                Err(_) => return true,
                _ => {}
            }
            match walker.GetParentElement(&el) {
                Ok(p) => el = p,
                Err(_) => return true,
            }
        }
        // no file list found above it: can't tell, so the menu still opens
        true
    }
}

unsafe fn find(root: HWND) -> Result<Vec<PathBuf>> {
    com();
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

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::Accessibility::{TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_NamePropertyId};

    /// Opens a real File Explorer window, so it only runs when asked: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn selection_box_from_empty_space_is_not_a_file_drag() {
        let dir = std::env::temp_dir().join("shiftdrop-tests").join("rubber band");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("first.txt"), "x").unwrap();
        // explorer.exe hands the folder to the running Explorer and returns at once
        let _ = std::process::Command::new("explorer").arg(&dir).status();
        com();
        let uia = UIA.with(|u| u.clone()).expect("UI Automation");
        unsafe {
            let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).unwrap();
            // only the window this test opened, never one of the user's
            let ours = || -> Option<IWebBrowserApp> {
                (0..shell.Count().ok()?).filter_map(|i| shell.Item(&int(i)).ok()?.cast::<IWebBrowserApp>().ok()).find(|a| a.LocationURL().is_ok_and(|u| u.to_string().contains("shiftdrop-tests/rubber")))
            };
            let mut app = None;
            for _ in 0..40 {
                app = ours();
                if app.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            let app = app.expect("the Explorer window didn't open");
            let hwnd = HWND(app.HWND().unwrap().0 as _);
            let result = std::panic::catch_unwind(|| {
                let cond = uia
                    .CreateAndCondition(&uia.CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from("first.txt")).unwrap(), &uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &int(UIA_ListItemControlTypeId.0)).unwrap())
                    .unwrap();
                let win = uia.ElementFromHandle(hwnd).unwrap();
                let mut item = None;
                for _ in 0..40 {
                    if let Ok(e) = win.FindFirst(TreeScope_Descendants, &cond) {
                        item = Some(e);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
                let item = item.expect("the Explorer window didn't show the file");
                let walker = uia.ControlViewWalker().unwrap();
                // a press lands on the name or icon inside the entry
                let inner = walker.GetFirstChildElement(&item).unwrap_or(item.clone());
                let mut list = walker.GetParentElement(&item).unwrap();
                while list.CurrentControlType().unwrap() != UIA_ListControlTypeId {
                    list = walker.GetParentElement(&list).unwrap();
                }
                (is_item(&uia, item) && is_item(&uia, inner), is_item(&uia, list))
            });
            let _ = app.Quit();
            let (on_file, on_blank) = result.unwrap();
            assert!(on_file, "a file entry wasn't seen as one");
            assert!(!on_blank, "the empty file list was taken for a file");
        }
    }
}
