use anyhow::{anyhow, bail, Context, Result};
use std::{
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, OnceLock,
    },
    time::{Duration, Instant},
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
    let s = probe(&["-select_streams", "v:0", "-show_entries", "stream=width,height:stream_tags=rotate:stream_side_data=rotation", "-of", "json"], file).ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    let st = &v["streams"][0];
    let (w, h) = (st["width"].as_u64()? as u32, st["height"].as_u64()? as u32);
    // phones store portrait video sideways with a rotation flag; report the size as it plays
    let rot = st["side_data_list"]
        .as_array()
        .and_then(|l| l.iter().find_map(|d| d["rotation"].as_i64()))
        .or_else(|| st["tags"]["rotate"].as_str().and_then(|r| r.parse().ok()))
        .unwrap_or(0);
    Some(if rot.rem_euclid(180) == 90 { (h, w) } else { (w, h) })
}

/// Ends a program and everything it started (Office, Edge and LibreOffice spawn helpers).
pub fn kill_tree(pid: u32) {
    let _ = quiet(Path::new("taskkill")).args(["/F", "/T", "/PID", &pid.to_string()]).output();
}

/// Stops a program that has gone quiet for `idle` (stuck on a hidden dialog, a damaged file…).
/// `touch()` marks progress; dropping the guard stops watching.
pub struct Watchdog {
    done: Arc<AtomicBool>,
    fired: Arc<AtomicBool>,
    last: Arc<AtomicU64>,
    start: Instant,
}

impl Watchdog {
    pub fn new(pid: u32, idle: Duration) -> Self {
        let (done, fired, last, start) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)), Arc::new(AtomicU64::new(0)), Instant::now());
        let (d, f, l) = (done.clone(), fired.clone(), last.clone());
        std::thread::spawn(move || {
            while !d.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(250));
                let quiet_for = start.elapsed().as_millis() as u64 - l.load(Ordering::Relaxed);
                if !d.load(Ordering::Relaxed) && quiet_for > idle.as_millis() as u64 {
                    f.store(true, Ordering::Relaxed);
                    kill_tree(pid);
                    break;
                }
            }
        });
        Self { done, fired, last, start }
    }

    pub fn touch(&self) {
        self.last.store(self.start.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    pub fn fired(&self) -> bool {
        self.fired.load(Ordering::Relaxed)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
    }
}

/// Like `Command::output`, but the program is stopped after `limit`.
pub fn output_within(cmd: &mut Command, limit: Duration) -> Result<Output> {
    let child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let dog = Watchdog::new(child.id(), limit);
    let out = child.wait_with_output()?;
    if dog.fired() {
        bail!("it took longer than {} and was stopped", human(limit));
    }
    Ok(out)
}

fn human(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 120 {
        format!("{} minutes", s / 60)
    } else {
        format!("{s} seconds")
    }
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

    // ffmpeg reports progress twice a second; silence for minutes means it is stuck
    let dog = Watchdog::new(child.id(), Duration::from_secs(300));
    let mut stderr = child.stderr.take().unwrap();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    for line in BufReader::new(child.stdout.take().unwrap()).lines().map_while(Result::ok) {
        dog.touch();
        if let Some(v) = line.strip_prefix("out_time_us=") {
            if let (Ok(us), true) = (v.parse::<f64>(), total > 0.0) {
                progress(((us / 1e6) / total).clamp(0.0, 1.0) as f32);
            }
        }
    }
    let status = child.wait()?;
    let err = err_thread.join().unwrap_or_default();
    if dog.fired() {
        bail!("FFmpeg stopped responding for 5 minutes and was stopped");
    }
    if !status.success() {
        let last = err.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("unknown error");
        bail!("FFmpeg failed: {last}");
    }
    Ok(())
}
