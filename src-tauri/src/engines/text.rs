//! Text, code and data files: to PDF / HTML (PDF is printed by headless Edge, so any script
//! renders), JSON formatting and CSV <-> JSON.

use super::{Action, Progress};
use crate::naming::{ext, out_path, stem, temp_file};
use anyhow::{bail, Context, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn run(src: &Path, action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    progress(0.1);
    let out = match action {
        Action::Convert { to } if to == "pdf" => to_pdf(src)?,
        Action::Convert { to } if to == "docx" => return super::office::run(src, action, progress),
        Action::Convert { to } if to == "html" => {
            let out = out_path(src, None, "html");
            std::fs::write(&out, to_html(src)?)?;
            out
        }
        Action::Convert { to } if to == "json" && matches!(ext(src).as_str(), "csv" | "tsv") => csv_to_json(src)?,
        Action::Convert { to } if to == "csv" => json_to_csv(src)?,
        Action::Convert { to } if matches!(to.as_str(), "json" | "yaml" | "toml") => data_to(src, to)?,
        Action::FormatJson { pretty } => format_json(src, *pretty)?,
        _ => bail!("not available for text files"),
    };
    Ok(vec![out])
}

fn read(src: &Path) -> Result<String> {
    let bytes = std::fs::read(src)?;
    // UTF-16 files (common for Windows logs) start with a byte-order mark
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    let s = String::from_utf8_lossy(&bytes).into_owned();
    Ok(s.strip_prefix('\u{feff}').map(str::to_owned).unwrap_or(s))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

const STYLE: &str = "body{font:14px/1.55 'Segoe UI',system-ui,sans-serif;color:#111;max-width:860px;margin:32px auto;padding:0 24px}\
pre{font:12px/1.5 Consolas,'Cascadia Mono',monospace;white-space:pre-wrap;word-break:break-word}\
code{font-family:Consolas,monospace;background:#f2f2f2;padding:1px 4px;border-radius:3px}\
pre code{background:none;padding:0}table{border-collapse:collapse;font-size:12px}\
td,th{border:1px solid #ccc;padding:4px 8px;text-align:left}th{background:#f2f2f2}\
img{max-width:100%}blockquote{margin-left:0;padding-left:16px;border-left:3px solid #ccc;color:#555}";

/// Text and Markdown from anywhere get no scripts, frames or embedded documents: a downloaded
/// .md with `<iframe src="file:///…">` would otherwise copy a local file into the PDF.
/// Pictures are allowed (relative ones resolve next to the source through `base`).
fn page(title: &str, body: &str, base: &str) -> String {
    format!("<!doctype html><html><head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data: file:; style-src 'unsafe-inline'; font-src data:\"><base href=\"{}\"><title>{}</title><style>{STYLE}</style></head><body>{body}</body></html>", escape(base), escape(title))
}

pub fn to_html(src: &Path) -> Result<String> {
    let text = read(src)?;
    let body = match ext(src).as_str() {
        "md" | "markdown" => {
            let mut html = String::new();
            let opts = pulldown_cmark::Options::all();
            pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new_ext(&text, opts));
            html
        }
        "html" | "htm" => return Ok(text),
        "csv" | "tsv" => {
            let rows = parse_csv(&text, if ext(src) == "tsv" { '\t' } else { ',' });
            let mut t = String::from("<table>");
            for (i, row) in rows.iter().enumerate() {
                let tag = if i == 0 { "th" } else { "td" };
                t += "<tr>";
                for cell in row {
                    t += &format!("<{tag}>{}</{tag}>", escape(cell));
                }
                t += "</tr>";
            }
            t + "</table>"
        }
        _ => format!("<pre>{}</pre>", escape(&text)),
    };
    let base = src.parent().map(|d| format!("{}/", url(d))).unwrap_or_default();
    Ok(page(&stem(src), &body, &base))
}

fn edge() -> Option<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"]
        .iter()
        .filter_map(|v| std::env::var_os(v))
        .map(|d| Path::new(&d).join("Microsoft").join("Edge").join("Application").join("msedge.exe"))
        .find(|p| p.is_file())
}

fn to_pdf(src: &Path) -> Result<PathBuf> {
    let edge = edge().context("Microsoft Edge is needed to make PDFs from text")?;
    // HTML files print as they are (relative images keep working); everything else is wrapped first
    let html = if matches!(ext(src).as_str(), "html" | "htm") {
        src.to_path_buf()
    } else {
        let tmp = temp_file("html");
        std::fs::write(&tmp, to_html(src)?)?;
        tmp
    };
    let out = out_path(src, None, "pdf");
    let profile = std::env::temp_dir().join("shiftdrop").join("edge");
    let mut cmd = Command::new(edge);
    // the page may be a stranger's HTML: with no network at all (no DNS, every request sent to
    // a dead proxy) printing it can't tell anyone the file was opened or leak what is in it
    cmd.arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--host-resolver-rules=MAP * ~NOTFOUND")
        .arg("--proxy-server=127.0.0.1:9")
        .arg("--proxy-bypass-list=<-loopback>")
        .arg("--disable-extensions")
        .arg("--no-pdf-header-footer")
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--print-to-pdf={}", out.display()))
        .arg(url(&html));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(super::tools::CREATE_NO_WINDOW);
    }
    let r = super::tools::output_within(&mut cmd, std::time::Duration::from_secs(120));
    if html != src {
        let _ = std::fs::remove_file(&html);
    }
    let status = r.context("Edge couldn't print the PDF")?.status;
    if !status.success() || !out.is_file() {
        bail!("Edge couldn't print the PDF");
    }
    Ok(out)
}

fn url(p: &Path) -> String {
    let s = p.display().to_string().replace('\\', "/");
    let enc: String = s
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("file:///{enc}")
}

fn format_json(src: &Path, pretty: bool) -> Result<PathBuf> {
    let v: serde_json::Value = serde_json::from_str(&read(src)?).context("not valid JSON")?;
    let (suffix, text) = if pretty { ("Formatted", serde_json::to_string_pretty(&v)?) } else { ("Minified", serde_json::to_string(&v)?) };
    let out = out_path(src, Some(suffix), "json");
    std::fs::write(&out, text)?;
    Ok(out)
}

/// RFC 4180-style parsing: quoted fields may hold separators, quotes ("") and newlines.
fn parse_csv(text: &str, sep: char) -> Vec<Vec<String>> {
    let (mut rows, mut row, mut field) = (Vec::new(), Vec::new(), String::new());
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => row.push(std::mem::take(&mut field)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            c => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn csv_to_json(src: &Path) -> Result<PathBuf> {
    let rows = parse_csv(&read(src)?, if ext(src) == "tsv" { '\t' } else { ',' });
    let Some((head, body)) = rows.split_first() else { bail!("the file is empty") };
    let items: Vec<serde_json::Value> = body
        .iter()
        .map(|r| {
            let obj = head.iter().enumerate().map(|(i, k)| (k.clone(), serde_json::Value::String(r.get(i).cloned().unwrap_or_default())));
            serde_json::Value::Object(obj.collect())
        })
        .collect();
    let out = out_path(src, None, "json");
    std::fs::write(&out, serde_json::to_string_pretty(&items)?)?;
    Ok(out)
}

fn json_to_csv(src: &Path) -> Result<PathBuf> {
    let v: serde_json::Value = serde_json::from_str(&read(src)?).context("not valid JSON")?;
    let rows = v.as_array().context("CSV needs a JSON list of objects")?;
    let mut keys: Vec<String> = Vec::new();
    for r in rows {
        for k in r.as_object().context("CSV needs a JSON list of objects")?.keys() {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
    }
    let cell = |v: Option<&serde_json::Value>| {
        let s = match v {
            None | Some(serde_json::Value::Null) => String::new(),
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        };
        if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
    };
    let mut csv = keys.iter().map(|k| cell(Some(&serde_json::Value::String(k.clone())))).collect::<Vec<_>>().join(",") + "\r\n";
    for r in rows {
        csv += &(keys.iter().map(|k| cell(r.get(k))).collect::<Vec<_>>().join(",") + "\r\n");
    }
    let out = out_path(src, None, "csv");
    std::fs::write(&out, csv)?;
    Ok(out)
}

/// Structured data (JSON, YAML, TOML, XML) read into one common shape.
fn load_data(src: &Path) -> Result<serde_json::Value> {
    let text = read(src)?;
    Ok(match ext(src).as_str() {
        "json" => serde_json::from_str(&text).context("not valid JSON")?,
        "yaml" | "yml" => serde_yaml::from_str(&text).context("not valid YAML")?,
        "toml" => serde_json::to_value(toml::from_str::<toml::Value>(&text).context("not valid TOML")?)?,
        "xml" => xml_to_value(&text)?,
        e => bail!("can't read .{e} as data"),
    })
}

fn data_to(src: &Path, to: &str) -> Result<PathBuf> {
    let v = load_data(src)?;
    let text = match to {
        "json" => serde_json::to_string_pretty(&v)?,
        "yaml" => serde_yaml::to_string(&v)?,
        "toml" => {
            // TOML documents must be tables
            let v = if v.is_object() { v } else { serde_json::json!({ "items": v }) };
            toml::to_string_pretty(&strip_nulls(v))?
        }
        _ => bail!("unknown format"),
    };
    let out = out_path(src, None, if to == "yaml" { "yaml" } else { to });
    std::fs::write(&out, text)?;
    Ok(out)
}

/// TOML has no null.
fn strip_nulls(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Object(m) => Value::Object(m.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k, strip_nulls(v))).collect()),
        Value::Array(a) => Value::Array(a.into_iter().filter(|v| !v.is_null()).map(strip_nulls).collect()),
        v => v,
    }
}

/// XML → JSON the common way: attributes as "@name", text as "#text", repeated children as arrays.
fn xml_to_value(text: &str) -> Result<serde_json::Value> {
    use quick_xml::events::Event;
    use serde_json::{Map, Value};
    fn push(m: &mut Map<String, Value>, k: String, v: Value) {
        match m.get_mut(&k) {
            Some(Value::Array(a)) => a.push(v),
            Some(old) => *old = Value::Array(vec![old.take(), v]),
            None => {
                m.insert(k, v);
            }
        }
    }
    fn finish(m: Map<String, Value>) -> Value {
        // <a>text</a> → "text"
        if m.len() == 1 {
            if let Some(Value::String(t)) = m.get("#text") {
                return Value::String(t.clone());
            }
        }
        if m.is_empty() { Value::Null } else { Value::Object(m) }
    }
    let mut r = quick_xml::Reader::from_str(text);
    r.config_mut().trim_text(true);
    let mut stack: Vec<(String, Map<String, Value>)> = vec![("".into(), Map::new())];
    let attrs = |e: &quick_xml::events::BytesStart| {
        let mut m = Map::new();
        for a in e.attributes().flatten() {
            let k = format!("@{}", String::from_utf8_lossy(a.key.as_ref()));
            m.insert(k, Value::String(a.unescape_value().map(|v| v.into_owned()).unwrap_or_default()));
        }
        m
    };
    loop {
        match r.read_event().context("not valid XML")? {
            Event::Start(e) => stack.push((String::from_utf8_lossy(e.name().as_ref()).into_owned(), attrs(&e))),
            Event::Empty(e) => {
                let k = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let v = finish(attrs(&e));
                push(&mut stack.last_mut().unwrap().1, k, v);
            }
            Event::Text(t) => {
                let t = t.unescape().map(|t| t.into_owned()).unwrap_or_default();
                if !t.is_empty() {
                    push(&mut stack.last_mut().unwrap().1, "#text".into(), Value::String(t));
                }
            }
            Event::CData(t) => push(&mut stack.last_mut().unwrap().1, "#text".into(), Value::String(String::from_utf8_lossy(&t).into_owned())),
            Event::End(_) => {
                let (k, m) = stack.pop().context("broken XML")?;
                push(&mut stack.last_mut().context("broken XML")?.1, k, finish(m));
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(Value::Object(stack.pop().map(|(_, m)| m).unwrap_or_default()))
}
