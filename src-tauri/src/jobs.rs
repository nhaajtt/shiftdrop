use crate::engines::{self, Action};
use serde::Serialize;
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

/// One worker runs jobs in order so a batch doesn't saturate every core at once.
pub fn start(app: AppHandle) -> Queue {
    let (tx, rx) = channel::<Job>();
    std::thread::spawn(move || {
        for job in rx {
            run(&app, job);
        }
    });
    Queue(Mutex::new(tx))
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
        match engines::run(unit, &job.action, &progress) {
            Ok(out) => outputs.extend(out),
            Err(e) => {
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
    emit(if done == 0 { "error" } else { "done" }, done, 1.0, &outputs, error, note);
}

#[tauri::command]
pub fn run_action(app: AppHandle, paths: Vec<String>, action: Action, title: String) -> Result<u64, String> {
    crate::log(&format!("run_action {action:?} on {paths:?}"));
    queue(&app, paths.into_iter().map(PathBuf::from).collect(), action, title)
}

/// Adds a job to the queue and shows the toast.
pub fn queue(app: &AppHandle, paths: Vec<PathBuf>, action: Action, title: String) -> Result<u64, String> {
    use tauri::Manager;
    static ID: AtomicU64 = AtomicU64::new(1);
    // folders only make sense for zip and copy-path
    let folders_ok = matches!(action, Action::Zip | Action::CopyPath | Action::CopyPathAs { .. });
    let paths: Vec<PathBuf> = paths.into_iter().filter(|p| p.is_file() || (folders_ok && p.is_dir())).collect();
    if paths.is_empty() {
        return Err("no files".into());
    }
    let id = ID.fetch_add(1, Ordering::Relaxed);
    crate::windows::show_toast(app);
    app.state::<Queue>().0.lock().unwrap().send(Job { id, title, paths, action }).map_err(|e| e.to_string())?;
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
