//! More PDF tools: rotate, rearrange pages, page numbers, watermark, signature stamp,
//! text (with OCR for scanned pages), images out, and passwords (through qpdf).

use super::{note, pdf, security, Action, Progress};
use crate::naming::{out_dir, out_path, temp_file, unique};
use anyhow::{bail, Context, Result};
use base64::Engine;
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

pub fn handles(a: &Action) -> bool {
    matches!(
        a,
        Action::Pages { .. }
            | Action::PageNumbers
            | Action::Watermark { .. }
            | Action::Stamp { .. }
            | Action::PdfText
            | Action::ExtractImages
            | Action::Protect { .. }
            | Action::Unprotect { .. }
            | Action::Rotate { .. }
    )
}

pub fn run(src: &Path, action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    match action {
        Action::Protect { password } => return protect(src, password).map(|p| vec![p]),
        Action::Unprotect { password } => return unprotect(src, password).map(|p| vec![p]),
        _ => {}
    }
    let mut doc = pdf::load(src)?;
    let out = match action {
        Action::Rotate { deg } => {
            for mut page in doc.pages().iter() {
                let now = rotation_deg(page.rotation().unwrap_or(PdfPageRenderRotation::None));
                page.set_rotation(rotation((now + deg) % 360));
            }
            save(&doc, src, "Rotated")?
        }
        Action::Pages { order, rotate } => return pages(&doc, src, order, rotate),
        Action::PageNumbers => {
            let font = doc.fonts_mut().helvetica();
            let n = doc.pages().len();
            for (i, mut page) in doc.pages().iter().enumerate() {
                let label = format!("{} / {n}", i + 1);
                let size = 10.0;
                let x = page.width().value / 2.0 - label.len() as f32 * size * 0.25;
                page.objects_mut().create_text_object(PdfPoints::new(x), PdfPoints::new(22.0), label, font, PdfPoints::new(size))?;
                progress((i + 1) as f32 / n as f32);
            }
            save(&doc, src, "Numbered")?
        }
        Action::Watermark { text } => {
            let font = unicode_font(&mut doc)?;
            let n = doc.pages().len();
            for (i, mut page) in doc.pages().iter().enumerate() {
                let (w, h) = (page.width().value, page.height().value);
                let size = (w.min(h) / 9.0).max(18.0);
                let mut t = PdfPageTextObject::new(&doc, text, font, PdfPoints::new(size))?;
                t.set_fill_color(PdfColor::new(120, 120, 140, 70))?;
                let tw = text.chars().count() as f32 * size * 0.55;
                t.translate(PdfPoints::new(-tw / 2.0), PdfPoints::new(-size / 3.0))?;
                t.rotate_counter_clockwise_degrees(35.0)?;
                t.translate(PdfPoints::new(w / 2.0), PdfPoints::new(h / 2.0))?;
                page.objects_mut().add_text_object(t)?;
                progress((i + 1) as f32 / n as f32);
            }
            save(&doc, src, "Watermarked")?
        }
        Action::Stamp { page, rect, png } => {
            let data = base64::engine::general_purpose::STANDARD.decode(png.trim_start_matches("data:image/png;base64,"))?;
            let stamp = image::load_from_memory(&data)?;
            let mut p = doc.pages().get(*page as _).context("no such page")?;
            let (w, h) = (p.width().value, p.height().value);
            // rect is [x, y, w, h] as fractions from the top-left; PDF counts from the bottom
            let (sw, sh) = (rect[2] as f32 * w, rect[3] as f32 * h);
            let (x, y) = (rect[0] as f32 * w, h - rect[1] as f32 * h - sh);
            p.objects_mut().create_image_object(PdfPoints::new(x), PdfPoints::new(y), &stamp, Some(PdfPoints::new(sw)), Some(PdfPoints::new(sh)))?;
            drop(p);
            save(&doc, src, "Signed")?
        }
        Action::PdfText => {
            let mut text = String::new();
            let n = doc.pages().len() as usize;
            let mut scanned = 0;
            for (i, page) in doc.pages().iter().enumerate() {
                let mut t = page.text().map(|t| t.all()).unwrap_or_default();
                if t.trim().len() < 8 {
                    // a scanned page: read it with OCR
                    let png = temp_file("png");
                    page.render_with_config(&PdfRenderConfig::new().scale_page_by_factor(200.0 / 72.0))?.as_image()?.save(&png)?;
                    t = super::imagex::ocr_text(&png).unwrap_or_default();
                    let _ = std::fs::remove_file(png);
                    scanned += 1;
                }
                text += &format!("--- Page {} ---\n{}\n\n", i + 1, t.trim());
                progress((i + 1) as f32 / n as f32);
            }
            if scanned > 0 {
                note(&format!("{scanned} scanned page{} read with OCR", if scanned == 1 { "" } else { "s" }));
            }
            let out = out_path(src, Some("text"), "txt");
            std::fs::write(&out, text)?;
            out
        }
        Action::ExtractImages => {
            let dir = out_dir(src, "images")?;
            let mut count = 0;
            let n = doc.pages().len() as usize;
            for (i, page) in doc.pages().iter().enumerate() {
                for obj in page.objects().iter() {
                    if let Some(im) = obj.as_image_object() {
                        if let Ok(raw) = im.get_raw_image() {
                            if raw.width() < 24 || raw.height() < 24 {
                                continue;
                            }
                            count += 1;
                            let out = unique(&dir, &format!("p{:02} image {count:02}", i + 1), "png");
                            raw.save(out)?;
                        }
                    }
                }
                progress((i + 1) as f32 / n as f32);
            }
            if count == 0 {
                let _ = std::fs::remove_dir(&dir);
                bail!("no images in this PDF");
            }
            note(&format!("{count} image{} saved", if count == 1 { "" } else { "s" }));
            dir
        }
        _ => bail!("not available for PDFs"),
    };
    Ok(vec![out])
}

fn save(doc: &PdfDocument, src: &Path, suffix: &str) -> Result<PathBuf> {
    let out = out_path(src, Some(suffix), "pdf");
    doc.save_to_file(&out)?;
    Ok(out)
}

fn rotation(deg: u32) -> PdfPageRenderRotation {
    match deg % 360 {
        90 => PdfPageRenderRotation::Degrees90,
        180 => PdfPageRenderRotation::Degrees180,
        270 => PdfPageRenderRotation::Degrees270,
        _ => PdfPageRenderRotation::None,
    }
}

fn rotation_deg(r: PdfPageRenderRotation) -> u32 {
    match r {
        PdfPageRenderRotation::Degrees90 => 90,
        PdfPageRenderRotation::Degrees180 => 180,
        PdfPageRenderRotation::Degrees270 => 270,
        PdfPageRenderRotation::None => 0,
    }
}

/// Keeps the listed pages (0-based) in that order, each turned by `rotate[i]` degrees.
fn pages(doc: &PdfDocument, src: &Path, order: &[u32], rotate: &[u32]) -> Result<Vec<PathBuf>> {
    if order.is_empty() {
        bail!("no pages left");
    }
    let mut out_doc = pdf::pdfium()?.create_new_pdf()?;
    for (i, &p) in order.iter().enumerate() {
        out_doc.pages_mut().copy_page_from_document(doc, p as _, i as _)?;
        let extra = rotate.get(i).copied().unwrap_or(0);
        if extra % 360 != 0 {
            let mut page = out_doc.pages().get(i as _)?;
            let now = rotation_deg(page.rotation().unwrap_or(PdfPageRenderRotation::None));
            page.set_rotation(rotation((now + extra) % 360));
        }
    }
    let out = out_path(src, Some("Edited"), "pdf");
    out_doc.save_to_file(&out)?;
    Ok(vec![out])
}

/// A Windows font that covers Vietnamese and most scripts, embedded in the PDF.
fn unicode_font(doc: &mut PdfDocument) -> Result<PdfFontToken> {
    let fonts = std::env::var_os("WINDIR").map(|w| Path::new(&w).join("Fonts"));
    if let Some(f) = fonts.and_then(|d| ["arial.ttf", "segoeui.ttf"].iter().map(|n| d.join(n)).find(|p| p.is_file())) {
        if let Ok(t) = doc.fonts_mut().load_true_type_from_file(&f, true) {
            return Ok(t);
        }
    }
    Ok(doc.fonts_mut().helvetica())
}

// --- Passwords: qpdf (downloaded in Settings › Add-ons) --------------------------------------

fn qpdf() -> Result<PathBuf> {
    security::tool_path("qpdf").context("Passwords need the qpdf add-on: Settings › Add-ons")
}

fn protect(src: &Path, password: &str) -> Result<PathBuf> {
    if password.len() < 4 {
        bail!("password is too short");
    }
    let out = out_path(src, Some("Protected"), "pdf");
    // owner password = user password: anyone with it can also print and copy
    let o = super::tools::quiet(&qpdf()?).arg("--encrypt").arg(password).arg(password).args(["256", "--"]).arg(src).arg(&out).output()?;
    if !o.status.success() {
        bail!("{}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("qpdf failed"));
    }
    Ok(out)
}

fn unprotect(src: &Path, password: &str) -> Result<PathBuf> {
    let out = out_path(src, Some("Unlocked"), "pdf");
    let o = super::tools::quiet(&qpdf()?).arg(format!("--password={password}")).arg("--decrypt").arg(src).arg(&out).output()?;
    // qpdf exits 3 for warnings but still writes the file
    if !out.is_file() {
        let msg = String::from_utf8_lossy(&o.stderr);
        bail!("{}", if msg.contains("invalid password") { "wrong password" } else { msg.lines().last().unwrap_or("qpdf failed") });
    }
    Ok(out)
}
