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
    /// folder of .yar rules for the YARA scan
    pub yara_rules: String,
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
            yara_rules: String::new(),
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

    /// A value that is still sealed (could not be opened) is written back as it was.
    pub fn lock(s: &str) -> String {
        if s.is_empty() || s.starts_with("dpapi:") {
            return s.to_string();
        }
        run(s.as_bytes(), true).map(|v| format!("dpapi:{}", base64::engine::general_purpose::STANDARD.encode(v))).unwrap_or_default()
    }

    /// None when the value was sealed by another Windows account or PC.
    pub fn unlock(s: &str) -> Option<String> {
        let Some(b64) = s.strip_prefix("dpapi:") else { return Some(s.to_string()) };
        base64::engine::general_purpose::STANDARD.decode(b64).ok().and_then(|v| run(&v, false)).and_then(|v| String::from_utf8(v).ok())
    }
}

#[cfg(not(windows))]
mod seal {
    pub fn lock(s: &str) -> String {
        s.to_string()
    }
    pub fn unlock(s: &str) -> Option<String> {
        Some(s.to_string())
    }
}

/// What the windows see instead of a saved key; sending it back leaves the key unchanged.
const MASK: &str = "••••••••";

/// A copy for the web views: keys are never handed to them, only whether one is set.
fn masked(s: &Settings) -> Settings {
    let mut s = s.clone();
    for v in s.secrets() {
        if !v.is_empty() {
            *v = MASK.into();
        }
    }
    s
}

/// The keys in memory. One that could not be opened stays sealed (so it isn't wiped on the next save)
/// and is treated as missing.
fn usable(v: &str) -> &str {
    if v.starts_with("dpapi:") {
        ""
    } else {
        v
    }
}

pub struct Store(pub Mutex<Settings>);

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

/// True before the settings file has ever been written (first launch).
pub fn first_run(app: &AppHandle) -> bool {
    file(app).is_some_and(|p| !p.exists())
}

pub fn load(app: &AppHandle) -> Settings {
    let Some(p) = file(app) else { return Settings::default() };
    let Ok(text) = fs::read_to_string(&p) else { return Settings::default() };
    match serde_json::from_str::<Settings>(&text) {
        Ok(mut s) => {
            for v in s.secrets() {
                match seal::unlock(v) {
                    Some(open) => *v = open,
                    None => crate::log("settings: a saved key could not be opened on this account; it needs to be entered again"),
                }
            }
            s
        }
        Err(e) => {
            // keep the broken file for the user instead of silently overwriting it with defaults
            let _ = fs::copy(&p, p.with_extension("json.bad"));
            crate::log(&format!("settings: unreadable ({e}); kept a copy as settings.json.bad and started from defaults"));
            Settings::default()
        }
    }
}

static CURRENT: Mutex<Option<Settings>> = Mutex::new(None);

/// Settings for code without an AppHandle (engines).
pub fn current() -> Settings {
    let mut s = CURRENT.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default();
    for v in s.secrets() {
        *v = usable(v).to_string();
    }
    s
}

pub fn get(app: &AppHandle) -> Settings {
    app.state::<Store>().0.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn update(app: &AppHandle, f: impl FnOnce(&mut Settings)) {
    let _ = try_update(app, |s| {
        f(s);
        Ok(())
    });
}

/// Changes the settings and saves them while holding the lock, so two changes can't be saved out of order.
fn try_update(app: &AppHandle, f: impl FnOnce(&mut Settings) -> Result<(), String>) -> Result<Settings, String> {
    let store = app.state::<Store>();
    let mut s = store.0.lock().unwrap_or_else(|e| e.into_inner());
    let mut next = s.clone();
    f(&mut next)?;
    *s = next;
    *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) = Some(s.clone());
    #[cfg(windows)]
    crate::win::set_peek(s.enabled && s.peek);
    if let Some(p) = file(app) {
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut sealed = s.clone();
        for v in sealed.secrets() {
            *v = seal::lock(v);
        }
        // write a new file and swap it in, so a crash mid-write can't leave half a file
        let tmp = p.with_extension("json.tmp");
        let saved = fs::write(&tmp, serde_json::to_string_pretty(&sealed).unwrap_or_default()).and_then(|_| fs::rename(&tmp, &p));
        if let Err(e) = saved {
            crate::log(&format!("settings: could not save: {e}"));
        }
    }
    let out = masked(&s);
    drop(s);
    let _ = app.emit("settings", &out);
    Ok(out)
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Settings {
    masked(&get(&app))
}

/// Applies a partial update from the settings view, e.g. `{"theme": "dark"}`.
#[tauri::command]
pub async fn update_settings(app: AppHandle, patch: serde_json::Value) -> Result<Settings, String> {
    try_update(&app, |s| {
        let keep: Vec<String> = s.secrets().iter().map(|v| v.to_string()).collect();
        let mut v = serde_json::to_value(&*s).map_err(|e| e.to_string())?;
        if let (Some(o), Some(p)) = (v.as_object_mut(), patch.as_object()) {
            for (k, val) in p {
                o.insert(k.clone(), val.clone());
            }
        }
        *s = serde_json::from_value(v).map_err(|e| e.to_string())?;
        // the mask coming back means "unchanged"
        for (v, old) in s.secrets().into_iter().zip(keep) {
            if v.as_str() == MASK {
                *v = old;
            }
        }
        Ok(())
    })
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub async fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
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
        assert_eq!(super::seal::unlock(&locked).as_deref(), Some("sk-test-123"));
        assert_eq!(super::seal::unlock("plain").as_deref(), Some("plain"));
        // a key sealed elsewhere is kept as it was, not wiped
        assert_eq!(super::seal::unlock("dpapi:AAAA"), None);
        assert_eq!(super::seal::lock("dpapi:AAAA"), "dpapi:AAAA");
    }
}
