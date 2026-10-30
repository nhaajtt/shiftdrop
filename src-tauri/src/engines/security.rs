//! Checking and protecting files: real type, signatures, hashes, Windows Sandbox, encryption,
//! secure delete, which program holds a file, VirusTotal (hash only) and analysis tools.

use super::{note, Action, Progress};
use crate::naming::{ext, out_path, stem};
use anyhow::{bail, Context, Result};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub fn handles(a: &Action) -> bool {
    matches!(
        a,
        Action::Identify
            | Action::VerifySignature
            | Action::Hashes { .. }
            | Action::Sandbox
            | Action::Encrypt { .. }
            | Action::Decrypt { .. }
            | Action::Shred
            | Action::Locks
            | Action::VirusTotal
            | Action::Tool { .. }
    )
}

pub fn run(paths: &[PathBuf], action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    let src = &paths[0];
    match action {
        Action::Identify => {
            note(&identify(src)?);
            Ok(vec![])
        }
        Action::VerifySignature => {
            note(&format!("{}: {}", name(src), signature(src)));
            Ok(vec![])
        }
        Action::Hashes { algo } => hashes(paths, algo, progress),
        Action::Sandbox => sandbox(src).map(|_| vec![]),
        Action::Encrypt { password } => encrypt(src, password).map(|p| vec![p]),
        Action::Decrypt { password } => decrypt(src, password).map(|p| vec![p]),
        Action::Shred => {
            for p in paths {
                shred(p)?;
            }
            note(&format!("{} securely deleted", if paths.len() == 1 { name(src) } else { format!("{} files", paths.len()) }));
            Ok(vec![])
        }
        Action::Locks => {
            note(&locks(src)?);
            Ok(vec![])
        }
        Action::VirusTotal => virustotal(src).map(|_| vec![]),
        Action::Tool { tool } => run_tool(src, tool),
        _ => bail!("not available"),
    }
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn head(p: &Path, n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    let got = std::fs::File::open(p).and_then(|mut f| f.read(&mut buf)).unwrap_or(0);
    buf.truncate(got);
    buf
}

// --- Real file type -------------------------------------------------------------------------

const PROGRAM_EXT: &[&str] = &["exe", "dll", "sys", "scr", "com", "cpl", "ocx", "drv", "efi", "msi", "elf", "so", "bin"];
const SCRIPT_EXT: &[&str] = &["bat", "cmd", "ps1", "vbs", "js", "jse", "wsf", "hta", "lnk", "msi", "sh"];

/// What the content really is: (extension, description).
pub fn sniff(p: &Path) -> Option<(String, String)> {
    sniff_bytes(&head(p, 8192))
}

pub fn sniff_bytes(h: &[u8]) -> Option<(String, String)> {
    if h.starts_with(b"MZ") {
        return Some(("exe".into(), "Windows program".into()));
    }
    let t = infer::get(h)?;
    Some((t.extension().to_string(), t.mime_type().to_string()))
}

/// A short warning when a file pretends to be something it isn't, e.g. "invoice.pdf" that is a program.
pub fn disguise(p: &Path) -> Option<String> {
    let e = ext(p);
    let n = name(p).to_lowercase();
    // "invoice.pdf.exe", "photo.jpg   .scr"
    let parts: Vec<&str> = n.split('.').collect();
    if parts.len() >= 3 && (PROGRAM_EXT.contains(&e.as_str()) || SCRIPT_EXT.contains(&e.as_str())) {
        let fake = parts[parts.len() - 2].trim();
        if ["pdf", "jpg", "jpeg", "png", "doc", "docx", "xls", "xlsx", "txt", "mp4", "mp3", "zip"].contains(&fake) {
            return Some(format!("{}: looks like .{fake} but is a program", name(p)));
        }
    }
    if n.contains('\u{202e}') {
        return Some(format!("{}: name uses a hidden right-to-left trick", name(p)));
    }
    let (real, _) = sniff(p)?;
    if real == "exe" && !PROGRAM_EXT.contains(&e.as_str()) {
        return Some(format!("{}: named .{e} but is a Windows program", name(p)));
    }
    None
}

fn identify(p: &Path) -> Result<String> {
    let e = ext(p);
    Ok(match sniff(p) {
        None => format!("{}: type not recognised from its content", name(p)),
        Some((real, desc)) => {
            let same = real == e || (real == "jpg" && e == "jpeg") || (real == "zip" && ["docx", "xlsx", "pptx", "jar", "apk", "odt"].contains(&e.as_str()));
            if same {
                format!("{}: really {desc} (.{real}), matches its name", name(p))
            } else {
                format!("{}: really {desc} (.{real}), but named .{e}", name(p))
            }
        }
    })
}

// --- Signature ----------------------------------------------------------------------------

#[cfg(windows)]
pub fn signature(p: &Path) -> String {
    use windows_sys::Win32::Security::WinTrust::*;
    let wide: Vec<u16> = p.as_os_str().encode_wide_z();
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide.as_ptr(),
        hFile: std::ptr::null_mut(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut data: WINTRUST_DATA = unsafe { std::mem::zeroed() };
    data.cbStruct = std::mem::size_of::<WINTRUST_DATA>() as u32;
    data.dwUIChoice = WTD_UI_NONE;
    data.fdwRevocationChecks = WTD_REVOKE_NONE;
    data.dwUnionChoice = WTD_CHOICE_FILE;
    data.Anonymous.pFile = &mut file;
    data.dwStateAction = WTD_STATEACTION_VERIFY;
    let mut guid = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let r = unsafe { WinVerifyTrust(std::ptr::null_mut(), &mut guid, &mut data as *mut _ as _) };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe { WinVerifyTrust(std::ptr::null_mut(), &mut guid, &mut data as *mut _ as _) };
    match r as u32 {
        0 => "signed, signature valid".into(),
        0x800B0100 => "not signed".into(),
        0x80096010 => "signature INVALID (file changed after signing)".into(),
        0x800B0109 => "signed by an untrusted root".into(),
        0x800B0101 => "signing certificate expired".into(),
        0x800B0111 => "signer is explicitly distrusted".into(),
        e => format!("signature not valid (0x{e:08X})"),
    }
}

#[cfg(not(windows))]
pub fn signature(_: &Path) -> String {
    "signature check is only available on Windows".into()
}

trait WideZ {
    fn encode_wide_z(&self) -> Vec<u16>;
}

#[cfg(windows)]
impl WideZ for std::ffi::OsStr {
    fn encode_wide_z(&self) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        self.encode_wide().chain([0]).collect()
    }
}

// --- Hashes -------------------------------------------------------------------------------

fn digest_all(p: &Path, progress: &dyn Fn(f32)) -> Result<[String; 4]> {
    use sha2::Digest;
    let mut f = std::fs::File::open(p)?;
    let len = f.metadata()?.len().max(1);
    let (mut m, mut s1, mut s256, mut s512) = (md5::Md5::new(), sha1::Sha1::new(), sha2::Sha256::new(), sha2::Sha512::new());
    let mut buf = vec![0u8; 1 << 20];
    let mut read = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        m.update(&buf[..n]);
        s1.update(&buf[..n]);
        s256.update(&buf[..n]);
        s512.update(&buf[..n]);
        read += n as u64;
        progress(read as f32 / len as f32);
    }
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    Ok([hex(&m.finalize()), hex(&s1.finalize()), hex(&s256.finalize()), hex(&s512.finalize())])
}

fn hashes(paths: &[PathBuf], algo: &str, progress: Progress) -> Result<Vec<PathBuf>> {
    let mut lines = Vec::new();
    let mut report = String::new();
    for (i, p) in paths.iter().enumerate() {
        let n = paths.len() as f32;
        let d = digest_all(p, &|f| progress((i as f32 + f) / n))?;
        let pick = match algo {
            "md5" => &d[0],
            "sha1" => &d[1],
            "sha512" => &d[3],
            "all" => {
                report += &format!("{}\n  MD5      {}\n  SHA-1    {}\n  SHA-256  {}\n  SHA-512  {}\n", name(p), d[0], d[1], d[2], d[3]);
                if let Some(ih) = super::files::imphash(p) {
                    report += &format!("  imphash  {ih}\n");
                }
                report += "\n";
                continue;
            }
            _ => &d[2],
        };
        lines.push(if paths.len() > 1 { format!("{pick}  {}", name(p)) } else { pick.clone() });
    }
    if algo == "all" {
        let out = out_path(&paths[0], Some("hashes"), "txt");
        std::fs::write(&out, report)?;
        return Ok(vec![out]);
    }
    crate::clipboard::set(&lines.join("\r\n"))?;
    note(&format!("{} copied", algo.to_uppercase()));
    Ok(vec![])
}

// --- Windows Sandbox ------------------------------------------------------------------------

pub fn sandbox_available() -> bool {
    std::env::var_os("SystemRoot").map(|r| Path::new(&r).join("System32").join("WindowsSandbox.exe").is_file()).unwrap_or(false)
}

/// Opens the program inside Windows Sandbox: its folder is shared read-only and nothing persists.
fn sandbox(src: &Path) -> Result<()> {
    if !sandbox_available() {
        bail!("Windows Sandbox needs Windows Pro or Enterprise with the feature turned on");
    }
    // only a copy of this one file is shared, not the rest of its folder (e.g. all of Downloads)
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis();
    let dir = std::env::temp_dir().join("shiftdrop").join(format!("sandbox-{stamp}"));
    std::fs::create_dir_all(&dir)?;
    std::fs::copy(src, dir.join(name(src)))?;
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let inside = "C:\\Users\\WDAGUtilityAccount\\Desktop\\Sample";
    let wsb = format!(
        "<Configuration><Networking>Disable</Networking><ClipboardRedirection>Disable</ClipboardRedirection><VGpu>Disable</VGpu><AudioInput>Disable</AudioInput><VideoInput>Disable</VideoInput><PrinterRedirection>Disable</PrinterRedirection><MappedFolders><MappedFolder><HostFolder>{}</HostFolder><SandboxFolder>{}</SandboxFolder><ReadOnly>true</ReadOnly></MappedFolder></MappedFolders><LogonCommand><Command>explorer.exe \"{}\"</Command></LogonCommand></Configuration>",
        esc(&dir.display().to_string()),
        esc(inside),
        esc(&format!("{inside}\\{}", name(src)))
    );
    // Windows Sandbox reads this after the job has finished, so it isn't a job temp file
    let file = dir.with_extension("wsb");
    std::fs::write(&file, wsb)?;
    std::process::Command::new("cmd").args(["/c", "start", ""]).arg(&file).spawn()?;
    note("Opening in Windows Sandbox (no network, read-only)");
    Ok(())
}

// --- Encryption (age format: open with any age tool) -----------------------------------------

fn encrypt(src: &Path, password: &str) -> Result<PathBuf> {
    // the file can be attacked offline at leisure, so short passwords are refused
    if password.chars().count() < 8 {
        bail!("password is too short");
    }
    let out = PathBuf::from(format!("{}.age", out_path(src, None, &ext(src)).display()));
    crate::naming::claim(&out);
    let out = if out.exists() { crate::naming::unique(src.parent().unwrap(), &name(src), "age") } else { out };
    let enc = age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(password.to_owned()));
    let mut w = enc.wrap_output(std::fs::File::create(&out)?)?;
    std::io::copy(&mut std::fs::File::open(src)?, &mut w)?;
    w.finish()?;
    Ok(out)
}

fn decrypt(src: &Path, password: &str) -> Result<PathBuf> {
    let dec = age::Decryptor::new(std::fs::File::open(src)?).context("not an encrypted (.age) file")?;
    let id = age::scrypt::Identity::new(age::secrecy::SecretString::from(password.to_owned()));
    let mut r = dec.decrypt(std::iter::once(&id as &dyn age::Identity)).map_err(|_| anyhow::anyhow!("wrong password"))?;
    // "report.pdf.age" → "report.pdf" (or "report (2).pdf")
    let inner = Path::new(&stem(src)).to_path_buf();
    let out = crate::naming::unique(src.parent().context("no folder")?, &stem(&inner), &ext(&inner));
    std::io::copy(&mut r, &mut std::fs::File::create(&out)?)?;
    Ok(out)
}

// --- Secure delete ------------------------------------------------------------------------

/// Overwrites the file with random bytes, renames it, then deletes it. On SSDs this is best effort.
fn shred(p: &Path) -> Result<()> {
    use std::io::{Seek, Write};
    // a link would make us overwrite some other file: the link target, or every hard-linked copy
    let meta = std::fs::symlink_metadata(p)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        bail!("{} is a link or folder, not a plain file", name(p));
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true);
    // open the link itself, never what it points to, even if the file was swapped for a link
    // after the check above; then check again on the open handle
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let mut f = opts.open(p)?;
    let meta = f.metadata()?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        bail!("{} is a link or folder, not a plain file", name(p));
    }
    let len = meta.len();
    if links(&f) > 1 {
        bail!("{} has other hard links; shredding it would destroy them too", name(p));
    }
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15 ^ len ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos() as u64;
    let mut buf = vec![0u8; 1 << 20];
    f.seek(std::io::SeekFrom::Start(0))?;
    let mut left = len;
    while left > 0 {
        for b in buf.iter_mut() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *b = x as u8;
        }
        let n = left.min(buf.len() as u64) as usize;
        f.write_all(&buf[..n])?;
        left -= n as u64;
    }
    f.sync_all()?;
    drop(f);
    let tmp = p.with_file_name(format!("{:016x}.tmp", x));
    std::fs::rename(p, &tmp)?;
    std::fs::remove_file(tmp)?;
    Ok(())
}

#[cfg(windows)]
fn links(f: &std::fs::File) -> u32 {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(f.as_raw_handle() as _, &mut info) } == 0 {
        return u32::MAX;
    }
    info.nNumberOfLinks
}

#[cfg(not(windows))]
fn links(f: &std::fs::File) -> u32 {
    use std::os::unix::fs::MetadataExt;
    f.metadata().map(|m| m.nlink() as u32).unwrap_or(u32::MAX)
}

// --- Who is using the file (Restart Manager) ----------------------------------------------

#[cfg(windows)]
fn locks(p: &Path) -> Result<String> {
    use windows_sys::Win32::System::RestartManager::*;
    let mut session = 0u32;
    let mut key = [0u16; (CCH_RM_SESSION_KEY + 1) as usize];
    if unsafe { RmStartSession(&mut session, 0, key.as_mut_ptr()) } != 0 {
        bail!("couldn't ask Windows");
    }
    let wide = p.as_os_str().encode_wide_z();
    let files = [wide.as_ptr()];
    let mut names = Vec::new();
    unsafe {
        if RmRegisterResources(session, 1, files.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null()) == 0 {
            let (mut needed, mut count, mut reasons) = (0u32, 0u32, 0u32);
            let mut info: Vec<RM_PROCESS_INFO> = Vec::new();
            loop {
                let r = RmGetList(session, &mut needed, &mut count, info.as_mut_ptr(), &mut reasons);
                if r == 234 {
                    // ERROR_MORE_DATA
                    info = vec![std::mem::zeroed(); needed as usize];
                    count = needed;
                    continue;
                }
                break;
            }
            for i in info.iter().take(count as usize) {
                let n = String::from_utf16_lossy(&i.strAppName).trim_end_matches('\0').to_string();
                names.push(format!("{n} (PID {})", i.Process.dwProcessId));
            }
        }
        RmEndSession(session);
    }
    Ok(if names.is_empty() { format!("{}: no program is using it", name(p)) } else { format!("{} is used by: {}", name(p), names.join(", ")) })
}

#[cfg(not(windows))]
fn locks(_: &Path) -> Result<String> {
    bail!("only available on Windows")
}

// --- VirusTotal: only the SHA-256 leaves this computer ------------------------------------

fn virustotal(src: &Path) -> Result<()> {
    let key = crate::settings::current().vt_key;
    let hash = super::files::sha256(src)?;
    let url = format!("https://www.virustotal.com/gui/file/{hash}");
    if key.trim().is_empty() {
        open_url(&url)?;
        note("Opened the VirusTotal report for this hash");
        return Ok(());
    }
    let out = curl_secret(
        super::tools::quiet(&curl()).args(["-s", "--max-time", "20"]).arg(format!("https://www.virustotal.com/api/v3/files/{hash}")),
        &format!("header = \"x-apikey: {}\"", key.trim()),
    )?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    if v["error"]["code"] == "NotFoundError" {
        note(&format!("{}: VirusTotal has never seen this file", name(src)));
        return Ok(());
    }
    let s = &v["data"]["attributes"]["last_analysis_stats"];
    if s.is_null() {
        bail!("VirusTotal didn't answer (check the API key)");
    }
    let bad = s["malicious"].as_u64().unwrap_or(0);
    let sus = s["suspicious"].as_u64().unwrap_or(0);
    let total = ["malicious", "suspicious", "undetected", "harmless"].iter().map(|k| s[k].as_u64().unwrap_or(0)).sum::<u64>();
    note(&format!("{}: {bad}/{total} engines flag it{}", name(src), if sus > 0 { format!(", {sus} suspicious") } else { String::new() }));
    if bad + sus > 0 {
        let _ = open_url(&url);
    }
    Ok(())
}

/// Runs curl with secrets (API keys, passwords) passed as a config on stdin, so they never
/// appear on the command line where other programs can list them.
pub fn curl_secret(cmd: &mut std::process::Command, config: &str) -> Result<std::process::Output> {
    use std::io::Write;
    use std::process::Stdio;
    // each line is `name = "value"`; a pasted key with a quote, backslash or line break could
    // otherwise add its own curl options (e.g. write a file somewhere)
    if config.lines().any(|l| l.matches('"').count() != 2 || l.contains('\\')) || config.contains('\r') {
        bail!("the saved key contains characters a key never has; paste it again in Settings");
    }
    let mut child = cmd.arg("-K").arg("-").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    child.stdin.take().context("no stdin")?.write_all(config.as_bytes())?;
    Ok(child.wait_with_output()?)
}

pub fn curl() -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    Path::new(&root).join("System32").join("curl.exe")
}

pub fn open_url(url: &str) -> Result<()> {
    #[cfg(windows)]
    std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn()?;
    Ok(())
}

// --- Analysis tools (capa, FLOSS, UPX) downloaded on request ---------------------------------

/// tool id, GitHub repo, asset name must contain, exe name
pub const TOOLS: &[(&str, &str, &str, &str)] = &[
    // FFmpeg is not shipped inside the app; this downloads the official Windows build on request
    ("ffmpeg", "BtbN/FFmpeg-Builds", "win64-gpl.zip", "ffmpeg.exe"),
    ("capa", "mandiant/capa", "windows", "capa.exe"),
    ("floss", "mandiant/flare-floss", "windows", "floss.exe"),
    ("upx", "upx/upx", "win64", "upx.exe"),
    ("qpdf", "qpdf/qpdf", "msvc64", "qpdf.exe"),
    ("yara", "VirusTotal/yara-x", "windows", "yr.exe"),
    ("whisper", "ggml-org/whisper.cpp", "whisper-bin-x64", "whisper-cli.exe"),
    (
        "upscale",
        "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-windows.zip",
        "",
        "realesrgan-ncnn-vulkan.exe",
    ),
    ("ai", "https://github.com/microsoft/onnxruntime/releases/download/v1.22.0/onnxruntime-win-x64-1.22.0.zip", "", "onnxruntime.dll"),
];

/// Files some tools need besides the program: (url, file name, smallest plausible size).
fn tool_extras(id: &str) -> &'static [(&'static str, &'static str, u64)] {
    match id {
        "whisper" => &[(WHISPER_MODEL, "ggml-base.bin", 10_000_000)],
        "ai" => &[
            ("https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx", "isnet-general-use.onnx", 10_000_000),
            ("https://github.com/onnx/models/raw/4c46cd00fbdb7cd30b6c1c17ab54f2e1f4f7b177/validated/vision/body_analysis/ultraface/models/version-RFB-320.onnx", "version-RFB-320.onnx", 500_000),
        ],
        _ => &[],
    }
}

/// A file that came with a tool (model, library), once installed.
pub fn tool_file(id: &str, name: &str) -> Option<PathBuf> {
    Some(tools_dir().join(id).join(name)).filter(|p| p.is_file())
}

/// whisper.cpp model: multilingual "base" (≈150 MB) handles Vietnamese and English well enough.
const WHISPER_MODEL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base.bin";

/// SHA-256 of the files from pinned downloads (anything else is refused and deleted).
/// Tools that follow GitHub's latest release are checked against the digest GitHub publishes.
const PINNED: &[(&str, &str, &str)] = &[
    ("ai", "onnxruntime.dll", "579b636403983254346a5c1d80bd28f1519cd1e284cd204f8d4ff41f8d711559"),
    ("ai", "isnet-general-use.onnx", "60920e99c45464f2ba57bee2ad08c919a52bbf852739e96947fbb4358c0d964a"),
    ("ai", "version-RFB-320.onnx", "34cd7e60aeff28744c657de7a3dc64e872d506741de66987f3426f2b79f88017"),
    ("upscale", "realesrgan-ncnn-vulkan.exe", "07e49f7cbb4ede01ae4dd4c399d3a7e5846e3d2085c3128eff881e55cb7b1a0c"),
    ("upscale", "realesrgan-x4plus.bin", "713ee713b0353afaa27976f0563a64a5043bd70b9bd8936c2e26e25ebcdbcddf"),
    ("upscale", "realesrgan-x4plus.param", "35330ececcea33b6c397a72548e788d5d53becee4734c50b7fada36e89f10a86"),
    ("whisper", "ggml-base.bin", "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe"),
];

pub fn whisper_model() -> Option<PathBuf> {
    tool_file("whisper", "ggml-base.bin")
}

pub fn tools_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("ShiftDrop").join("tools")
}

/// Path of an installed tool: our tools folder first, then PATH.
pub fn tool_path(id: &str) -> Option<PathBuf> {
    let exe = TOOLS.iter().find(|t| t.0 == id)?.3;
    let dir = tools_dir().join(id);
    if let Some(p) = find_file(&dir, exe, 3) {
        return Some(p);
    }
    // a DLL found on PATH (Windows ships an older onnxruntime.dll) would be loaded into the app itself
    if id == "ai" {
        return None;
    }
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(exe)).find(|p| p.is_file()))
}

fn find_file(dir: &Path, file: &str, depth: u32) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() && depth > 0 {
            if let Some(f) = find_file(&p, file, depth - 1) {
                return Some(f);
            }
        } else if e.file_name().to_string_lossy().eq_ignore_ascii_case(file) {
            return Some(p);
        }
    }
    None
}

/// Downloads the latest Windows release of a tool from GitHub into the tools folder.
pub fn install_tool(id: &str) -> Result<PathBuf> {
    // a second click while the first download runs would delete its files
    static BUSY: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    {
        let mut busy = BUSY.lock().unwrap_or_else(|e| e.into_inner());
        if busy.iter().any(|b| b == id) {
            bail!("already downloading");
        }
        busy.push(id.to_string());
    }
    let r = install_tool_once(id);
    BUSY.lock().unwrap_or_else(|e| e.into_inner()).retain(|b| b != id);
    r
}

fn install_tool_once(id: &str) -> Result<PathBuf> {
    let (_, source, want, _) = *TOOLS.iter().find(|t| t.0 == id).context("unknown tool")?;
    let mut digest = None;
    let url = if source.starts_with("https://") {
        source.to_string()
    } else {
        let api = super::tools::quiet(&curl())
            .args(["-sL", "--max-time", "30", "-H", "User-Agent: ShiftDrop"])
            .arg(format!("https://api.github.com/repos/{source}/releases/latest"))
            .output()?;
        let v: serde_json::Value = serde_json::from_slice(&api.stdout).context("GitHub didn't answer")?;
        let asset = v["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["name"].as_str().is_some_and(|n| n.to_lowercase().contains(want) && n.ends_with(".zip"))))
            .context("no Windows download found")?;
        digest = Some(
            asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).context("GitHub didn't publish a checksum for this download, so it wasn't installed")?.to_lowercase(),
        );
        asset["browser_download_url"].as_str().context("no download link")?.to_string()
    };
    // build the new copy next to the old one; a failed or tampered download leaves the working one alone
    let dir = tools_dir().join(id);
    let new = tools_dir().join(format!("{id}.new"));
    let _ = std::fs::remove_dir_all(&new);
    std::fs::create_dir_all(&new)?;
    let r = (|| -> Result<()> {
        let zip = new.join("download.zip");
        download(&url, &zip, 1000)?;
        if let Some(want) = &digest {
            if &super::files::sha256(&zip)? != want {
                bail!("the download didn't match its published checksum, so it wasn't installed");
            }
        }
        super::files::untar(&zip, &new)?;
        let _ = std::fs::remove_file(&zip);
        for (url, name, min) in tool_extras(id) {
            download(url, &new.join(name), *min)?;
        }
        for (_, name, sum) in PINNED.iter().filter(|p| p.0 == id) {
            let f = find_file(&new, name, 4).with_context(|| format!("the download is missing {name}"))?;
            if super::files::sha256(&f)? != *sum {
                bail!("{name} didn't match its known checksum, so it wasn't installed");
            }
        }
        Ok(())
    })();
    if let Err(e) = r {
        let _ = std::fs::remove_dir_all(&new);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&new, &dir).context("the old version is still in use; close what is using it and try again")?;
    tool_path(id).context("the download didn't contain the program")
}

fn download(url: &str, to: &Path, min_size: u64) -> Result<()> {
    let ok = super::tools::quiet(&curl())
        .args(["-sfL", "--proto", "=https", "--proto-redir", "=https", "--retry", "2", "--max-time", "1800", "-o"])
        .arg(to)
        .arg(url)
        .status()?
        .success();
    if !ok || std::fs::metadata(to).map(|m| m.len()).unwrap_or(0) < min_size {
        let _ = std::fs::remove_file(to);
        bail!("download failed: {}", url.rsplit('/').next().unwrap_or(url));
    }
    Ok(())
}

fn run_tool(src: &Path, tool: &str) -> Result<Vec<PathBuf>> {
    let exe = tool_path(tool).with_context(|| format!("{tool} isn't installed yet: Settings › Analysis tools"))?;
    let mut cmd = super::tools::quiet(&exe);
    match tool {
        "upx" => {
            let out = out_path(src, Some("unpacked"), &ext(src));
            let o = super::tools::output_within(cmd.arg("-d").arg("-o").arg(&out).arg(src), std::time::Duration::from_secs(600))?;
            if !o.status.success() {
                bail!("{}", String::from_utf8_lossy(&o.stdout).lines().find(|l| l.contains("Exception") || l.contains("error")).unwrap_or("not packed with UPX"));
            }
            Ok(vec![out])
        }
        "yara" => {
            let rules = crate::settings::current().yara_rules;
            if rules.trim().is_empty() || !Path::new(rules.trim()).exists() {
                bail!("choose a YARA rules folder in Settings › Add-ons");
            }
            let o = super::tools::output_within(cmd.args(["scan", "--recursive"]).arg(rules.trim()).arg(src), std::time::Duration::from_secs(600))?;
            let text = String::from_utf8_lossy(&o.stdout).into_owned();
            let hits = text.lines().filter(|l| !l.trim().is_empty()).count();
            if !o.status.success() && hits == 0 {
                bail!("{}", String::from_utf8_lossy(&o.stderr).lines().find(|l| !l.trim().is_empty()).unwrap_or("yara failed"));
            }
            note(&format!("{}: {hits} YARA rule{} matched", name(src), if hits == 1 { "" } else { "s" }));
            if hits == 0 {
                return Ok(vec![]);
            }
            let out = out_path(src, Some("yara"), "txt");
            std::fs::write(&out, text)?;
            Ok(vec![out])
        }
        "capa" | "floss" => {
            let o = super::tools::output_within(cmd.arg(src), std::time::Duration::from_secs(1800))?;
            let text = String::from_utf8_lossy(&o.stdout);
            if text.trim().is_empty() {
                bail!("{}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("no output"));
            }
            let out = out_path(src, Some(tool), "txt");
            std::fs::write(&out, text.as_bytes())?;
            Ok(vec![out])
        }
        _ => bail!("unknown tool"),
    }
}

#[derive(serde::Serialize)]
pub struct ToolInfo {
    id: &'static str,
    installed: bool,
}

#[tauri::command]
pub async fn tools_status() -> Vec<ToolInfo> {
    TOOLS.iter().map(|t| ToolInfo { id: t.0, installed: tool_path(t.0).is_some() && tool_extras(t.0).iter().all(|(_, n, _)| tool_file(t.0, n).is_some()) }).collect()
}

#[tauri::command]
pub async fn tools_install(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || install_tool(&id).map(|_| ()).map_err(|e| e.to_string())).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn sandbox_ok() -> bool {
    sandbox_available()
}
