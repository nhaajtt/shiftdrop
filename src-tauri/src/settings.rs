use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub const ACCENTS: [&str; 7] = ["gold", "noir", "oxblood", "navy", "racing", "violet", "graphite"];
pub const THEMES: [&str; 3] = ["system", "light", "dark"];

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    /// Only react to drags that start in File Explorer or on the desktop.
    pub explorer_only: bool,
    pub accent: String,
    pub theme: String,
    /// system | en | vi
    pub lang: String,
    /// VirusTotal API key; empty = only open the public report page
    pub vt_key: String,
    pub watches: Vec<Watch>,
    /// Ctrl+Space previews the file selected in Explorer
    pub peek: bool,
    pub anthropic_key: String,
    pub r2_account: String,
    pub r2_key: String,
    pub r2_secret: String,
    pub r2_bucket: String,
    /// public base URL of the bucket (r2.dev or a custom domain)
    pub r2_public: String,
}

/// A folder whose new files get a recipe automatically.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Watch {
    pub folder: String,
    /// only these extensions (empty = all)
    pub exts: Vec<String>,
    pub title: String,
    /// an engines::Action as JSON
    pub action: serde_json::Value,
}

impl Default for Settings {
    fn default() -> Self {
        Self { enabled: true, explorer_only: true, accent: "violet".into(), theme: "system".into(), vt_key: String::new(), watches: Vec::new(), peek: true, ..Self::blank() }
    }
}

impl Settings {
    fn blank() -> Self {
        Self {
            enabled: true,
            explorer_only: true,
            accent: String::new(),
            theme: String::new(),
            lang: "system".into(),
            vt_key: String::new(),
            watches: Vec::new(),
            peek: true,
            anthropic_key: String::new(),
            r2_account: String::new(),
            r2_key: String::new(),
            r2_secret: String::new(),
            r2_bucket: String::new(),
            r2_public: String::new(),
        }
    }

    /// The API keys and secrets, so they can be sealed on disk.
    fn secrets(&mut self) -> [&mut String; 4] {
        [&mut self.vt_key, &mut self.anthropic_key, &mut self.r2_key, &mut self.r2_secret]
    }
}

/// Secrets are stored encrypted with Windows DPAPI: only this Windows user can read them.
#[cfg(windows)]
mod seal {
    use base64::Engine;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB},
    };

    fn run(data: &[u8], protect: bool) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        let ok = unsafe {
            if protect {
                CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, &mut out)
            } else {
                CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, &mut out)
            }
        };
        if ok == 0 {
            return None;
        }
        let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec() };
        unsafe { LocalFree(out.pbData as _) };
        Some(v)
    }

    pub fn lock(s: &str) -> String {
        if s.is_empty() {
            return String::new();
        }
        run(s.as_bytes(), true).map(|v| format!("dpapi:{}", base64::engine::general_purpose::STANDARD.encode(v))).unwrap_or_default()
    }

    pub fn unlock(s: &str) -> String {
        let Some(b64) = s.strip_prefix("dpapi:") else { return s.to_string() };
        base64::engine::general_purpose::STANDARD.decode(b64).ok().and_then(|v| run(&v, false)).and_then(|v| String::from_utf8(v).ok()).unwrap_or_default()
    }
}

#[cfg(not(windows))]
mod seal {
    pub fn lock(s: &str) -> String {
        s.to_string()
    }
    pub fn unlock(s: &str) -> String {
        s.to_string()
    }
}

pub struct Store(pub Mutex<Settings>);

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    file(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .map(|mut s| {
            for v in s.secrets() {
                *v = seal::unlock(v);
            }
            s
        })
        .unwrap_or_default()
}

static CURRENT: Mutex<Option<Settings>> = Mutex::new(None);

/// Settings for code without an AppHandle (engines).
pub fn current() -> Settings {
    CURRENT.lock().unwrap().clone().unwrap_or_default()
}

pub fn get(app: &AppHandle) -> Settings {
    app.state::<Store>().0.lock().unwrap().clone()
}

pub fn update(app: &AppHandle, f: impl FnOnce(&mut Settings)) {
    let snapshot = {
        let store = app.state::<Store>();
        let mut s = store.0.lock().unwrap();
        f(&mut s);
        *CURRENT.lock().unwrap() = Some(s.clone());
        s.clone()
    };
    if let Some(p) = file(app) {
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut sealed = snapshot.clone();
        for v in sealed.secrets() {
            *v = seal::lock(v);
        }
        let _ = fs::write(p, serde_json::to_string_pretty(&sealed).unwrap_or_default());
    }
    let _ = app.emit("settings", &snapshot);
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    get(&app)
}

/// Applies a partial update from the settings view, e.g. `{"theme": "dark"}`.
#[tauri::command]
pub fn update_settings(app: AppHandle, patch: serde_json::Value) -> Result<Settings, String> {
    let mut v = serde_json::to_value(get(&app)).map_err(|e| e.to_string())?;
    if let (Some(o), Some(p)) = (v.as_object_mut(), patch.as_object()) {
        for (k, val) in p {
            o.insert(k.clone(), val.clone());
        }
    }
    let new: Settings = serde_json::from_value(v).map_err(|e| e.to_string())?;
    update(&app, |s| *s = new.clone());
    Ok(new)
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if on { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn secrets_round_trip() {
        let locked = super::seal::lock("sk-test-123");
        assert!(locked.starts_with("dpapi:") && !locked.contains("sk-test"));
        assert_eq!(super::seal::unlock(&locked), "sk-test-123");
        assert_eq!(super::seal::unlock("plain"), "plain");
    }
}
