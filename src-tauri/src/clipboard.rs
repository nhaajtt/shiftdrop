//! Puts plain text on the system clipboard.

use anyhow::{bail, Result};

#[cfg(windows)]
pub fn set(text: &str) -> Result<()> {
    use windows_sys::Win32::System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    };
    const CF_UNICODETEXT: u32 = 13;
    let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
    unsafe {
        // another app may hold the clipboard for a moment
        let mut opened = false;
        for _ in 0..10 {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if !opened {
            bail!("the clipboard is busy");
        }
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2);
        if mem.is_null() {
            CloseClipboard();
            bail!("out of memory");
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), GlobalLock(mem) as *mut u16, wide.len());
        GlobalUnlock(mem);
        // on success the clipboard owns the memory
        if SetClipboardData(CF_UNICODETEXT, mem as _).is_null() {
            CloseClipboard();
            bail!("couldn't copy");
        }
        CloseClipboard();
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn set(text: &str) -> Result<()> {
    use std::io::Write;
    let mut child = std::process::Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn()?;
    child.stdin.take().unwrap().write_all(text.as_bytes())?;
    if !child.wait()?.success() {
        bail!("couldn't copy");
    }
    Ok(())
}
