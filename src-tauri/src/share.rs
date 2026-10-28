//! Sending files elsewhere, only when the user asks:
//! - to a phone on the same Wi-Fi (one-time link, LAN address only, expires),
//! - to the user's own Cloudflare R2 bucket (link copied),
//! and translating or summarising text with the user's own Claude API key.

use crate::{engines::security::{curl, curl_secret}, engines::tools::quiet, settings};
use anyhow::{bail, Context, Result};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, UdpSocket},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn token() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("no randomness from the OS");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// This PC's address on the local network (no packet is sent).
fn lan_ip() -> Result<std::net::IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0")?;
    s.connect("192.168.0.1:9")?;
    let ip = s.local_addr()?.ip();
    if ip.is_loopback() || ip.is_unspecified() {
        bail!("not connected to a local network");
    }
    Ok(ip)
}

#[derive(serde::Serialize)]
pub struct LanShare {
    url: String,
    /// QR code as an SVG string
    qr: String,
    minutes: u64,
}

const LAN_MINUTES: u64 = 10;

/// Serves one file (several are zipped first) at a secret link on the LAN for 10 minutes.
/// Only that exact URL answers; there are no uploads and nothing else is served.
#[tauri::command]
pub async fn share_lan(paths: Vec<String>) -> Result<LanShare, String> {
    tauri::async_runtime::spawn_blocking(move || lan(paths).map_err(|e| e.to_string())).await.map_err(|e| e.to_string())?
}

fn lan(paths: Vec<String>) -> Result<LanShare> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).filter(|p| p.is_file()).collect();
    let file = match paths.as_slice() {
        [] => bail!("no files"),
        [one] => one.clone(),
        many => {
            let out = crate::engines::run(many, &crate::engines::Action::Zip, &|_| {})?;
            out.into_iter().next().context("zip failed")?
        }
    };
    let ip = lan_ip()?;
    // bound to the LAN address only, never 0.0.0.0
    let listener = TcpListener::bind((ip, 0))?;
    let port = listener.local_addr()?.port();
    let tok = token();
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or("file".into());
    let enc: String = name.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
    let url = format!("http://{ip}:{port}/{tok}/{enc}");
    let qr = qrcode::QrCode::new(url.as_bytes())?.render::<qrcode::render::svg::Color>().min_dimensions(240, 240).build();
    let prefix = format!("/{tok}/");
    std::thread::spawn(move || serve(listener, file, prefix, Duration::from_secs(LAN_MINUTES * 60)));
    Ok(LanShare { url, qr, minutes: LAN_MINUTES })
}

fn serve(listener: TcpListener, file: PathBuf, prefix: String, life: Duration) {
    let end = Instant::now() + life;
    let _ = listener.set_nonblocking(true);
    while Instant::now() < end {
        let Ok((mut s, _)) = listener.accept() else {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        };
        let _ = s.set_nonblocking(false);
        let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
        // a client that stops reading must not hold the only connection slot forever
        let _ = s.set_write_timeout(Some(Duration::from_secs(60)));
        let mut line = String::new();
        if BufReader::new(&s).read_line(&mut line).is_err() {
            continue;
        }
        let path = line.split_whitespace().nth(1).unwrap_or("");
        if !line.starts_with("GET ") || !path.starts_with(&prefix) {
            let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            continue;
        }
        let Ok(mut f) = std::fs::File::open(&file) else { continue };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        let name = file.file_name().map(|n| n.to_string_lossy().replace('"', "")).unwrap_or_default();
        let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {len}\r\nContent-Disposition: attachment; filename*=UTF-8''{}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", urlenc(&name));
        if s.write_all(head.as_bytes()).is_ok() {
            let mut buf = vec![0u8; 1 << 16];
            while let Ok(n) = f.read(&mut buf) {
                if n == 0 || s.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
        }
    }
}

fn urlenc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Uploads to the user's R2 bucket (S3 API, signed by curl) and copies the public link.
#[tauri::command]
pub async fn share_r2(paths: Vec<String>) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || r2(paths).map_err(|e| e.to_string())).await.map_err(|e| e.to_string())?
}

fn r2(paths: Vec<String>) -> Result<Vec<String>> {
    let s = settings::current();
    if [&s.r2_account, &s.r2_key, &s.r2_secret, &s.r2_bucket].iter().any(|v| v.trim().is_empty()) {
        bail!("set up Cloudflare R2 in Settings first");
    }
    // the account and bucket end up in the upload URL: only the characters R2 allows
    let plain = |v: &str| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !plain(s.r2_account.trim()) || !plain(s.r2_bucket.trim()) {
        bail!("the R2 account ID and bucket name may only use letters, digits and dashes");
    }
    let mut links = Vec::new();
    for p in paths.iter().map(Path::new).filter(|p| p.is_file()) {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        // a random folder keeps links unguessable even in a public bucket
        let key = format!("{}/{}", &token()[..12], urlenc(&name));
        let url = format!("https://{}.r2.cloudflarestorage.com/{}/{key}", s.r2_account.trim(), s.r2_bucket.trim());
        let o = curl_secret(
            quiet(&curl()).args(["-sS", "--fail-with-body", "--max-time", "600", "--aws-sigv4", "aws:amz:auto:s3", "-T"]).arg(p).arg(&url),
            &format!("user = \"{}:{}\"", s.r2_key.trim(), s.r2_secret.trim()),
        )?;
        if !o.status.success() {
            bail!("upload failed: {}", String::from_utf8_lossy(&o.stdout).chars().take(200).collect::<String>());
        }
        let base = s.r2_public.trim().trim_end_matches('/');
        links.push(if base.is_empty() { url } else { format!("{base}/{key}") });
    }
    crate::clipboard::set(&links.join("\r\n"))?;
    Ok(links)
}

// --- Claude: translate / summarise text files with the user's API key -------------------------

const MODEL: &str = "claude-opus-5-5";
/// Larger files are refused rather than silently cut.
const MAX_CHARS: usize = 600_000;

pub fn ask_claude(src: &Path, task: &str, lang: &str) -> Result<PathBuf> {
    let key = settings::current().anthropic_key;
    if key.trim().is_empty() {
        bail!("add your Claude API key in Settings › Online services");
    }
    let text = std::fs::read_to_string(src).context("not a text file")?;
    if text.chars().count() > MAX_CHARS {
        bail!("the file is too long to send in one go ({} characters, limit {MAX_CHARS})", text.chars().count());
    }
    let lang = if lang.trim().is_empty() { "Vietnamese" } else { lang.trim() };
    let instruction = match task {
        "translate" => format!("Translate the document below into {lang}. Keep its formatting (Markdown, line breaks, lists, code) and do not add notes. Reply with the translation only."),
        _ => format!("Summarise the document below in {lang}: a one-sentence overview, then the key points as a short bullet list. Reply with the summary only."),
    };
    let body = serde_json::json!({
        "model": MODEL,
        "max_tokens": 64000,
        "stream": false,
        "fallbacks": "default",
        "messages": [{ "role": "user", "content": format!("{instruction}\n\n<document>\n{text}\n</document>") }],
    });
    let req = crate::naming::temp_file("json");
    std::fs::write(&req, serde_json::to_vec(&body)?)?;
    let o = curl_secret(
        quiet(&curl())
            .args(["-sS", "--max-time", "900", "https://api.anthropic.com/v1/messages", "-H", "content-type: application/json", "-H", "anthropic-version: 2023-06-01"])
            .args(["-H", "anthropic-beta: server-side-fallback-2026-07-01", "--data-binary"])
            .arg(format!("@{}", req.display())),
        &format!("header = \"x-api-key: {}\"", key.trim()),
    );
    let _ = std::fs::remove_file(&req);
    let v: serde_json::Value = serde_json::from_slice(&o?.stdout).context("no answer from the Claude API")?;
    if let Some(e) = v["error"]["message"].as_str() {
        bail!("Claude API: {e}");
    }
    if v["stop_reason"] == "refusal" {
        bail!("Claude declined this document");
    }
    let out_text: String = v["content"].as_array().map(|a| a.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default();
    if out_text.trim().is_empty() {
        bail!("the answer was empty");
    }
    let suffix = if task == "translate" { lang.to_string() } else { "summary".into() };
    let e = crate::naming::ext(src);
    let out = crate::naming::out_path(src, Some(&suffix), if e.is_empty() { "txt" } else { &e });
    std::fs::write(&out, out_text)?;
    Ok(out)
}
