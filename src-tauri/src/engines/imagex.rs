//! More image tools: OCR, QR codes, icon and favicon sets, colour palette, watermark,
//! contact sheet and vector tracing.

use super::{image::open, image::save, note, Action, Progress};
use crate::naming::{out_dir, out_path, temp_file};
use anyhow::{bail, Context, Result};
use image::{imageops, DynamicImage, GenericImageView, Rgba, RgbaImage};
use std::path::{Path, PathBuf};

pub fn handles(a: &Action) -> bool {
    matches!(
        a,
        Action::Ocr { .. } | Action::QrRead | Action::QrMake | Action::IconSet { .. } | Action::Palette | Action::Watermark { .. } | Action::ContactSheet | Action::Vectorize
    )
}

pub fn run(paths: &[PathBuf], action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    let src = &paths[0];
    match action {
        Action::Ocr { to } => ocr(src, to, progress),
        Action::QrRead => {
            let text = qr_read(src)?;
            crate::clipboard::set(&text)?;
            note(&format!("QR: {}", ellipsis(&text, 80)));
            Ok(vec![])
        }
        Action::QrMake => qr_make(src).map(|p| vec![p]),
        Action::IconSet { web } => icon_set(src, *web),
        Action::Palette => palette(src).map(|p| vec![p]),
        Action::Watermark { text } => watermark(src, text).map(|p| vec![p]),
        Action::ContactSheet => contact_sheet(paths, progress).map(|p| vec![p]),
        Action::Vectorize => vectorize(src).map(|p| vec![p]),
        _ => bail!("not available"),
    }
}

fn ellipsis(s: &str, n: usize) -> String {
    let one = s.replace(['\r', '\n'], " ");
    if one.chars().count() > n { one.chars().take(n).collect::<String>() + "…" } else { one }
}

// --- OCR ------------------------------------------------------------------------------------

/// A PNG copy small enough for Windows OCR (or the file itself when it already fits).
fn ocr_ready(src: &Path) -> Result<(PathBuf, bool)> {
    let Ok(img) = open(src) else { return Ok((src.to_path_buf(), false)) };
    #[cfg(windows)]
    let max = crate::winrt::ocr_max_side();
    #[cfg(not(windows))]
    let max = 4096;
    let (w, h) = img.dimensions();
    // small text reads better when enlarged a little
    let scale = if w.max(h) > max { max as f32 / w.max(h) as f32 } else if w.max(h) < 1200 { 2.0 } else { 1.0 };
    let tmp = temp_file("png");
    let img = if scale != 1.0 { img.resize((w as f32 * scale) as u32, (h as f32 * scale) as u32, imageops::FilterType::Lanczos3) } else { img };
    img.save(&tmp)?;
    Ok((tmp, true))
}

pub fn ocr_text(src: &Path) -> Result<String> {
    #[cfg(windows)]
    {
        let (file, temp) = ocr_ready(src)?;
        let r = crate::winrt::ocr(&file);
        if temp {
            let _ = std::fs::remove_file(&file);
        }
        r
    }
    #[cfg(not(windows))]
    {
        let _ = src;
        bail!("text recognition is only available on Windows for now")
    }
}

fn ocr(src: &Path, to: &str, progress: Progress) -> Result<Vec<PathBuf>> {
    progress(0.2);
    let text = ocr_text(src)?;
    if text.trim().is_empty() {
        bail!("no text found");
    }
    if to == "txt" {
        let out = out_path(src, Some("text"), "txt");
        std::fs::write(&out, text)?;
        return Ok(vec![out]);
    }
    crate::clipboard::set(&text)?;
    note(&format!("Copied: {}", ellipsis(&text, 70)));
    Ok(vec![])
}

// --- QR ---------------------------------------------------------------------------------------

fn qr_read(src: &Path) -> Result<String> {
    let img = open(src)?.to_luma8();
    // try as is, then smaller (phone photos of a screen are often too large and noisy)
    for scale in [1.0f32, 0.5, 0.25] {
        let g = if scale < 1.0 {
            imageops::resize(&img, (img.width() as f32 * scale) as u32, (img.height() as f32 * scale) as u32, imageops::FilterType::Triangle)
        } else {
            img.clone()
        };
        let mut prep = rqrr::PreparedImage::prepare(g);
        for grid in prep.detect_grids() {
            if let Ok((_, text)) = grid.decode() {
                return Ok(text);
            }
        }
    }
    bail!("no QR code found")
}

/// A QR code holding the text of a small text file.
fn qr_make(src: &Path) -> Result<PathBuf> {
    let text = std::fs::read_to_string(src).context("not a text file")?;
    let text = text.trim();
    if text.is_empty() {
        bail!("the file is empty");
    }
    if text.len() > 2000 {
        bail!("too much text for a QR code (2000 characters at most)");
    }
    let code = qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M).context("too much text for a QR code")?;
    let img = code.render::<image::Luma<u8>>().min_dimensions(800, 800).quiet_zone(true).build();
    let out = out_path(src, Some("QR"), "png");
    img.save(&out)?;
    Ok(out)
}

// --- Icons -----------------------------------------------------------------------------------

/// Square, centred, transparent padding: icons must not be stretched.
fn square(img: &DynamicImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let side = w.max(h);
    let mut canvas = RgbaImage::from_pixel(side, side, Rgba([0, 0, 0, 0]));
    imageops::overlay(&mut canvas, &img.to_rgba8(), ((side - w) / 2) as i64, ((side - h) / 2) as i64);
    canvas
}

fn sized(sq: &RgbaImage, n: u32) -> RgbaImage {
    imageops::resize(sq, n, n, imageops::FilterType::Lanczos3)
}

fn write_ico(sq: &RgbaImage, sizes: &[u32], out: &Path) -> Result<()> {
    use image::codecs::ico::{IcoEncoder, IcoFrame};
    let frames: Vec<IcoFrame> = sizes
        .iter()
        .map(|&n| IcoFrame::as_png(sized(sq, n).as_raw(), n, n, image::ExtendedColorType::Rgba8))
        .collect::<Result<_, _>>()?;
    IcoEncoder::new(std::fs::File::create(out)?).encode_images(&frames)?;
    Ok(())
}

/// App icon (.ico with every Windows size) or a website favicon folder.
fn icon_set(src: &Path, web: bool) -> Result<Vec<PathBuf>> {
    let sq = square(&open(src)?);
    if !web {
        let out = out_path(src, Some("icon"), "ico");
        write_ico(&sq, &[16, 24, 32, 48, 64, 128, 256], &out)?;
        return Ok(vec![out]);
    }
    let dir = out_dir(src, "favicons")?;
    write_ico(&sq, &[16, 32, 48], &dir.join("favicon.ico"))?;
    for (n, name) in [(16, "favicon-16x16.png"), (32, "favicon-32x32.png"), (180, "apple-touch-icon.png"), (192, "android-chrome-192x192.png"), (512, "android-chrome-512x512.png")] {
        sized(&sq, n).save(dir.join(name))?;
    }
    std::fs::write(
        dir.join("site.webmanifest"),
        r#"{"icons":[{"src":"/android-chrome-192x192.png","sizes":"192x192","type":"image/png"},{"src":"/android-chrome-512x512.png","sizes":"512x512","type":"image/png"}],"display":"standalone"}"#,
    )?;
    std::fs::write(
        dir.join("head.html"),
        "<link rel=\"icon\" href=\"/favicon.ico\" sizes=\"48x48\">\n<link rel=\"icon\" type=\"image/png\" sizes=\"32x32\" href=\"/favicon-32x32.png\">\n<link rel=\"apple-touch-icon\" href=\"/apple-touch-icon.png\">\n<link rel=\"manifest\" href=\"/site.webmanifest\">\n",
    )?;
    Ok(vec![dir])
}

// --- Palette ------------------------------------------------------------------------------

/// Main colours by k-means on a thumbnail; writes a swatch image and copies the hex codes.
fn palette(src: &Path) -> Result<PathBuf> {
    let img = open(src)?.thumbnail(160, 160).to_rgb8();
    let px: Vec<[f32; 3]> = img.pixels().map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]).collect();
    if px.is_empty() {
        bail!("empty image");
    }
    let k = 6.min(px.len());
    let mut centers: Vec<[f32; 3]> = (0..k).map(|i| px[i * px.len() / k]).collect();
    let mut counts = vec![0usize; k];
    for _ in 0..12 {
        let mut sums = vec![[0f32; 3]; k];
        counts = vec![0; k];
        for p in &px {
            let c = (0..k)
                .min_by(|&a, &b| dist(p, &centers[a]).total_cmp(&dist(p, &centers[b])))
                .unwrap_or(0);
            counts[c] += 1;
            for i in 0..3 {
                sums[c][i] += p[i];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                centers[c] = sums[c].map(|v| v / counts[c] as f32);
            }
        }
    }
    let mut order: Vec<usize> = (0..k).filter(|&c| counts[c] > 0).collect();
    order.sort_by_key(|&c| std::cmp::Reverse(counts[c]));
    let hex: Vec<String> = order.iter().map(|&c| format!("#{:02X}{:02X}{:02X}", centers[c][0] as u8, centers[c][1] as u8, centers[c][2] as u8)).collect();
    let (sw, sh) = (160u32, 200u32);
    let mut out_img = RgbaImage::from_pixel(sw * order.len() as u32, sh, Rgba([255, 255, 255, 255]));
    for (i, &c) in order.iter().enumerate() {
        let col = Rgba([centers[c][0] as u8, centers[c][1] as u8, centers[c][2] as u8, 255]);
        for y in 0..sh {
            for x in 0..sw {
                out_img.put_pixel(i as u32 * sw + x, y, col);
            }
        }
    }
    let out = out_path(src, Some("palette"), "png");
    out_img.save(&out)?;
    crate::clipboard::set(&hex.join(" "))?;
    note(&format!("Colours copied: {}", hex.join(" ")));
    Ok(out)
}

fn dist(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

// --- Watermark (drawn by ffmpeg so any script and font works) ---------------------------------

fn watermark(src: &Path, text: &str) -> Result<PathBuf> {
    let text = text.trim();
    if text.is_empty() {
        bail!("no watermark text");
    }
    let fonts = std::env::var_os("WINDIR").map(|w| Path::new(&w).join("Fonts")).context("no fonts folder")?;
    let font = ["segoeuib.ttf", "arialbd.ttf", "segoeui.ttf", "arial.ttf"].iter().map(|f| fonts.join(f)).find(|p| p.is_file()).context("no font found")?;
    // ffmpeg runs in a scratch folder holding the font and text, so no path needs escaping
    let dir = temp_file("wm");
    std::fs::create_dir_all(&dir)?;
    std::fs::copy(&font, dir.join("font.ttf"))?;
    std::fs::write(dir.join("text.txt"), text)?;
    let fmt = super::image::keep_fmt(src);
    let png = dir.join("in.png");
    open(src)?.save(&png)?;
    let tmp_out = dir.join("out.png");
    let vf = "drawtext=fontfile=font.ttf:textfile=text.txt:fontcolor=white@0.55:borderw=2:bordercolor=black@0.35:fontsize=h/18:x=w-tw-h/30:y=h-th-h/30";
    let args: Vec<String> = ["-i", "in.png", "-vf", vf, "out.png"].iter().map(|s| s.to_string()).collect();
    let r = super::tools::ffmpeg_in(Some(&dir), &args, 0.0, &|_| {});
    let result = r.and_then(|_| {
        let out = out_path(src, Some("Watermarked"), &fmt);
        save(&image::open(&tmp_out)?, &out, &fmt, 92)?;
        Ok(out)
    });
    let _ = std::fs::remove_dir_all(&dir);
    result
}

// --- Contact sheet ----------------------------------------------------------------------------

fn contact_sheet(paths: &[PathBuf], progress: Progress) -> Result<PathBuf> {
    let n = paths.len();
    let cols = (n as f64).sqrt().ceil() as u32;
    let rows = (n as u32).div_ceil(cols);
    // the sheet stays under ~8000 px a side however many pictures there are
    let cell = (8000 / cols).clamp(64, 400) - 12;
    let gap = 12u32;
    let mut sheet = RgbaImage::from_pixel(cols * (cell + gap) + gap, rows * (cell + gap) + gap, Rgba([255, 255, 255, 255]));
    let mut drawn = 0;
    for (i, p) in paths.iter().enumerate() {
        progress(i as f32 / n as f32);
        let Ok(img) = open(p) else { continue };
        drawn += 1;
        let t = img.thumbnail(cell, cell).to_rgba8();
        let (c, r) = (i as u32 % cols, i as u32 / cols);
        let x = gap + c * (cell + gap) + (cell - t.width()) / 2;
        let y = gap + r * (cell + gap) + (cell - t.height()) / 2;
        imageops::overlay(&mut sheet, &t, x as i64, y as i64);
    }
    if drawn == 0 {
        bail!("none of these pictures could be opened");
    }
    let out = out_path(&paths[0], Some("Contact sheet"), "jpg");
    save(&DynamicImage::ImageRgba8(sheet), &out, "jpg", 90)?;
    Ok(out)
}

// --- Vector tracing -------------------------------------------------------------------------

fn vectorize(src: &Path) -> Result<PathBuf> {
    let img = open(src)?;
    // tracing time grows fast with size; logos and drawings look the same at 1600 px
    let img = if img.width().max(img.height()) > 1600 { img.resize(1600, 1600, imageops::FilterType::Lanczos3) } else { img };
    let png = temp_file("png");
    img.save(&png)?;
    let out = out_path(src, None, "svg");
    let cfg = vtracer::Config { filter_speckle: 4, color_precision: 6, layer_difference: 16, ..Default::default() };
    let r = vtracer::convert_image_to_svg(&png, &out, cfg).map_err(|e| anyhow::anyhow!(e));
    let _ = std::fs::remove_file(png);
    r?;
    Ok(out)
}
