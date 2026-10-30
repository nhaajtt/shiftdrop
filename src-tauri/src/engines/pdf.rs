use super::{image as img, note, pdfwrite, tools, Action, Level, Progress};
use crate::naming::{kind, out_dir, out_path, stem, unique, Kind};
use anyhow::{anyhow, bail, Result};
use image::{DynamicImage, Rgba};
use pdfium_render::prelude::*;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

pub fn pdfium() -> Result<&'static Pdfium> {
    if let Some(p) = PDFIUM.get() {
        return Ok(p);
    }
    let dir = tools::bin_dir().ok_or_else(|| anyhow!("PDF engine not found"))?;
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir))
        .or_else(|_| Pdfium::bind_to_system_library())
        .map_err(|e| anyhow!("PDF engine (pdfium) missing: {e}"))?;
    Ok(PDFIUM.get_or_init(|| Pdfium::new(bindings)))
}

pub fn load(path: &Path) -> Result<PdfDocument<'static>> {
    pdfium()?.load_pdf_from_file(path, None).map_err(|e| anyhow!("can't open PDF: {e}"))
}

/// Renders every page at `dpi`, calling `each(index, image, page size in points)`.
fn render_pages(doc: &PdfDocument, dpi: f32, progress: Progress, mut each: impl FnMut(usize, DynamicImage, (f64, f64)) -> Result<()>) -> Result<()> {
    let n = doc.pages().len() as usize;
    for (i, page) in doc.pages().iter().enumerate() {
        let size = (page.width().value as f64, page.height().value as f64);
        // a poster-sized or broken page box must not ask for gigabytes of pixels
        let bmp = page.render_with_config(&PdfRenderConfig::new().scale_page_by_factor(dpi / 72.0).set_maximum_width(10000).set_maximum_height(10000))?;
        each(i, bmp.as_image()?, size)?;
        progress((i + 1) as f32 / n as f32);
    }
    Ok(())
}

pub fn run(src: &Path, action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    let doc = load(src)?;
    let n = doc.pages().len() as usize;
    if n == 0 {
        bail!("this PDF has no pages");
    }
    match action {
        Action::Convert { to } if matches!(to.as_str(), "jpg" | "png" | "webp") => {
            let mut outs = Vec::new();
            let dir = if n > 1 { Some(out_dir(src, "pages")?) } else { None };
            render_pages(&doc, 150.0, progress, |i, im, _| {
                let out = match &dir {
                    Some(d) => unique(d, &format!("{} {:02}", stem(src), i + 1), to),
                    None => out_path(src, None, to),
                };
                img::save(&im, &out, to, 90)?;
                outs.push(out);
                Ok(())
            })?;
            Ok(outs)
        }
        Action::Compress { level } => {
            let (dpi, q) = match level {
                Level::Low => (150.0, 75),
                Level::Medium => (110.0, 62),
                Level::High => (80.0, 50),
            };
            let mut pages = Vec::new();
            render_pages(&doc, dpi, progress, |_, im, pt| {
                pages.push(pdfwrite::Page { px: (im.width(), im.height()), jpeg: img::jpeg_bytes(&im, q)?, pt });
                Ok(())
            })?;
            let out = out_path(src, Some("Compressed"), "pdf");
            pdfwrite::write(&out, &pages)?;
            // pages are redrawn as pictures: a text PDF can come out bigger, and it loses its text
            let (before, after) = (std::fs::metadata(src)?.len(), std::fs::metadata(&out)?.len());
            if after >= before {
                let _ = std::fs::remove_file(&out);
                bail!("this PDF is already as small as it gets");
            }
            note("Pages were redrawn as pictures: text can no longer be selected");
            Ok(vec![out])
        }
        Action::Split => {
            if n < 2 {
                bail!("this PDF has only one page");
            }
            let dir = out_dir(src, "pages")?;
            let mut outs = Vec::new();
            for i in 0..n {
                let mut one = pdfium()?.create_new_pdf()?;
                one.pages_mut().copy_page_from_document(&doc, i as _, 0)?;
                let out = unique(&dir, &format!("{} p{:02}", stem(src), i + 1), "pdf");
                one.save_to_file(&out)?;
                outs.push(out);
                progress((i + 1) as f32 / n as f32);
            }
            Ok(outs)
        }
        Action::Redact { boxes } => {
            // boxes apply to the first page; every page is flattened so hidden text is gone
            let mut pages = Vec::new();
            render_pages(&doc, 150.0, progress, |i, im, pt| {
                let mut im = im.to_rgba8();
                if i == 0 {
                    let (w, h) = (im.width() as f64, im.height() as f64);
                    for b in boxes {
                        let (x0, y0) = ((b[0] * w) as u32, (b[1] * h) as u32);
                        let (x1, y1) = (((b[0] + b[2]) * w).min(w) as u32, ((b[1] + b[3]) * h).min(h) as u32);
                        for y in y0..y1 {
                            for x in x0..x1 {
                                im.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                            }
                        }
                    }
                }
                let im = DynamicImage::ImageRgba8(im);
                pages.push(pdfwrite::Page { px: (im.width(), im.height()), jpeg: img::jpeg_bytes(&im, 85)?, pt });
                Ok(())
            })?;
            let out = out_path(src, Some("Redacted"), "pdf");
            pdfwrite::write(&out, &pages)?;
            Ok(vec![out])
        }
        _ => bail!("not available for PDFs"),
    }
}

/// Merges PDFs and images (in the given order) into one PDF.
pub fn merge(paths: &[PathBuf], progress: Progress) -> Result<Vec<PathBuf>> {
    let out = out_path(&paths[0], Some("Merged"), "pdf");
    if paths.iter().all(|p| kind(p) == Kind::Image) {
        let mut pages = Vec::new();
        for (i, p) in paths.iter().enumerate() {
            pages.push(pdfwrite::Page::from_image(&img::open(p)?, 90)?);
            progress((i + 1) as f32 / paths.len() as f32);
        }
        pdfwrite::write(&out, &pages)?;
        return Ok(vec![out]);
    }
    let mut doc = pdfium()?.create_new_pdf()?;
    for (i, p) in paths.iter().enumerate() {
        if kind(p) == Kind::Pdf {
            doc.pages_mut().append(&load(p)?)?;
        } else {
            // images go through a one-page temp PDF so they keep their place in the order
            let tmp = crate::naming::temp_file("pdf");
            pdfwrite::write(&tmp, &[pdfwrite::Page::from_image(&img::open(p)?, 90)?])?;
            doc.pages_mut().append(&load(&tmp)?)?;
            let _ = std::fs::remove_file(tmp);
        }
        progress((i + 1) as f32 / paths.len() as f32);
    }
    doc.save_to_file(&out)?;
    Ok(vec![out])
}

/// Small page previews (JPEG data URLs) for the page and signature editors.
#[tauri::command]
pub async fn pdf_thumbs(path: String, width: u32) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>> {
        use base64::Engine;
        let doc = load(Path::new(&path))?;
        let mut out = Vec::new();
        for page in doc.pages().iter() {
            let im = page.render_with_config(&PdfRenderConfig::new().set_target_width(width.clamp(32, 800) as _).set_maximum_height(1600))?.as_image()?;
            out.push(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(img::jpeg_bytes(&im, 80)?)));
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
