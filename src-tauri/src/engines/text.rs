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
        let units: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
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
    format!("<!doctype html><html><head><meta charset=\"utf-8\">{}<title>{}</title><style>{STYLE}</style></head><body>{body}</body></html>", lockdown(base), escape(title))
}

/// The policy every printed page gets, plus the folder relative pictures are read from.
fn lockdown(base: &str) -> String {
    format!("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data: file:; style-src 'unsafe-inline' file:; font-src data: file:\"><base href=\"{}\">", escape(base))
}

/// A stranger's HTML file gets the same lockdown as Markdown, and loses its `<meta http-equiv>`
/// tags: a refresh to `file:///…` would otherwise print some other local file into the PDF.
/// A `Content-Type` one (the page's own character set) stays when the bytes are kept as they were.
pub(crate) fn lock_html(html: &str, base: &str) -> String {
    lock_html_with(html, base, false)
}

fn lock_html_with(html: &str, base: &str, keep_charset: bool) -> String {
    let lower = html.to_ascii_lowercase();
    let b = lower.as_bytes();
    let mut out = String::with_capacity(html.len() + 300);
    // after the doctype, so the page isn't switched to quirks mode
    let start = {
        let t = lower.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
        if t.starts_with("<!doctype") { lower.len() - t.len() + t.find('>').map_or(0, |i| i + 1) } else { 0 }
    };
    out.push_str(&html[..start]);
    out.push_str(&lockdown(base));
    let mut i = start;
    while let Some(off) = lower[i..].find("<meta") {
        let at = i + off;
        // the tag ends at the first `>` outside a quoted attribute value
        let mut j = at + 5;
        while j < b.len() && b[j] != b'>' {
            if b[j] == b'=' {
                j += 1;
                while j < b.len() && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < b.len() && (b[j] == b'"' || b[j] == b'\'') {
                    let q = b[j];
                    j += 1;
                    while j < b.len() && b[j] != q {
                        j += 1;
                    }
                    j += 1;
                }
                continue;
            }
            j += 1;
        }
        let end = (j + 1).min(b.len());
        out.push_str(&html[i..at]);
        let tag = &lower[at..end];
        if !tag.contains("http-equiv") || (keep_charset && tag.contains("content-type")) {
            out.push_str(&html[at..end]);
        }
        i = end;
    }
    out.push_str(&html[i..]);
    out
}

/// Markdown can carry raw HTML; it is shown as text instead, so it can't load or redirect anything.
pub(crate) fn markdown(text: &str) -> String {
    use pulldown_cmark::Event;
    let mut html = String::new();
    let events = pulldown_cmark::Parser::new_ext(text, pulldown_cmark::Options::all()).map(|e| match e {
        Event::Html(t) | Event::InlineHtml(t) => Event::Text(t),
        e => e,
    });
    pulldown_cmark::html::push_html(&mut html, events);
    html
}

pub fn to_html(src: &Path) -> Result<String> {
    let text = read(src)?;
    let body = match ext(src).as_str() {
        "md" | "markdown" => markdown(&text),
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
        .filter_map(std::env::var_os)
        .map(|d| Path::new(&d).join("Microsoft").join("Edge").join("Application").join("msedge.exe"))
        .find(|p| p.is_file())
}

fn to_pdf(src: &Path) -> Result<PathBuf> {
    let edge = edge().context("Microsoft Edge is needed to make PDFs from text")?;
    // HTML files print as they are but locked down (relative images keep working through the
    // base); everything else is wrapped first
    let doc: Vec<u8> = if matches!(ext(src).as_str(), "html" | "htm") {
        let base = src.parent().map(|d| format!("{}/", url(d))).unwrap_or_default();
        let bytes = std::fs::read(src)?;
        if std::str::from_utf8(&bytes).is_ok() || bytes.starts_with(&[0xFF, 0xFE]) {
            // UTF-8 (or UTF-16, which read() turns into UTF-8): say so before anything else
            let locked = lock_html(&read(src)?, &base);
            let at = locked.to_ascii_lowercase().find("<meta http-equiv=\"content-security-policy\"").unwrap_or(0);
            format!("{}<meta charset=\"utf-8\">{}", &locked[..at], &locked[at..]).into_bytes()
        } else {
            // an older character set (Windows-1252, Shift-JIS…): edit around the bytes without
            // decoding them, so the page's own charset still reads them right
            let latin: String = bytes.iter().map(|&b| b as char).collect();
            lock_html_with(&latin, &base, true).chars().map(|c| u8::try_from(c as u32).unwrap_or(b'?')).collect()
        }
    } else {
        to_html(src)?.into_bytes()
    };
    let html = temp_file("html");
    std::fs::write(&html, doc)?;
    let out = out_path(src, None, "pdf");
    // a profile of its own, kept between jobs (they run one at a time), so it starts quickly
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
    let _ = std::fs::remove_file(&html);
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
    let text = read(src)?;
    serde_json::from_str::<serde::de::IgnoredAny>(&text).context("not valid JSON")?;
    let (suffix, text) = (if pretty { "Formatted" } else { "Minified" }, reformat_json(&text, pretty));
    let out = out_path(src, Some(suffix), "json");
    std::fs::write(&out, text)?;
    Ok(out)
}

/// Re-spaces valid JSON without parsing it into values, so every number, escape and key order
/// stays exactly as written (parsing would round numbers longer than 15–17 digits).
fn reformat_json(text: &str, pretty: bool) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 4);
    let mut depth = 0usize;
    let (mut in_str, mut escaped) = (false, false);
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let newline = |out: &mut String, depth: usize| {
        out.push('\n');
        out.push_str(&"  ".repeat(depth));
    };
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push(c);
            }
            c if c.is_whitespace() => {}
            '{' | '[' => {
                out.push(c);
                // keep {} and [] on one line
                while chars.peek().is_some_and(|n| n.is_whitespace()) {
                    chars.next();
                }
                if let Some(close) = chars.next_if(|n| matches!(n, '}' | ']')) {
                    out.push(close);
                } else {
                    depth += 1;
                    if pretty {
                        newline(&mut out, depth);
                    }
                }
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                if pretty {
                    newline(&mut out, depth);
                }
                out.push(c);
            }
            ',' => {
                out.push(c);
                if pretty {
                    newline(&mut out, depth);
                }
            }
            ':' => out.push_str(if pretty { ": " } else { ":" }),
            c => out.push(c),
        }
    }
    if pretty {
        out.push('\n');
    }
    out
}

/// Numbers that a 64-bit value can't hold exactly (long IDs, very precise decimals) become text
/// with all their digits before parsing, instead of being silently rounded. Returns how many.
fn keep_long_numbers(text: &str) -> (String, usize) {
    let (mut out, mut count) = (String::with_capacity(text.len()), 0);
    let (mut in_str, mut escaped) = (false, false);
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if in_str {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_str = false,
                _ => {}
            }
            out.push(c);
            continue;
        }
        if c == '"' {
            in_str = true;
            out.push(c);
            continue;
        }
        if c == '-' || c.is_ascii_digit() {
            let mut end = i + 1;
            while let Some((j, _)) = chars.next_if(|&(_, n)| n.is_ascii_digit() || matches!(n, '.' | 'e' | 'E' | '+' | '-')) {
                end = j + 1;
            }
            let num = &text[i..end];
            let exact = if num.contains(['.', 'e', 'E']) {
                // a decimal survives when the shortest form of the parsed double has the same digits
                let digits = |s: &str| s.split(['e', 'E']).next().unwrap_or(s).chars().filter(char::is_ascii_digit).collect::<String>().trim_start_matches('0').trim_end_matches('0').to_string();
                num.parse::<f64>().is_ok_and(|f| f.is_finite() && digits(&format!("{:e}", f.abs())) == digits(num))
            } else {
                num.parse::<i64>().is_ok() || num.parse::<u64>().is_ok()
            };
            if exact {
                out.push_str(num);
            } else {
                count += 1;
                out.push('"');
                out.push_str(num);
                out.push('"');
            }
            continue;
        }
        out.push(c);
    }
    (out, count)
}

fn parse_json(text: &str) -> Result<serde_json::Value> {
    let (text, long) = keep_long_numbers(text);
    let v = serde_json::from_str(&text).context("not valid JSON")?;
    if long > 0 {
        super::note(&format!("{long} long number{} kept as text so no digits were lost", if long == 1 { " was" } else { "s were" }));
    }
    Ok(v)
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
    let v = parse_json(&read(src)?)?;
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
        "json" => parse_json(&text)?,
        "yaml" | "yml" => serde_yaml::from_str(&text).context("not valid YAML")?,
        "toml" => toml_to_json(toml::from_str::<toml::Value>(&text).context("not valid TOML")?),
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

/// TOML dates and times become plain text (serde would write them as a private wrapper object).
pub(crate) fn toml_to_json(v: toml::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::from(i),
        toml::Value::Float(f) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => Value::Object(t.into_iter().map(|(k, v)| (k, toml_to_json(v))).collect()),
    }
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
pub(crate) fn xml_to_value(text: &str) -> Result<serde_json::Value> {
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
            Event::Start(e) => {
                // each level is a step of recursion later on (writing JSON, dropping it)
                if stack.len() > 100 {
                    bail!("the XML is nested too deeply (over 100 levels)");
                }
                stack.push((String::from_utf8_lossy(e.name().as_ref()).into_owned(), attrs(&e)))
            }
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
            Event::Eof if stack.len() > 1 => bail!("the XML file is cut off (a tag is never closed)"),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(Value::Object(stack.pop().map(|(_, m)| m).unwrap_or_default()))
}
