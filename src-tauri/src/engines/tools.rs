use anyhow::{anyhow, bail, Context, Result};
use std::{
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

static BIN_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Folder that holds bundled helpers (pdfium.dll, optionally ffmpeg.exe / ffprobe.exe).
pub fn set_bin_dir(p: PathBuf) {
    let _ = BIN_DIR.set(p);
}

pub fn bin_dir() -> Option<&'static Path> {
    BIN_DIR.get().map(|p| p.as_path())
}

fn exe(name: &str) -> PathBuf {
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    if let Some(dir) = bin_dir() {
        let p = dir.join(&file);
        if p.exists() {
            return p;
        }
    }
    // the FFmpeg add-on (Settings › Add-ons) keeps both programs in one bin folder
    if let Some(p) = super::security::tool_path("ffmpeg").and_then(|f| f.parent().map(|d| d.join(&file))).filter(|p| p.exists()) {
        return p;
    }
    PathBuf::from(file)
}

/// Keeps console tools from flashing a window.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn command(name: &str) -> Command {
    quiet(&exe(name))
}

/// A command for any program, without a console window popping up.
pub fn quiet(program: &Path) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

fn probe(args: &[&str], file: &Path) -> Result<String> {
    let out = command("ffprobe")
        .args(["-v", "error"])
        .args(args)
        .arg(file)
        .output()
        .context("Video and audio need the FFmpeg add-on: Settings › Add-ons")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn duration(file: &Path) -> f64 {
    probe(&["-show_entries", "format=duration", "-of", "csv=p=0"], file)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

pub fn has_audio(file: &Path) -> bool {
    probe(&["-select_streams", "a", "-show_entries", "stream=index", "-of", "csv=p=0"], file)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn video_size(file: &Path) -> Option<(u32, u32)> {
    let s = probe(&["-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x"], file).ok()?;
    let (w, h) = s.lines().next()?.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

/// Runs ffmpeg with `args` (inputs and outputs included). `total` seconds drives progress.
pub fn ffmpeg(args: &[String], total: f64, progress: &dyn Fn(f32)) -> Result<()> {
    ffmpeg_in(None, args, total, progress)
}

/// Same, run from `dir` (filters like `subtitles` read relative paths more reliably).
pub fn ffmpeg_in(dir: Option<&Path>, args: &[String], total: f64, progress: &dyn Fn(f32)) -> Result<()> {
    let mut cmd = command("ffmpeg");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let mut child = cmd
        .args(["-hide_banner", "-loglevel", "error", "-nostats", "-progress", "pipe:1", "-y"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| anyhow!("Video and audio need the FFmpeg add-on: Settings › Add-ons"))?;

    let mut stderr = child.stderr.take().unwrap();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    for line in BufReader::new(child.stdout.take().unwrap()).lines().map_while(Result::ok) {
        if let Some(v) = line.strip_prefix("out_time_us=") {
            if let (Ok(us), true) = (v.parse::<f64>(), total > 0.0) {
                progress(((us / 1e6) / total).clamp(0.0, 1.0) as f32);
            }
        }
    }
    let status = child.wait()?;
    let err = err_thread.join().unwrap_or_default();
    if !status.success() {
        let last = err.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("unknown error");
        bail!("FFmpeg failed: {last}");
    }
    Ok(())
}
