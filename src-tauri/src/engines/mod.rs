pub mod ai;
pub mod dev;
pub mod docx;
pub mod files;
pub mod image;
pub mod imagex;
pub mod media;
pub mod office;
pub mod pdf;
pub mod pdfwrite;
pub mod pdfx;
pub mod security;
pub mod text;
pub mod tools;
#[cfg(test)]
mod tests;

use crate::naming::{kind, Kind};
use anyhow::{bail, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Low,
    Medium,
    High,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Action {
    Convert { to: String },
    Compress { level: Level },
    /// Images: percent of the original size. Video: target height in pixels.
    Resize { value: u32 },
    Rotate { deg: u32 },
    StripMetadata,
    ExtractAudio { to: String },
    Mute,
    Merge,
    Split,
    /// Rectangles are fractions (0–1) of the image: [x, y, w, h].
    Crop { rect: [f64; 4] },
    AddBg { color: String, padding: f64, radius: f64, shadow: f64 },
    Redact { boxes: Vec<[f64; 4]> },
    Trim { start: f64, end: f64 },
    FormatJson { pretty: bool },
    Zip,
    Extract,
    Hash,
    CopyPath,
    CopyText,
    Strings,
    PeInfo,
    Identify,
    VerifySignature,
    /// md5 | sha1 | sha256 | sha512 → clipboard; all → report file
    Hashes { algo: String },
    Sandbox,
    Encrypt { password: String },
    Decrypt { password: String },
    Shred,
    Locks,
    VirusTotal,
    /// capa | floss | upx
    Tool { tool: String },
    Base64 { decode: bool },
    /// plain | quoted | wsl | url | markdown
    CopyPathAs { style: String },
    Diff,
    /// Re-encode to fit in this many megabytes (e.g. Discord, Zalo, email limits).
    TargetSize { mb: f64 },
    Speed { factor: f64 },
    /// h | v
    Flip { dir: String },
    /// one PNG every `every` seconds into a folder
    Frames { every: f64 },
    Thumbnail,
    /// images → video, `seconds` per image
    Slideshow { seconds: f64 },
    /// denoise | normalize
    AudioFix { mode: String },
    /// speech → .srt with whisper.cpp; `burn` also draws them into the video
    Transcribe { burn: bool },
    /// clipboard | txt
    Ocr { to: String },
    QrRead,
    /// text file → QR code image
    QrMake,
    /// .ico for apps, or a favicon folder for websites
    IconSet { web: bool },
    Palette,
    Watermark { text: String },
    ContactSheet,
    /// image → SVG
    Vectorize,
    RemoveBg,
    BlurFaces,
    Upscale { scale: u32 },
    /// keep these pages (0-based) in this order, each rotated by `rotate[i]` degrees
    Pages { order: Vec<u32>, #[serde(default)] rotate: Vec<u32> },
    PageNumbers,
    /// a PNG (data URL) placed on `page` at [x, y, w, h] fractions from the top-left
    Stamp { page: u32, rect: [f64; 4], png: String },
    /// text of every page, OCR for scanned ones
    PdfText,
    ExtractImages,
    Protect { password: String },
    Unprotect { password: String },
    /// a recipe: each step runs on what the previous one made
    Chain { steps: Vec<Action> },
    /// the user's own command line; {file} {dir} {name} {stem} {ext} are filled in
    Custom { command: String },
    /// translate | summarize, with the user's Claude API key
    AskClaude { task: String, #[serde(default)] lang: String },
}

thread_local! {
    static NOTES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A one-line result shown in the toast (for actions that don't create files).
pub fn note(msg: &str) {
    NOTES.with(|n| n.borrow_mut().push(msg.to_string()));
}

/// Notes left by the actions run on this thread since the last call.
pub fn take_notes() -> Vec<String> {
    NOTES.with(|n| std::mem::take(&mut *n.borrow_mut()))
}

/// Actions that treat the whole selection as one unit.
pub fn is_group(a: &Action) -> bool {
    files::is_group(a) || matches!(a, Action::Merge | Action::Diff | Action::Shred | Action::Slideshow { .. } | Action::ContactSheet) || matches!(a, Action::Hashes { algo } if algo != "all")
}

pub type Progress<'a> = &'a dyn Fn(f32);

/// Runs one action on one file (or on all files for `Merge`). Returns created files.
pub fn run(paths: &[PathBuf], action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    if let Action::Merge = action {
        return merge(paths, progress);
    }
    if let Action::Chain { steps } = action {
        return chain(paths, steps, progress);
    }
    if let Action::AskClaude { task, lang } = action {
        return crate::share::ask_claude(&paths[0], task, lang).map(|p| vec![p]);
    }
    if let Action::Custom { command } = action {
        return custom(&paths[0], command);
    }
    if let Action::Slideshow { seconds } = action {
        return media::slideshow(paths, *seconds, progress);
    }
    if files::is_group(action) || matches!(action, Action::Extract | Action::Strings | Action::PeInfo) {
        return files::run(paths, action, progress);
    }
    if security::handles(action) {
        return security::run(paths, action, progress);
    }
    if kind(&paths[0]) == Kind::Pdf && pdfx::handles(action) {
        return pdfx::run(&paths[0], action, progress);
    }
    if ai::handles(action) {
        return ai::run(paths, action, progress);
    }
    if imagex::handles(action) {
        return imagex::run(paths, action, progress);
    }
    if dev::handles(action) {
        return dev::run(paths, action, progress);
    }
    let [src] = paths else { bail!("expected one file") };
    match kind(src) {
        Kind::Text => text::run(src, action, progress),
        Kind::Image => image::run(src, action, progress),
        Kind::Video | Kind::Audio => media::run(src, action, progress),
        Kind::Pdf => pdf::run(src, action, progress),
        Kind::Office => office::run(src, action, progress),
        Kind::Archive | Kind::Exe | Kind::Other => bail!("unsupported file type"),
    }
}

fn chain(paths: &[PathBuf], steps: &[Action], progress: Progress) -> Result<Vec<PathBuf>> {
    let n = steps.len().max(1) as f32;
    let mut cur = paths.to_vec();
    let mut made = Vec::new();
    for (k, step) in steps.iter().enumerate() {
        let units: Vec<Vec<PathBuf>> = if is_group(step) { vec![cur.clone()] } else { cur.iter().map(|p| vec![p.clone()]).collect() };
        let mut next = Vec::new();
        for u in units {
            next.extend(run(&u, step, &|p| progress((k as f32 + p) / n))?);
        }
        // steps that only copy or report (hash, copy path…) pass their input on
        if next.is_empty() {
            next = cur.clone();
        }
        made.extend(next.iter().filter(|p| !paths.contains(p)).cloned());
        cur = next;
    }
    // keep only the final results, not the files made along the way
    for p in made.iter().filter(|p| !cur.contains(p)) {
        let _ = if p.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
    }
    Ok(cur.into_iter().filter(|p| !paths.contains(p)).collect())
}

/// Runs a user-defined command on one file, from the file's folder.
fn custom(src: &std::path::Path, command: &str) -> Result<Vec<PathBuf>> {
    use crate::naming::{ext, stem};
    let dir = src.parent().map(|d| d.to_path_buf()).unwrap_or_default();
    let q = |s: &str| format!("\"{s}\"");
    let line = command
        .replace("{file}", &q(&src.display().to_string()))
        .replace("{dir}", &q(&dir.display().to_string()))
        .replace("{name}", &src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
        .replace("{stem}", &stem(src))
        .replace("{ext}", &ext(src));
    let before: std::collections::HashSet<PathBuf> = std::fs::read_dir(&dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    #[cfg(windows)]
    let o = {
        use std::os::windows::process::CommandExt;
        tools::quiet(std::path::Path::new("cmd")).arg("/D").arg("/C").raw_arg(&line).current_dir(&dir).output()?
    };
    #[cfg(not(windows))]
    let o = std::process::Command::new("sh").arg("-c").arg(&line).current_dir(&dir).output()?;
    if !o.status.success() {
        let err = String::from_utf8_lossy(&o.stderr);
        bail!("{}", err.lines().last().unwrap_or("the command failed"));
    }
    if let Some(last) = String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.trim().is_empty()).last() {
        note(last.trim());
    }
    // whatever appeared in the folder counts as the result
    Ok(std::fs::read_dir(&dir).map(|r| r.flatten().map(|e| e.path()).filter(|p| !before.contains(p)).collect()).unwrap_or_default())
}

fn merge(paths: &[PathBuf], progress: Progress) -> Result<Vec<PathBuf>> {
    if paths.len() < 2 {
        bail!("merge needs at least two files");
    }
    let kinds: Vec<Kind> = paths.iter().map(|p| kind(p)).collect();
    let all = |k: Kind| kinds.iter().all(|x| *x == k);
    if all(Kind::Video) || all(Kind::Audio) {
        media::merge(paths, progress)
    } else if kinds.iter().all(|k| matches!(k, Kind::Pdf | Kind::Image)) {
        pdf::merge(paths, progress)
    } else {
        bail!("these files can't be merged together")
    }
}
