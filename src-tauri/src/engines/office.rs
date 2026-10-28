//! Word, Excel and PowerPoint documents: converted by Microsoft Office when it is installed,
//! otherwise by LibreOffice (free). Markdown and HTML can become .docx the same way.

use super::{text, Action, Progress};
use crate::naming::{ext, out_path, temp_file};
use anyhow::{bail, Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

const NEED: &str = "Office documents need Microsoft Office or the free LibreOffice (libreoffice.org)";

pub fn run(src: &Path, action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    let Action::Convert { to } = action else { bail!("not available for documents") };
    progress(0.1);
    let input = if matches!(ext(src).as_str(), "md" | "markdown" | "txt" | "csv") {
        // Markdown etc. go through HTML so headings, lists and tables survive
        let html = temp_file("html");
        std::fs::write(&html, text::to_html(src)?)?;
        Some(html)
    } else {
        None
    };
    let from = input.as_deref().unwrap_or(src);
    let out = out_path(src, None, to);
    let r = convert(from, &out, to);
    if let Some(h) = &input {
        let _ = std::fs::remove_file(h);
    }
    r?;
    Ok(vec![out])
}

fn convert(src: &Path, out: &Path, to: &str) -> Result<()> {
    if let Some(app) = ms_office_app(src) {
        if ms_office(app, src, out, to).is_ok() && out.is_file() {
            return Ok(());
        }
    }
    let soffice = libreoffice().context(NEED)?;
    let dir = temp_file("lo");
    std::fs::create_dir_all(&dir)?;
    let filter = match to {
        "docx" => "docx:MS Word 2007 XML",
        "txt" => "txt:Text (encoded):UTF8",
        other => other,
    };
    let o = super::tools::quiet(soffice)
        .args(["--headless", "--norestore", "--convert-to", filter, "--outdir"])
        .arg(&dir)
        .arg(src)
        .output()?;
    let made = std::fs::read_dir(&dir)?.flatten().map(|e| e.path()).find(|p| p.is_file());
    let r = match made {
        Some(f) => std::fs::rename(&f, out).or_else(|_| std::fs::copy(&f, out).map(|_| ())).map_err(Into::into),
        None => Err(anyhow::anyhow!("LibreOffice couldn't convert it: {}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or(""))),
    };
    let _ = std::fs::remove_dir_all(&dir);
    r
}

fn libreoffice() -> Option<&'static Path> {
    static P: OnceLock<Option<PathBuf>> = OnceLock::new();
    P.get_or_init(|| {
        ["ProgramFiles", "ProgramFiles(x86)"]
            .iter()
            .filter_map(|v| std::env::var_os(v))
            .map(|d| Path::new(&d).join("LibreOffice").join("program").join("soffice.exe"))
            .find(|p| p.is_file())
    })
    .as_deref()
}

/// Which Office app opens this file, if that app is installed.
fn ms_office_app(src: &Path) -> Option<&'static str> {
    let app = match ext(src).as_str() {
        "xls" | "xlsx" | "xlsm" | "ods" => "Excel",
        "ppt" | "pptx" | "odp" => "PowerPoint",
        _ => "Word",
    };
    installed(app).then_some(app)
}

fn installed(app: &str) -> bool {
    static CACHE: OnceLock<[bool; 3]> = OnceLock::new();
    let found = CACHE.get_or_init(|| {
        ["Word", "Excel", "PowerPoint"].map(|a| {
            super::tools::quiet(Path::new("reg")).args(["query", &format!("HKCR\\{a}.Application\\CLSID")]).output().map(|o| o.status.success()).unwrap_or(false)
        })
    });
    match app {
        "Word" => found[0],
        "Excel" => found[1],
        _ => found[2],
    }
}

/// Drives Office through COM from PowerShell; paths go in environment variables, never in the script.
fn ms_office(app: &str, src: &Path, out: &Path, to: &str) -> Result<()> {
    let script = match (app, to) {
        ("Word", "pdf") => "$a=New-Object -ComObject Word.Application;$a.Visible=$false;$d=$a.Documents.Open($env:SD_IN,$false,$true);$d.ExportAsFixedFormat($env:SD_OUT,17);$d.Close($false);$a.Quit()",
        ("Word", "docx") => "$a=New-Object -ComObject Word.Application;$a.Visible=$false;$d=$a.Documents.Open($env:SD_IN,$false,$true);$d.SaveAs2($env:SD_OUT,16);$d.Close($false);$a.Quit()",
        ("Word", "txt") => "$a=New-Object -ComObject Word.Application;$a.Visible=$false;$d=$a.Documents.Open($env:SD_IN,$false,$true);$d.SaveAs2($env:SD_OUT,7,$false,'',$false,'',$false,$false,$false,$false,$false,65001);$d.Close($false);$a.Quit()",
        ("Excel", "pdf") => "$a=New-Object -ComObject Excel.Application;$a.DisplayAlerts=$false;$b=$a.Workbooks.Open($env:SD_IN,0,$true);$b.ExportAsFixedFormat(0,$env:SD_OUT);$b.Close($false);$a.Quit()",
        ("PowerPoint", "pdf") => "$a=New-Object -ComObject PowerPoint.Application;$p=$a.Presentations.Open($env:SD_IN,$true,$false,$false);$p.SaveAs($env:SD_OUT,32);$p.Close();$a.Quit()",
        _ => bail!("{app} can't make .{to}"),
    };
    let o = super::tools::quiet(Path::new("powershell"))
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .env("SD_IN", src)
        .env("SD_OUT", out)
        .output()?;
    if !o.status.success() {
        bail!("{app} failed: {}", String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or(""));
    }
    Ok(())
}

#[tauri::command]
pub fn office_ok() -> bool {
    libreoffice().is_some() || ["Word", "Excel", "PowerPoint"].iter().any(|a| installed(a))
}
