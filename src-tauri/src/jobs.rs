use crate::engines::{self, Action};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{channel, Sender},
        Mutex,
    },
};
use tauri::{AppHandle, Emitter};

struct Job {
    id: u64,
    title: String,
    paths: Vec<PathBuf>,
    action: Action,
}

#[derive(Clone, Serialize)]
struct Update<'a> {
    id: u64,
    title: &'a str,
    state: &'a str,
    done: usize,
    total: usize,
    percent: f32,
    outputs: &'a [PathBuf],
    error: Option<String>,
    note: Option<String>,
}

pub struct Queue(Mutex<Sender<Job>>);

/// A finished job as kept in the Activity list (also across restarts).
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: u64,
    /// seconds since 1970
    pub time: u64,
    pub title: String,
    pub files: Vec<String>,
    pub state: String,
    pub outputs: Vec<String>,
    pub error: Option<String>,
    pub note: Option<String>,
    /// what to run again for "Retry"; never kept when it holds a password or a signature
    pub action: Option<serde_json::Value>,
}

const KEEP: usize = 200;
static HISTORY: Mutex<Option<Vec<Record>>> = Mutex::new(None);

fn history_file(app: &AppHandle) -> Option<PathBuf> {
    use tauri::Manager;
    app.path().app_config_dir().ok().map(|d| d.join("history.json"))
}

fn with_history<T>(app: &AppHandle, f: impl FnOnce(&mut Vec<Record>) -> T) -> T {
    let mut h = HISTORY.lock().unwrap_or_else(|e| e.into_inner());
    let list = h.get_or_insert_with(|| {
        history_file(app).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    });
    f(list)
}

fn remember(app: &AppHandle, r: Record) {
    let snapshot = with_history(app, |list| {
        list.insert(0, r);
        list.truncate(KEEP);
        serde_json::to_string(list).unwrap_or_default()
    });
    if let Some(p) = history_file(app) {
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        let _ = std::fs::write(p, snapshot);
    }
    let _ = app.emit("history", ());
}

#[tauri::command]
pub async fn get_history(app: AppHandle) -> Vec<Record> {
    with_history(&app, |l| l.clone())
}

#[tauri::command]
pub async fn clear_history(app: AppHandle) {
    with_history(&app, |l| l.clear());
    if let Some(p) = history_file(&app) {
        let _ = std::fs::remove_file(p);
    }
    let _ = app.emit("history", ());
}

/// Debug text of an action with passwords blanked out, for the log.
pub fn redacted(action: &Action) -> String {
    redact(&format!("{action:?}"))
}

/// Blanks the values that must never reach the log: passwords and the signature image.
pub fn redact(text: &str) -> String {
    ["password: \"", "png: \""].iter().fold(text.to_string(), |t, key| blank(&t, key))
}

fn blank(text: &str, key: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(key) {
        let start = i + key.len();
        out.push_str(&rest[..start]);
        out.push_str("•••");
        rest = &rest[start..];
        // skip to the closing quote (a quote inside the password is escaped as \")
        let mut end = 0;
        let bytes = rest.as_bytes();
        while end < bytes.len() && !(bytes[end] == b'"' && (end == 0 || bytes[end - 1] != b'\\')) {
            end += 1;
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// One worker runs jobs in order so a batch doesn't saturate every core at once.
pub fn start(app: AppHandle) -> Queue {
    let (tx, rx) = channel::<Job>();
    std::thread::spawn(move || {
        for job in rx {
            // a crash inside one engine must not take the whole queue down
            let (id, title, paths) = (job.id, job.title.clone(), job.paths.clone());
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&app, job))).is_err() {
                crate::naming::discard();
                crashed(&app, id, &title, &paths);
            }
        }
    });
    Queue(Mutex::new(tx))
}

fn crashed(app: &AppHandle, id: u64, title: &str, paths: &[PathBuf]) {
    let error = "ShiftDrop hit an internal error on this file. Nothing was changed.".to_string();
    crate::log(&format!("job {id} panicked"));
    remember(
        app,
        Record {
            id,
            time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            title: title.into(),
            files: paths.iter().map(|p| p.display().to_string()).collect(),
            state: "error".into(),
            outputs: vec![],
            error: Some(error.clone()),
            note: None,
            action: None,
        },
    );
    let _ = app.emit("job", Update { id, title, state: "error", done: 0, total: paths.len().max(1), percent: 1.0, outputs: &[], error: Some(error), note: None });
}

fn run(app: &AppHandle, job: Job) {
    // Merge is one unit of work; everything else runs once per file.
    let units: Vec<Vec<PathBuf>> =
        if engines::is_group(&job.action) { vec![job.paths.clone()] } else { job.paths.iter().map(|p| vec![p.clone()]).collect() };
    let total = units.len();
    let mut outputs = Vec::new();
    let mut errors = Vec::new();
    let emit = |state: &str, done: usize, percent: f32, outputs: &[PathBuf], error: Option<String>, note: Option<String>| {
        let _ = app.emit("job", Update { id: job.id, title: &job.title, state, done, total, percent, outputs, error, note });
    };
    emit("running", 0, 0.0, &[], None, None);
    engines::take_notes();
    for (i, unit) in units.iter().enumerate() {
        let progress = |p: f32| emit("running", i, (i as f32 + p) / total as f32, &[], None, None);
        crate::naming::begin();
        match engines::run(unit, &job.action, &progress) {
            Ok(out) => {
                crate::naming::finish();
                outputs.extend(out)
            }
            Err(e) => {
                crate::naming::discard();
                let name = unit[0].file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                errors.push(if total > 1 { format!("{name}: {e}") } else { e.to_string() });
            }
        }
    }
    let done = total - errors.len();
    let error = (!errors.is_empty()).then(|| errors.join("\n"));
    crate::log(&format!("job {} done: {outputs:?} errors: {errors:?}", job.id));
    crate::watch::made(&outputs);
    let notes = engines::take_notes();
    let note = (!notes.is_empty()).then(|| notes.join("
"));
    let state = if done == 0 { "error" } else { "done" };
    remember(
        app,
        Record {
            id: job.id,
            time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            title: job.title.clone(),
            files: job.paths.iter().map(|p| p.display().to_string()).collect(),
            state: if errors.is_empty() { state.into() } else if done > 0 { "partial".into() } else { "error".into() },
            outputs: outputs.iter().map(|p| p.display().to_string()).collect(),
            error: error.clone(),
            note: note.clone(),
            action: Some(&job.action).filter(|a| !redacted(a).contains("•••")).and_then(|a| serde_json::to_value(a).ok()),
        },
    );
    emit(state, done, 1.0, &outputs, error, note);
}

#[tauri::command]
pub async fn run_action(app: AppHandle, paths: Vec<String>, action: Action, title: String) -> Result<u64, String> {
    crate::log(&format!("run_action {} on {paths:?}", redacted(&action)));
    queue(&app, paths.into_iter().map(PathBuf::from).collect(), action, title)
}

/// Adds a job to the queue and shows the toast.
pub fn queue(app: &AppHandle, paths: Vec<PathBuf>, action: Action, title: String) -> Result<u64, String> {
    use tauri::Manager;
    // ids keep growing across restarts so the Activity list never mixes two jobs up
    static ID: AtomicU64 = AtomicU64::new(0);
    let _ = ID.compare_exchange(0, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1), Ordering::SeqCst, Ordering::SeqCst);
    // folders only make sense for zip and copy-path
    let folders_ok = matches!(action, Action::Zip | Action::CopyPath | Action::CopyPathAs { .. });
    let paths: Vec<PathBuf> = paths.into_iter().filter(|p| p.is_file() || (folders_ok && p.is_dir())).collect();
    if paths.is_empty() {
        return Err("no files".into());
    }
    let id = ID.fetch_add(1, Ordering::Relaxed);
    crate::windows::show_toast(app);
    app.state::<Queue>().0.lock().unwrap_or_else(|e| e.into_inner()).send(Job { id, title, paths, action }).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Opens File Explorer with the file selected.
#[tauri::command]
pub async fn reveal(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("file not found".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // raw_arg keeps the quotes Explorer expects around paths with spaces
        std::process::Command::new("explorer").raw_arg(format!("/select,\"{}\"", p.display())).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg("-R").arg(&p).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn passwords_never_reach_the_log() {
        let s = super::redact(r#"Protect { password: "Secr\"et-1" } and Decrypt { password: "abc" } Stamp { page: 0, png: "data:image/png;base64,iVBORw0K" }"#);
        assert!(!s.contains("Secr") && !s.contains("abc") && !s.contains("iVBOR"), "{s}");
        assert_eq!(s.matches("•••").count(), 3);
    }
}
