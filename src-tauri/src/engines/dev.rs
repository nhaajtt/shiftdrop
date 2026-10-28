//! Small developer helpers: Base64, path formats and comparing two files.

use super::{note, Action, Progress};
use crate::naming::{out_path, stem, unique};
use anyhow::{bail, Context, Result};
use base64::Engine;
use std::path::{Path, PathBuf};

pub fn handles(a: &Action) -> bool {
    matches!(a, Action::Base64 { .. } | Action::CopyPathAs { .. } | Action::Diff)
}

pub fn run(paths: &[PathBuf], action: &Action, _progress: Progress) -> Result<Vec<PathBuf>> {
    match action {
        Action::Base64 { decode: false } => {
            let p = &paths[0];
            let data = std::fs::read(p)?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            // images become data URLs, ready to paste into HTML or CSS
            let text = match super::security::sniff(p) {
                Some((_, mime)) if mime.starts_with("image/") => format!("data:{mime};base64,{b64}"),
                _ => b64,
            };
            if text.len() <= 8 << 20 {
                crate::clipboard::set(&text)?;
                note("Base64 copied");
                return Ok(vec![]);
            }
            let out = out_path(p, Some("base64"), "txt");
            std::fs::write(&out, text)?;
            Ok(vec![out])
        }
        Action::Base64 { decode: true } => {
            let p = &paths[0];
            let raw = std::fs::read_to_string(p).context("not a text file")?;
            let body = raw.trim().rsplit_once("base64,").map(|(_, b)| b).unwrap_or(raw.trim());
            let clean: String = body.chars().filter(|c| !c.is_whitespace()).collect();
            let data = base64::engine::general_purpose::STANDARD
                .decode(&clean)
                .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(clean.trim_end_matches('=')))
                .context("not valid Base64")?;
            let e = super::security::sniff_bytes(&data).map(|(e, _)| e).unwrap_or_else(|| "bin".into());
            let out = unique(p.parent().context("no folder")?, &format!("{} decoded", stem(p)), &e);
            std::fs::write(&out, data)?;
            Ok(vec![out])
        }
        Action::CopyPathAs { style } => {
            let lines: Vec<String> = paths.iter().map(|p| path_as(p, style)).collect();
            crate::clipboard::set(&lines.join(if style == "quoted" { " " } else { "\r\n" }))?;
            note("Path copied");
            Ok(vec![])
        }
        Action::Diff => diff(paths),
        _ => bail!("not available"),
    }
}

pub fn path_as(p: &Path, style: &str) -> String {
    let s = p.display().to_string();
    match style {
        "quoted" => format!("\"{s}\""),
        // C:\Users\me\a b.txt → /mnt/c/Users/me/a b.txt
        "wsl" => match s.split_once(":\\") {
            Some((drive, rest)) => format!("/mnt/{}/{}", drive.to_lowercase(), rest.replace('\\', "/")),
            None => s.replace('\\', "/"),
        },
        "url" => {
            let enc: String = s
                .replace('\\', "/")
                .bytes()
                .map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
                    _ => format!("%{b:02X}"),
                })
                .collect();
            format!("file:///{enc}")
        }
        "markdown" => {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            format!("[{name}](<{}>)", s.replace('\\', "/"))
        }
        _ => s,
    }
}

/// Two files side by side in VS Code, or a unified .diff file next to the first one.
fn diff(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let [a, b] = paths else { bail!("pick exactly two files to compare") };
    if !cfg!(test) && crate::apps::launch_id("vscode", &format!("--diff \"{}\" \"{}\"", a.display(), b.display())).is_ok() {
        note("Opened the comparison in VS Code");
        return Ok(vec![]);
    }
    let (ta, tb) = (std::fs::read_to_string(a).context("not a text file")?, std::fs::read_to_string(b).context("not a text file")?);
    let text = similar::TextDiff::from_lines(&ta, &tb)
        .unified_diff()
        .context_radius(3)
        .header(&a.display().to_string(), &b.display().to_string())
        .to_string();
    if text.is_empty() {
        note("The two files are identical");
        return Ok(vec![]);
    }
    let out = out_path(a, Some(&format!("vs {}", stem(b))), "diff");
    std::fs::write(&out, text)?;
    Ok(vec![out])
}
