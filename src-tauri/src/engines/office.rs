//! Word, Excel and PowerPoint documents: converted by Microsoft Office when it is installed,
//! otherwise by LibreOffice (free). Markdown and HTML can become .docx the same way.

use super::{text, Action, Progress};
use crate::naming::{ext, out_path, temp_file};
use anyhow::{bail, Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

const NEED: &str = "Office documents need Microsoft Office or the free LibreOffice (libreoffice.org)";

pub fn run(src: &Path, action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    let Action::Convert { to } = action else { bail!("not available for documents") };
    progress(0.1);
    if to == "md" && ext(src) == "docx" {
        return super::docx::to_markdown(src).map(|p| vec![p]);
    }
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

/// Longest a single document may take before Office or LibreOffice is stopped.
const LIMIT: Duration = Duration::from_secs(180);

fn convert(src: &Path, out: &Path, to: &str) -> Result<()> {
    let mut office_err = None;
    if let Some(app) = ms_office_app(src) {
        match ms_office(app, src, out, to) {
            Ok(()) if out.is_file() => return Ok(()),
            Ok(()) => office_err = Some(anyhow::anyhow!("{app} didn't produce the file")),
            Err(e) => office_err = Some(e),
        }
        let _ = std::fs::remove_file(out);
    }
    let Some(soffice) = libreoffice() else {
        // Office is installed but failed: its reason is more useful than "install LibreOffice"
        return Err(office_err.unwrap_or_else(|| anyhow::anyhow!(NEED)));
    };
    let dir = temp_file("lo");
    std::fs::create_dir_all(&dir)?;
    let filter = match to {
        "docx" => "docx:MS Word 2007 XML",
        "txt" => "txt:Text (encoded):UTF8",
        other => other,
    };
    // a profile of our own, so a LibreOffice the user has open doesn't swallow the request
    let profile = std::env::temp_dir().join("shiftdrop").join("lo-profile");
    let profile_url = format!("-env:UserInstallation=file:///{}", profile.display().to_string().replace('\\', "/").replace(' ', "%20"));
    let o = super::tools::output_within(
        super::tools::quiet(soffice).arg(profile_url).args(["--headless", "--norestore", "--nologo", "--convert-to", filter, "--outdir"]).arg(&dir).arg(src),
        LIMIT,
    );
    let o = match o {
        Ok(o) => o,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e.context("LibreOffice couldn't convert it"));
        }
    };
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
            .filter_map(std::env::var_os)
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
/// AutomationSecurity=3 (force disable) keeps macros in the document from running: Office
/// otherwise enables them for automation.
/// A wrong password ('-') is given on purpose: a protected file then fails at once instead of
/// waiting on a hidden password prompt. Office is always closed again, also after an error.
fn ms_office(app: &str, src: &Path, out: &Path, to: &str) -> Result<()> {
    let (open, save, close) = match (app, to) {
        ("Word", _) => (
            "$a=New-Object -ComObject Word.Application;$a.DisplayAlerts=0;$a.AutomationSecurity=3;$a.Visible=$false;$d=$a.Documents.Open($env:SD_IN,$false,$true,$false,'-','-')",
            match to {
                "pdf" => "$d.ExportAsFixedFormat($env:SD_OUT,17)",
                "docx" => "$d.SaveAs2($env:SD_OUT,16)",
                "txt" => "$d.SaveAs2($env:SD_OUT,7,$false,'',$false,'',$false,$false,$false,$false,$false,65001)",
                _ => bail!("Word can't make .{to}"),
            },
            "$d.Close($false)",
        ),
        ("Excel", "pdf") => (
            "$a=New-Object -ComObject Excel.Application;$a.DisplayAlerts=$false;$a.AutomationSecurity=3;$d=$a.Workbooks.Open($env:SD_IN,0,$true,[Type]::Missing,'-')",
            "$d.ExportAsFixedFormat(0,$env:SD_OUT)",
            "$d.Close($false)",
        ),
        // PowerPoint runs once per user: if it is already open we would take over (and quit) the user's own
        ("PowerPoint", "pdf") => (
            "if(Get-Process POWERPNT -EA SilentlyContinue){throw 'PowerPoint is open. Close it and try again.'};$a=New-Object -ComObject PowerPoint.Application;$a.AutomationSecurity=3;$d=$a.Presentations.Open($env:SD_IN,$true,$false,$false)",
            "$d.SaveAs($env:SD_OUT,32)",
            "$d.Close()",
        ),
        _ => bail!("{app} can't make .{to}"),
    };
    let script = format!(
        "$ErrorActionPreference='Stop';$a=$null;$d=$null;$f=$null;try{{{open};{save}}}catch{{$f=$_.Exception.Message}}finally{{if($d){{try{{{close}}}catch{{}}}};if($a){{try{{$a.Quit()}}catch{{}}}}}};if($f){{[Console]::Error.WriteLine($f);exit 1}}"
    );
    let o = super::tools::output_within(
        super::tools::quiet(Path::new("powershell"))
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .env("SD_IN", src)
            .env("SD_OUT", out),
        LIMIT,
    )
    .with_context(|| format!("{app} couldn't convert it"))?;
    if !o.status.success() {
        let err = String::from_utf8_lossy(&o.stderr);
        let first = err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("unknown error");
        if first.to_lowercase().contains("password") {
            bail!("{app} failed: the file is password-protected. Remove the password in {app} first.");
        }
        bail!("{app} failed: {first}");
    }
    Ok(())
}

#[tauri::command]
pub async fn office_ok() -> bool {
    libreoffice().is_some() || ["Word", "Excel", "PowerPoint"].iter().any(|a| installed(a))
}
