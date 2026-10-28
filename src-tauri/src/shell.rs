//! Small file-system helpers for the hub: undo (to the Recycle Bin), files or images from the
//! clipboard, batch rename, and the Explorer right-click entry.

use std::path::{Path, PathBuf};

/// Moves files to the Recycle Bin, so "Undo" can itself be undone from there.
#[tauri::command]
pub async fn trash(paths: Vec<String>) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::{SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
        let mut from: Vec<u16> = Vec::new();
        for p in paths.iter().filter(|p| Path::new(p).exists()) {
            from.extend(p.encode_utf16());
            from.push(0);
        }
        if from.is_empty() {
            return Ok(());
        }
        from.push(0);
        let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
        op.wFunc = FO_DELETE;
        op.pFrom = from.as_ptr();
        op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI) as u16;
        if unsafe { SHFileOperationW(&mut op) } != 0 {
            return Err("couldn't move to the Recycle Bin".into());
        }
    }
    #[cfg(not(windows))]
    for p in paths {
        let _ = std::fs::remove_file(p);
    }
    Ok(())
}

/// Files copied in Explorer, or a copied image saved to Pictures\ShiftDrop.
#[tauri::command]
pub async fn clipboard_files() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        if let Some(files) = clip::files() {
            return Ok(files);
        }
        if let Some(png) = clip::image_png() {
            let dir = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Pictures").join("ShiftDrop");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let out = crate::naming::unique(&dir, &format!("Pasted image {stamp}"), "png");
            std::fs::write(&out, png).map_err(|e| e.to_string())?;
            return Ok(vec![out.display().to_string()]);
        }
    }
    Err("the clipboard has no files or image".into())
}

#[cfg(windows)]
mod clip {
    use windows_sys::Win32::{
        System::{
            DataExchange::{CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard},
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
        },
        UI::Shell::{DragQueryFileW, HDROP},
    };
    const CF_HDROP: u32 = 15;
    const CF_DIB: u32 = 8;

    fn with<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let r = f();
            CloseClipboard();
            r
        }
    }

    pub fn files() -> Option<Vec<String>> {
        with(|| unsafe {
            if IsClipboardFormatAvailable(CF_HDROP) == 0 {
                return None;
            }
            let h = GetClipboardData(CF_HDROP) as HDROP;
            let n = DragQueryFileW(h, u32::MAX, std::ptr::null_mut(), 0);
            let mut out = Vec::new();
            for i in 0..n {
                let len = DragQueryFileW(h, i, std::ptr::null_mut(), 0) as usize;
                let mut buf = vec![0u16; len + 1];
                DragQueryFileW(h, i, buf.as_mut_ptr(), buf.len() as u32);
                out.push(String::from_utf16_lossy(&buf[..len]));
            }
            (!out.is_empty()).then_some(out)
        })
    }

    /// The clipboard bitmap (CF_DIB) re-encoded as PNG.
    pub fn image_png() -> Option<Vec<u8>> {
        let dib = with(|| unsafe {
            if IsClipboardFormatAvailable(CF_DIB) == 0 {
                return None;
            }
            let h = GetClipboardData(CF_DIB);
            let p = GlobalLock(h) as *const u8;
            if p.is_null() {
                return None;
            }
            let v = std::slice::from_raw_parts(p, GlobalSize(h)).to_vec();
            GlobalUnlock(h);
            Some(v)
        })?;
        // a DIB is a BMP without its 14-byte file header
        let header_len = u32::from_le_bytes(dib.get(0..4)?.try_into().ok()?) as usize;
        let bits = u16::from_le_bytes(dib.get(14..16)?.try_into().ok()?) as usize;
        let colors = u32::from_le_bytes(dib.get(32..36)?.try_into().ok()?) as usize;
        let compression = u32::from_le_bytes(dib.get(16..20)?.try_into().ok()?);
        let masks = if compression == 3 && header_len == 40 { 12 } else { 0 };
        let palette = if bits <= 8 { (if colors == 0 { 1 << bits } else { colors }) * 4 } else { 0 };
        let offset = 14 + header_len + masks + palette;
        let mut bmp = Vec::with_capacity(dib.len() + 14);
        bmp.extend(b"BM");
        bmp.extend(((dib.len() + 14) as u32).to_le_bytes());
        bmp.extend([0u8; 4]);
        bmp.extend((offset as u32).to_le_bytes());
        bmp.extend(&dib);
        let img = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp).ok()?;
        // 32-bit clipboard bitmaps often carry an empty alpha channel
        let img = if img.color().has_alpha() && img.to_rgba8().pixels().all(|p| p[3] == 0) { image::DynamicImage::ImageRgb8(img.to_rgb8()) } else { img };
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).ok()?;
        Some(out.into_inner())
    }
}

/// Renames files (old, new full paths). Goes through temporary names so swaps work.
#[tauri::command]
pub async fn rename_files(pairs: Vec<(String, String)>) -> Result<usize, String> {
    let pairs: Vec<(PathBuf, PathBuf)> = pairs.into_iter().map(|(a, b)| (PathBuf::from(a), PathBuf::from(b))).filter(|(a, b)| a != b).collect();
    let sources: std::collections::HashSet<_> = pairs.iter().map(|(a, _)| a.to_string_lossy().to_lowercase()).collect();
    let mut seen = std::collections::HashSet::new();
    for (_, b) in &pairs {
        let key = b.to_string_lossy().to_lowercase();
        if !seen.insert(key.clone()) {
            return Err(format!("two files would be named {}", b.file_name().unwrap_or_default().to_string_lossy()));
        }
        if b.exists() && !sources.contains(&key) {
            return Err(format!("{} already exists", b.file_name().unwrap_or_default().to_string_lossy()));
        }
    }
    let mut temps = Vec::new();
    for (i, (a, _)) in pairs.iter().enumerate() {
        let t = a.with_file_name(format!(".shiftdrop-rename-{}-{i}", std::process::id()));
        std::fs::rename(a, &t).map_err(|e| format!("{}: {e}", a.display()))?;
        temps.push(t);
    }
    for ((_, b), t) in pairs.iter().zip(&temps) {
        std::fs::rename(t, b).map_err(|e| format!("{}: {e}", b.display()))?;
    }
    Ok(pairs.len())
}

/// "ShiftDrop" in Explorer's right-click menu (per user, no admin rights needed).
#[tauri::command]
pub async fn context_menu(enable: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        for root in [r"HKCU\Software\Classes\*\shell\ShiftDrop", r"HKCU\Software\Classes\Directory\shell\ShiftDrop"] {
            let reg = |args: &[&str]| crate::engines::tools::quiet(Path::new("reg")).args(args).output().map(|o| o.status.success()).unwrap_or(false);
            if enable {
                let cmd = format!("\"{}\" --open \"%1\"", exe.display());
                let icon = format!("{},0", exe.display());
                let ok = reg(&["add", root, "/ve", "/d", "ShiftDrop…", "/f"])
                    && reg(&["add", root, "/v", "Icon", "/d", &icon, "/f"])
                    && reg(&["add", root, "/v", "MultiSelectModel", "/d", "Player", "/f"])
                    && reg(&["add", &format!(r"{root}\command"), "/ve", "/d", &cmd, "/f"]);
                if !ok {
                    return Err("couldn't add the menu entry".into());
                }
            } else {
                reg(&["delete", root, "/f"]);
            }
        }
    }
    let _ = enable;
    Ok(())
}

#[tauri::command]
pub async fn context_menu_on() -> bool {
    #[cfg(windows)]
    return crate::engines::tools::quiet(Path::new("reg"))
        .args(["query", r"HKCU\Software\Classes\*\shell\ShiftDrop\command"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    #[cfg(not(windows))]
    false
}
