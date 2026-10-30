//! Watched folders: a new file that lands in one (a download, a scan, a screenshot) runs the
//! chosen recipe on its own. Polls every two seconds; a file counts once its size stops changing.

use crate::{engines::Action, settings};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};
use tauri::AppHandle;

/// Files ShiftDrop made itself, so a recipe never runs on its own output.
static MADE: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);

pub fn made(outputs: &[PathBuf]) {
    MADE.lock().unwrap().get_or_insert_with(HashSet::new).extend(outputs.iter().cloned());
}

fn is_made(p: &PathBuf) -> bool {
    MADE.lock().unwrap().as_ref().is_some_and(|m| m.contains(p))
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        // folder → files already there (or handled) and sizes of files still being written
        let mut known: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();
        let mut growing: HashMap<PathBuf, u64> = HashMap::new();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let watches = settings::get(&app).watches;
            known.retain(|dir, _| watches.iter().any(|w| PathBuf::from(&w.folder) == *dir));
            for w in &watches {
                let dir = PathBuf::from(&w.folder);
                let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                let now: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
                let Some(seen) = known.get_mut(&dir) else {
                    // first look: what is already there is left alone
                    known.insert(dir, now.into_iter().collect());
                    continue;
                };
                for p in now {
                    if seen.contains(&p) || is_made(&p) || ignorable(&p) {
                        continue;
                    }
                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    if growing.get(&p) != Some(&size) || size == 0 {
                        growing.insert(p, size);
                        continue;
                    }
                    growing.remove(&p);
                    seen.insert(p.clone());
                    if !w.exts.is_empty() && !w.exts.iter().any(|e| crate::naming::ext(&p) == e.trim().trim_start_matches('.').to_lowercase()) {
                        continue;
                    }
                    match serde_json::from_value::<Action>(w.action.clone()) {
                        Ok(action) if !unattended_ok(&action) => crate::log(&format!("watch {}: action not allowed unattended", w.folder)),
                        Ok(action) => {
                            crate::log(&format!("watch {}: {}", w.folder, p.display()));
                            let _ = crate::jobs::queue(&app, vec![p], action, w.title.clone());
                        }
                        Err(e) => crate::log(&format!("watch: bad action {e}")),
                    }
                }
            }
        }
    });
}

/// Files arriving in a watched folder can come from anyone (a download, a share). Nothing that
/// runs commands, destroys data or sends the file off this PC is done without the user present.
fn unattended_ok(a: &Action) -> bool {
    // allow-list: an action added later stays off for watched folders until it is put here
    match a {
        Action::Chain { steps } => steps.iter().all(unattended_ok),
        Action::Convert { .. }
        | Action::Compress { .. }
        | Action::Resize { .. }
        | Action::Rotate { .. }
        | Action::StripMetadata
        | Action::ExtractAudio { .. }
        | Action::Mute
        | Action::Crop { .. }
        | Action::AddBg { .. }
        | Action::Trim { .. }
        | Action::FormatJson { .. }
        | Action::Zip
        | Action::Hash
        | Action::Hashes { .. }
        | Action::Identify
        | Action::TargetSize { .. }
        | Action::Speed { .. }
        | Action::Flip { .. }
        | Action::Thumbnail
        | Action::AudioFix { .. }
        | Action::Ocr { .. }
        | Action::QrRead
        | Action::IconSet { .. }
        | Action::Palette
        | Action::Watermark { .. }
        | Action::Vectorize
        | Action::RemoveBg
        | Action::BlurFaces
        | Action::Upscale { .. }
        | Action::PageNumbers
        | Action::PdfText
        | Action::ExtractImages => true,
        _ => false,
    }
}

/// Browser partial downloads and temporary files.
fn ignorable(p: &PathBuf) -> bool {
    let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    n.starts_with('~') || n.starts_with('.') || [".crdownload", ".part", ".tmp", ".download", ".partial"].iter().any(|e| n.ends_with(e))
}

#[tauri::command]
pub async fn folder_exists(path: String) -> bool {
    std::path::Path::new(&path).is_dir()
}

#[cfg(test)]
mod tests {
    use crate::engines::Action;

    #[test]
    fn only_safe_actions_run_unattended() {
        assert!(super::unattended_ok(&Action::Convert { to: "png".into() }));
        assert!(super::unattended_ok(&Action::Chain { steps: vec![Action::StripMetadata, Action::Zip] }));
        assert!(!super::unattended_ok(&Action::Shred));
        assert!(!super::unattended_ok(&Action::Chain { steps: vec![Action::Zip, Action::Custom { command: "x".into() }] }));
    }
}
