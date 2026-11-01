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
                let f = Frame::of(&page);
                let t = PdfPageTextObject::new(&doc, &label, font, PdfPoints::new(size))?;
                let t = f.place(t.into(), f.w_shown() / 2.0 - label.len() as f32 * size * 0.25, 22.0, 0.0)?;
                page.objects_mut().add_object(t)?;
                progress((i + 1) as f32 / n as f32);
            }
            save(&doc, src, "Numbered")?
        }
        Action::Watermark { text } => {
            let font = unicode_font(&mut doc)?;
            let n = doc.pages().len();
            for (i, mut page) in doc.pages().iter().enumerate() {
                let f = Frame::of(&page);
                let size = (f.w.min(f.h) / 9.0).max(18.0);
                let mut t = PdfPageTextObject::new(&doc, text, font, PdfPoints::new(size))?;
                t.set_fill_color(PdfColor::new(120, 120, 140, 70))?;
                let tw = text.chars().count() as f32 * size * 0.55;
                t.translate(PdfPoints::new(-tw / 2.0), PdfPoints::new(-size / 3.0))?;
                let t = f.place(t.into(), f.w_shown() / 2.0, f.h_shown() / 2.0, 35.0)?;
                page.objects_mut().add_object(t)?;
                progress((i + 1) as f32 / n as f32);
            }
            save(&doc, src, "Watermarked")?
        }
        Action::Stamp { page, rect, png } => {
            let data = base64::engine::general_purpose::STANDARD.decode(png.trim_start_matches("data:image/png;base64,"))?;
            let stamp = image::load_from_memory(&data)?;
            let mut p = doc.pages().get(*page as _).context("no such page")?;
            let f = Frame::of(&p);
            let (w, h) = (f.w_shown(), f.h_shown());
            // rect is [x, y, w, h] as fractions from the top-left of the page as shown; PDF counts from the bottom
            let (sw, sh) = (rect[2] as f32 * w, rect[3] as f32 * h);
            let (x, y) = (rect[0] as f32 * w, h - rect[1] as f32 * h - sh);
            let im = PdfPageImageObject::new_with_size(&doc, &stamp, PdfPoints::new(sw), PdfPoints::new(sh))?;
            let im = f.place(im.into(), x, y, 0.0)?;
            p.objects_mut().add_object(im)?;
            drop(p);
            save(&doc, src, "Signed")?
        }
        Action::PdfText => {
            let mut text = String::new();
            let n = doc.pages().len() as usize;
            let mut scanned = 0;
            for (i, page) in doc.pages().iter().enumerate() {
                let mut t = page.text().map(|t| t.all()).unwrap_or_default();
                if t.trim().chars().count() < 8 {
                    // probably a scanned page: read it with OCR, but keep what text there was if OCR finds nothing
                    let png = temp_file("png");
                    let cfg = PdfRenderConfig::new().scale_page_by_factor(200.0 / 72.0).set_maximum_width(5000).set_maximum_height(5000);
                    if page.render_with_config(&cfg).ok().and_then(|r| r.as_image().ok()).is_some_and(|im| im.save(&png).is_ok()) {
                        if let Ok(read) = super::imagex::ocr_text(&png) {
                            if read.trim().chars().count() > t.trim().chars().count() {
                                t = read;
                                scanned += 1;
                            }
                        }
                    }
                    let _ = std::fs::remove_file(png);
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
                    each_image(&obj, 0, &mut |im| {
                        if let Ok(raw) = im.get_raw_image() {
                            if raw.width() >= 24 && raw.height() >= 24 {
                                count += 1;
                                let out = unique(&dir, &format!("p{:02} image {count:02}", i + 1), "png");
                                raw.save(out)?;
                            }
                        }
                        Ok(())
                    })?;
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

/// Pictures are often wrapped in form objects (reusable groups of content), so those are opened too.
fn each_image<F: FnMut(&PdfPageImageObject) -> Result<()>>(obj: &PdfPageObject, depth: u32, f: &mut F) -> Result<()> {
    if let Some(im) = obj.as_image_object() {
        f(im)?;
    } else if let Some(form) = obj.as_x_object_form_object() {
        // forms can nest (or loop in a broken file): stop somewhere sensible
        if depth < 16 {
            for k in 0..form.len() {
                if let Ok(child) = form.get(k) {
                    each_image(&child, depth + 1, f)?;
                }
            }
        }
    }
    Ok(())
}

fn save(doc: &PdfDocument, src: &Path, suffix: &str) -> Result<PathBuf> {
    let out = out_path(src, Some(suffix), "pdf");
    doc.save_to_file(&out)?;
    Ok(out)
}

/// A page as it is shown. Pages can carry a /Rotate and a crop box that doesn't start at 0,0,
/// while objects are placed in the unrotated page space: this maps one onto the other.
struct Frame {
    rot: u32,
    left: f32,
    bottom: f32,
    /// unrotated size of the visible box
    w: f32,
    h: f32,
}

impl Frame {
    fn of(page: &PdfPage) -> Frame {
        // the visible box (crop within media), also when the page inherits it from its parent
        let b = page.boundaries();
        let r = b.bounding().or_else(|_| b.crop()).or_else(|_| b.media()).map(|x| x.bounds).unwrap_or_else(|_| {
            // page.width/height are already turned by /Rotate
            let (w, h) = (page.width().value, page.height().value);
            let rot = rotation_deg(page.rotation().unwrap_or(PdfPageRenderRotation::None));
            if rot % 180 == 90 { PdfRect::new_from_values(0.0, 0.0, w, h) } else { PdfRect::new_from_values(0.0, 0.0, h, w) }
        });
        let rot = rotation_deg(page.rotation().unwrap_or(PdfPageRenderRotation::None));
        Frame { rot, left: r.left().value, bottom: r.bottom().value, w: r.width().value, h: r.height().value }
    }

    fn w_shown(&self) -> f32 {
        if self.rot % 180 == 90 { self.h } else { self.w }
    }

    fn h_shown(&self) -> f32 {
        if self.rot % 180 == 90 { self.w } else { self.h }
    }

    /// Page space point for (x, y) measured from the bottom-left of the page as shown.
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        let (u, v) = match self.rot {
            90 => (self.w - y, x),
            180 => (self.w - x, self.h - y),
            270 => (y, self.h - x),
            _ => (x, y),
        };
        (self.left + u, self.bottom + v)
    }

    /// Puts an object built upright at the origin at (x, y) on the page as shown, turned `deg`
    /// counter-clockwise as seen, so it reads the right way up on a rotated page too.
    fn place<'a>(&self, mut obj: PdfPageObject<'a>, x: f32, y: f32, deg: f32) -> Result<PdfPageObject<'a>> {
        let turn = deg + self.rot as f32;
        if turn != 0.0 {
            obj.rotate_counter_clockwise_degrees(turn)?;
        }
        let (px, py) = self.point(x, y);
        obj.translate(PdfPoints::new(px), PdfPoints::new(py))?;
        Ok(obj)
    }
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
    if password.chars().count() < 8 {
        bail!("password is too short");
    }
    let out = out_path(src, Some("Protected"), "pdf");
    // owner password = user password: anyone with it can also print and copy
    let o = qpdf_with_secret(&["--encrypt", password, password, "256", "--"], src, &out)?;
    // qpdf exits 3 for warnings but still writes the file
    if !matches!(o.status.code(), Some(0 | 3)) || !out.is_file() {
        bail!("{}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("qpdf failed"));
    }
    Ok(out)
}

fn unprotect(src: &Path, password: &str) -> Result<PathBuf> {
    let out = out_path(src, Some("Unlocked"), "pdf");
    let pw = format!("--password={password}");
    let o = qpdf_with_secret(&[&pw, "--decrypt"], src, &out)?;
    if !matches!(o.status.code(), Some(0 | 3)) || !out.is_file() {
        let msg = String::from_utf8_lossy(&o.stderr);
        bail!("{}", if msg.contains("invalid password") { "wrong password" } else { msg.lines().last().unwrap_or("qpdf failed") });
    }
    Ok(out)
}

/// Passwords go to qpdf in an argument file (in the user's own temp folder, deleted right after),
/// never on the command line where any program can read them.
fn qpdf_with_secret(args: &[&str], src: &Path, out: &Path) -> Result<std::process::Output> {
    if args.iter().any(|a| a.contains(['\n', '\r'])) {
        bail!("the password can't contain line breaks");
    }
    let file = temp_file("args");
    std::fs::write(&file, args.join("\n"))?;
    let r = super::tools::output_within(
        super::tools::quiet(&qpdf()?).arg(format!("@{}", file.display())).arg(src).arg(out),
        std::time::Duration::from_secs(300),
    );
    let _ = std::fs::remove_file(&file);
    r
}
