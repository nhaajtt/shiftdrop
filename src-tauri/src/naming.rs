use std::path::{Path, PathBuf};

pub fn stem(src: &Path) -> String {
    src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into())
}

pub fn ext(src: &Path) -> String {
    src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// `<dir>/<stem>[ <suffix>].<ext>`, never overwriting: adds " (2)", " (3)"… when taken.
pub fn out_path(src: &Path, suffix: Option<&str>, ext: &str) -> PathBuf {
    let dir = src.parent().unwrap_or_else(|| Path::new("."));
    let base = match suffix {
        Some(s) => format!("{} {}", stem(src), s),
        None => stem(src),
    };
    unique(dir, &base, ext)
}

pub fn unique(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    p
}

/// A new folder next to `src` for multi-file output, e.g. "Report pages".
pub fn out_dir(src: &Path, suffix: &str) -> std::io::Result<PathBuf> {
    let dir = src.parent().unwrap_or_else(|| Path::new("."));
    let base = format!("{} {}", stem(src), suffix);
    let mut p = dir.join(&base);
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base} ({n})"));
        n += 1;
    }
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

pub fn temp_file(ext: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join("shiftdrop");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{}-{}.{ext}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Image,
    Video,
    Audio,
    Pdf,
    Text,
    Archive,
    Exe,
    Office,
    Other,
}

pub fn kind(p: &Path) -> Kind {
    match ext(p).as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tif" | "tiff" | "ico" | "avif" | "heic" | "heif" | "jxl" | "jxr" | "dng" | "cr2"
        | "cr3" | "nef" | "arw" | "orf" | "rw2" | "raf" => Kind::Image,
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "wmv" | "flv" | "mts" | "m2ts" | "3gp" => Kind::Video,
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" | "opus" | "wma" | "aiff" | "aif" => Kind::Audio,
        "pdf" => Kind::Pdf,
        "txt" | "md" | "markdown" | "csv" | "tsv" | "json" | "log" | "xml" | "html" | "htm" | "yaml" | "yml" | "ini" | "cfg" | "conf"
        | "toml" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "py" | "rs" | "c" | "h" | "cpp" | "hpp" | "cc" | "cs" | "java" | "kt"
        | "go" | "rb" | "php" | "sh" | "bash" | "ps1" | "bat" | "cmd" | "sql" | "css" | "scss" | "lua" | "swift" | "dart" | "vue"
        | "svelte" => Kind::Text,
        "doc" | "docx" | "docm" | "odt" | "rtf" | "xls" | "xlsx" | "xlsm" | "ods" | "ppt" | "pptx" | "odp" => Kind::Office,
        "zip" | "tar" | "gz" | "tgz" | "7z" | "rar" | "bz2" | "xz" | "zst" => Kind::Archive,
        "exe" | "dll" | "sys" | "ocx" | "scr" | "cpl" | "drv" | "efi" | "so" | "elf" => Kind::Exe,
        _ => Kind::Other,
    }
}
