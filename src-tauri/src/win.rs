//! Windows-only: global mouse hook that notices "dragging + Shift held", plus small Win32 helpers.

use std::{
    cell::Cell,
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        OnceLock,
    },
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND, LPARAM, LRESULT, POINT, WPARAM},
    System::{
        LibraryLoader::GetModuleHandleW,
        Threading::{GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION},
    },
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SHIFT},
        WindowsAndMessaging::{
            CallNextHookEx, GetAncestor, GetMessageW, GetWindowThreadProcessId, SetWindowPos, SetWindowsHookExW, WindowFromPoint,
            GA_ROOT, HWND_TOPMOST, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, SWP_NOACTIVATE, SWP_SHOWWINDOW, WH_KEYBOARD_LL, WH_MOUSE_LL,
            WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_SYSKEYDOWN,
        },
    },
};

pub enum HookEvent {
    /// A drag passed the threshold while Shift was held; `ox, oy` = where it started.
    ShiftDrag { ox: i32, oy: i32 },
    /// Cursor position while a Shift-drag is in progress.
    Moved { x: i32, y: i32 },
    Released { x: i32, y: i32 },
    /// 1–8 picks a wedge, 0 = Escape (cancel), during a Shift-drag.
    Key { n: u32 },
    /// Ctrl+Space in File Explorer or on the desktop: preview the selected file.
    Peek { root: isize },
}

static TX: OnceLock<Sender<HookEvent>> = OnceLock::new();
const THRESHOLD: i32 = 10;
/// Ctrl+Space is only taken from Explorer while Quick look is on.
static PEEK: AtomicBool = AtomicBool::new(false);
/// Digits are only taken from other apps while the wheel is actually on screen.
static MENU_OPEN: AtomicBool = AtomicBool::new(false);

pub fn set_peek(on: bool) {
    PEEK.store(on, Ordering::Relaxed);
}

pub fn set_menu_open(open: bool) {
    MENU_OPEN.store(open, Ordering::Relaxed);
}

thread_local! {
    static DOWN: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
    static FIRED: Cell<bool> = const { Cell::new(false) };
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        let (x, y) = (info.pt.x, info.pt.y);
        match wparam as u32 {
            WM_LBUTTONDOWN => {
                DOWN.set(Some((x, y)));
                FIRED.set(false);
            }
            WM_MOUSEMOVE => {
                if FIRED.get() {
                    if let Some(tx) = TX.get() {
                        let _ = tx.send(HookEvent::Moved { x, y });
                    }
                } else if let Some((ox, oy)) = DOWN.get() {
                    let far = (x - ox).abs() > THRESHOLD || (y - oy).abs() > THRESHOLD;
                    if far && GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000 != 0 {
                        FIRED.set(true);
                        if let Some(tx) = TX.get() {
                            let _ = tx.send(HookEvent::ShiftDrag { ox, oy });
                        }
                    }
                }
            }
            WM_LBUTTONUP => {
                if FIRED.get() {
                    if let Some(tx) = TX.get() {
                        let _ = tx.send(HookEvent::Released { x, y });
                    }
                }
                DOWN.set(None);
                FIRED.set(false);
            }
            _ => {}
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

/// Number keys pick a wedge while the menu is open; Escape cancels it.
/// The focused Explorer window or desktop, unless the user is typing (rename, address bar).
unsafe fn explorer_focused() -> Option<HWND> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GUITHREADINFO};
    let class = |h: HWND| {
        let mut buf = [0u16; 64];
        let n = GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    };
    let fg = GetForegroundWindow();
    if !["CabinetWClass", "Progman", "WorkerW"].contains(&class(fg).as_str()) {
        return None;
    }
    let mut info: GUITHREADINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if GetGUIThreadInfo(GetWindowThreadProcessId(fg, std::ptr::null_mut()), &mut info) != 0 && !info.hwndFocus.is_null()
        && (["Edit", "RichEdit20W", "RICHEDIT50W"].contains(&class(info.hwndFocus).as_str()) || !info.hwndCaret.is_null()) {
            return None;
        }
    Some(fg)
}

unsafe extern "system" fn key_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam as u32 == WM_KEYDOWN && (*(lparam as *const KBDLLHOOKSTRUCT)).vkCode == 0x20 && !FIRED.get() && PEEK.load(Ordering::Relaxed) {
        // Ctrl+Space (without Shift/Alt) in Explorer opens the preview
        let down = |vk: u16| GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0;
        if down(0x11) && !down(VK_SHIFT) && !down(0x12) {
            if let (Some(fg), Some(tx)) = (explorer_focused(), TX.get()) {
                let _ = tx.send(HookEvent::Peek { root: fg as isize });
                return 1;
            }
        }
    }
    if code >= 0 && FIRED.get() && matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
        let vk = (*(lparam as *const KBDLLHOOKSTRUCT)).vkCode;
        let n = match vk {
            0x31..=0x38 => Some(vk - 0x30),
            0x61..=0x68 => Some(vk - 0x60), // numpad
            0x1B => Some(0),
            _ => None,
        };
        if let (Some(n), Some(tx)) = (n, TX.get()) {
            let _ = tx.send(HookEvent::Key { n });
            if n != 0 && MENU_OPEN.load(Ordering::Relaxed) {
                // keep the digit from reaching Explorer; Escape still cancels its drag
                return 1;
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

/// Every click and key on the PC passes through the hooks, so they must never be slowed down.
/// Windows 11 puts tray apps into "Efficiency mode" (low priority, efficiency cores), which made
/// the whole system's mouse stutter (e.g. right-click menus); opt out for the process and give
/// the hook thread top priority.
unsafe fn keep_fast() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentThread, SetProcessInformation, SetThreadInformation, SetThreadPriority, ProcessPowerThrottling,
        ThreadPowerThrottling, PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
        THREAD_POWER_THROTTLING_CURRENT_VERSION, THREAD_POWER_THROTTLING_EXECUTION_SPEED, THREAD_POWER_THROTTLING_STATE, THREAD_PRIORITY_TIME_CRITICAL,
    };
    let p = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: 0,
    };
    SetProcessInformation(GetCurrentProcess(), ProcessPowerThrottling, &p as *const _ as _, std::mem::size_of_val(&p) as u32);
    let t = THREAD_POWER_THROTTLING_STATE {
        Version: THREAD_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: 0,
    };
    SetThreadInformation(GetCurrentThread(), ThreadPowerThrottling, &t as *const _ as _, std::mem::size_of_val(&t) as u32);
    SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
}

/// Installs the low-level mouse hook on its own thread (it needs a message loop).
pub fn start_hook(tx: Sender<HookEvent>) {
    let _ = TX.set(tx);
    std::thread::spawn(|| unsafe {
        keep_fast();
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), GetModuleHandleW(std::ptr::null()), 0);
        if hook.is_null() {
            crate::log("mouse hook failed");
            return;
        }
        crate::log("mouse hook installed");
        // same thread, so the keyboard hook sees whether a Shift-drag is in progress
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), GetModuleHandleW(std::ptr::null()), 0);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
    });
}

fn root_at(x: i32, y: i32) -> HWND {
    unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) }
}

fn pid_at(x: i32, y: i32) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(root_at(x, y), &mut pid) };
    pid
}

/// Top-level window under the point, as a plain handle value.
pub fn root_handle_at(x: i32, y: i32) -> isize {
    root_at(x, y) as isize
}

pub fn is_own_window_at(x: i32, y: i32) -> bool {
    pid_at(x, y) == unsafe { GetCurrentProcessId() }
}

/// True when the window under the point belongs to explorer.exe (desktop, File Explorer).
pub fn is_explorer_at(x: i32, y: i32) -> bool {
    let pid = pid_at(x, y);
    if pid == 0 {
        return false;
    }
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) != 0;
        CloseHandle(h);
        ok && String::from_utf16_lossy(&buf[..len as usize]).to_lowercase().ends_with("\\explorer.exe")
    }
}

/// Moves, sizes and shows a window on top without taking focus from the app being dragged from.
pub fn show_no_activate(hwnd: *mut c_void, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}



/// Frosted glass on the small capture: three box-blur passes (close to a Gaussian), then
/// more colour and a touch of brightness, like light through tinted glass.
fn frost(rgb: &mut [u8], w: usize, h: usize) {
    let mut buf = vec![0u8; rgb.len()];
    for _ in 0..3 {
        box_pass(rgb, &mut buf, w, h, 2, true);
        box_pass(&buf, rgb, w, h, 2, false);
    }
    for p in rgb.as_chunks_mut::<3>().0 {
        let l = 0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32;
        for c in p.iter_mut() {
            *c = ((l + (*c as f32 - l) * 1.9) * 1.06).clamp(0.0, 255.0) as u8;
        }
    }
}

/// One horizontal or vertical box-blur pass of radius `r`, edges clamped.
fn box_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    let (len, lines) = if horizontal { (w, h) } else { (h, w) };
    let idx = |line: usize, i: usize| if horizontal { (line * w + i) * 3 } else { (i * w + line) * 3 };
    let n = (2 * r + 1) as u32;
    for line in 0..lines {
        for c in 0..3 {
            let mut sum: u32 = (0..=2 * r).map(|k| src[idx(line, k.saturating_sub(r).min(len - 1)) + c] as u32).sum();
            for i in 0..len {
                dst[idx(line, i) + c] = (sum / n) as u8;
                let out = i.saturating_sub(r);
                let inn = (i + r + 1).min(len - 1);
                sum = sum + src[idx(line, inn) + c] as u32 - src[idx(line, out) + c] as u32;
            }
        }
    }
}

/// A small, downscaled copy of the screen inside the rectangle, as JPEG bytes. The menu blurs it
/// into its frosted-glass background, so it must be taken before the menu appears.
pub fn capture(x: i32, y: i32, w: i32, h: i32, shrink: i32) -> Option<Vec<u8>> {
    use windows_sys::Win32::Graphics::Gdi::*;
    let (sw, sh) = ((w / shrink).max(1), (h / shrink).max(1));
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, sw, sh);
        let old = SelectObject(dc, bmp);
        SetStretchBltMode(dc, HALFTONE);
        StretchBlt(dc, 0, 0, sw, sh, screen, x, y, w, h, SRCCOPY);
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = sw;
        info.bmiHeader.biHeight = -sh; // top-down rows
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        let mut bgra = vec![0u8; (sw * sh * 4) as usize];
        let rows = GetDIBits(dc, bmp, 0, sh as u32, bgra.as_mut_ptr() as _, &mut info, DIB_RGB_COLORS);
        SelectObject(dc, old);
        DeleteObject(bmp);
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), screen);
        if rows == 0 {
            return None;
        }
        let mut rgb: Vec<u8> = bgra.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0]]).collect();
        // the frost is baked in here, once: animating a CSS blur would redo it every frame
        frost(&mut rgb, sw as usize, sh as usize);
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80).encode(&rgb, sw as u32, sh as u32, image::ExtendedColorType::Rgb8).ok()?;
        Some(out)
    }
}
