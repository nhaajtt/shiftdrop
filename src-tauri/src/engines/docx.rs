//! Word (.docx) to Markdown without Office: headings, bold/italic, lists, tables and line breaks.

use crate::naming::{out_path, temp_file};
use anyhow::{Context, Result};
use quick_xml::events::Event;
use std::path::{Path, PathBuf};

pub fn to_markdown(src: &Path) -> Result<PathBuf> {
    // a .docx is a zip; Windows' tar reads zips
    let dir = temp_file("docx");
    std::fs::create_dir_all(&dir)?;
    let r = super::files::untar_one(src, &dir, "word/document.xml").and_then(|_| {
        let xml = super::files::read_capped(&dir.join("word").join("document.xml"), 200)?;
        String::from_utf8(xml).ok().context("not a Word document")
    });
    let _ = std::fs::remove_dir_all(&dir);
    let md = convert(&r?)?;
    let out = out_path(src, None, "md");
    std::fs::write(&out, md)?;
    Ok(out)
}

#[derive(Default)]
struct Para {
    style: String,
    list: bool,
    text: String,
}

pub(crate) fn convert(xml: &str) -> Result<String> {
    let mut r = quick_xml::Reader::from_str(xml);
    let mut out = String::new();
    let mut para: Option<Para> = None;
    let (mut bold, mut italic) = (false, false);
    let mut in_text = false;
    // <w:tab/> inside a run is a tab character; inside paragraph properties it is a tab stop
    let mut in_run = false;
    // tables: rows of cells, each cell's paragraphs joined
    let mut table: Vec<Vec<String>> = Vec::new();
    let mut depth = 0usize; // nested table depth
    let mut cell: Option<String> = None;
    loop {
        match r.read_event().context("broken document.xml")? {
            // an empty <w:t/> has no text and no end tag to switch reading off again
            Event::Empty(e) if e.local_name().as_ref() == "t" => {}
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                "tbl" => {
                    depth += 1;
                    if depth == 1 {
                        table.clear();
                    }
                }
                "tr" if depth == 1 => table.push(Vec::new()),
                "tc" if depth == 1 => cell = Some(String::new()),
                "p" => {
                    para = Some(Para::default());
                    in_run = false;
                }
                "pStyle" => {
                    if let (Some(p), Some(v)) = (para.as_mut(), attr(&e, "val")) {
                        p.style = v;
                    }
                }
                "numPr" => {
                    if let Some(p) = para.as_mut() {
                        p.list = true;
                    }
                }
                "r" => {
                    bold = false;
                    italic = false;
                    in_run = true;
                }
                "b" => bold = attr(&e, "val").is_none_or(|v| v != "0" && v != "false"),
                "i" => italic = attr(&e, "val").is_none_or(|v| v != "0" && v != "false"),
                "t" => in_text = true,
                "tab" if in_run => push(&mut para, "\t"),
                // a table row is one line in Markdown, so a break inside a cell stays an HTML break
                "br" => push(&mut para, if cell.is_some() { "<br>" } else { "  \n" }),
                _ => {}
            },
            // text arrives in pieces around &amp; and the like
            ev @ (Event::Text(_) | Event::GeneralRef(_)) if in_text => {
                // the document's own * _ ` \ are escaped, so they never read as formatting (and the
                // clean-up of "****"/"__" below can't eat them)
                let s = match ev {
                    Event::Text(t) => t.xml10_content().into_owned(),
                    Event::GeneralRef(r) => super::text::xml_ref(&r),
                    _ => String::new(),
                };
                let s: String = s.chars().flat_map(|c| if matches!(c, '\\' | '*' | '_' | '`') { vec!['\\', c] } else { vec![c] }).collect();
                if s.trim().is_empty() {
                    push(&mut para, &s);
                } else {
                    let mark = match (bold, italic) {
                        (true, true) => "***",
                        (true, false) => "**",
                        (false, true) => "_",
                        _ => "",
                    };
                    push(&mut para, &format!("{mark}{s}{mark}"));
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                "t" => in_text = false,
                "r" => in_run = false,
                "p" => {
                    let Some(p) = para.take() else { continue };
                    let line = render(&p);
                    if let Some(c) = cell.as_mut() {
                        if !c.is_empty() {
                            c.push_str("<br>");
                        }
                        c.push_str(line.trim());
                    } else if !line.trim().is_empty() {
                        out += &line;
                        out += if p.list { "\n" } else { "\n\n" };
                    }
                }
                "tc" if depth == 1 => {
                    if let (Some(c), Some(row)) = (cell.take(), table.last_mut()) {
                        row.push(c.replace('|', "\\|"));
                    }
                }
                "tbl" => {
                    depth -= 1;
                    if depth == 0 && !table.is_empty() {
                        out += &md_table(&table);
                        out += "\n";
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    // runs that toggle formatting leave "**a****b**"; join them
    Ok(out.replace("****", "").replace("__", "").trim_end().to_string() + "\n")
}

fn push(p: &mut Option<Para>, s: &str) {
    if let Some(p) = p.as_mut() {
        p.text.push_str(s);
    }
}

fn render(p: &Para) -> String {
    let s = p.style.to_lowercase();
    let level = if s == "title" {
        Some(1)
    } else {
        s.strip_prefix("heading").and_then(|n| n.trim().parse::<usize>().ok()).map(|n| n.clamp(1, 6))
    };
    // headings are bold anyway
    let plain = || p.text.replace("**", "");
    match level {
        Some(n) => format!("{} {}", "#".repeat(n), plain().trim()),
        None if p.list => format!("- {}", p.text.trim()),
        None if s.contains("quote") => format!("> {}", p.text.trim()),
        None => p.text.trim_end().to_string(),
    }
}

fn md_table(rows: &[Vec<String>]) -> String {
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let line = |r: &Vec<String>| format!("| {} |\n", (0..cols).map(|i| r.get(i).cloned().unwrap_or_default()).collect::<Vec<_>>().join(" | "));
    let mut s = line(&rows[0]);
    s += &format!("|{}\n", " --- |".repeat(cols));
    for r in &rows[1..] {
        s += &line(r);
    }
    s
}

fn attr(e: &quick_xml::events::BytesStart, name: &str) -> Option<String> {
    e.attributes().flatten().find(|a| a.key.local_name().as_ref() == name).and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok().map(|v| v.into_owned()))
}
