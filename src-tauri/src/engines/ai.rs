//! On-device models (downloaded once in Settings › Add-ons): background removal, face blurring,
//! and upscaling with Real-ESRGAN. Nothing is uploaded.

use super::{image::open, image::save, note, security, Action, Progress};
use crate::naming::{out_path, temp_file};
use anyhow::{bail, Context, Result};
use image::{imageops, DynamicImage, GenericImageView, Rgba, RgbaImage};
use ort::{session::Session, value::Tensor};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

pub fn handles(a: &Action) -> bool {
    matches!(a, Action::RemoveBg | Action::BlurFaces | Action::Upscale { .. })
}

pub fn run(paths: &[PathBuf], action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    let src = &paths[0];
    match action {
        Action::RemoveBg => remove_bg(src, progress).map(|p| vec![p]),
        Action::BlurFaces => blur_faces(src).map(|p| vec![p]),
        Action::Upscale { scale } => upscale(src, *scale).map(|p| vec![p]),
        _ => bail!("not available"),
    }
}

const MISSING: &str = "the AI add-on isn't installed yet: Settings › Add-ons";

/// Loads onnxruntime.dll from the add-on folder once. Only success is remembered, so installing the
/// add-on after a first failed try works without restarting.
fn runtime() -> Result<()> {
    static READY: Mutex<bool> = Mutex::new(false);
    let mut ready = READY.lock().unwrap_or_else(|e| e.into_inner());
    if !*ready {
        let dll = security::tool_path("ai").context(MISSING)?;
        ort::init_from(dll.to_string_lossy().to_string()).commit().map_err(|e| anyhow::anyhow!("the AI add-on couldn't start: {e}"))?;
        *ready = true;
    }
    Ok(())
}

fn session(cache: &'static OnceLock<Mutex<Session>>, model: &str) -> Result<&'static Mutex<Session>> {
    if let Some(s) = cache.get() {
        return Ok(s);
    }
    let path = security::tool_file("ai", model).context(MISSING)?;
    runtime()?;
    let s = Session::builder()?.commit_from_file(path)?;
    Ok(cache.get_or_init(|| Mutex::new(s)))
}

/// NCHW float tensor from an RGB image, normalised per channel.
fn tensor(img: &image::RgbImage, mean: [f32; 3], std: [f32; 3], scale: f32) -> Result<Tensor<f32>> {
    let (w, h) = img.dimensions();
    let n = (w * h) as usize;
    let mut data = vec![0f32; 3 * n];
    for (i, p) in img.pixels().enumerate() {
        for c in 0..3 {
            data[c * n + i] = (p[c] as f32 / scale - mean[c]) / std[c];
        }
    }
    Ok(Tensor::from_array(([1usize, 3, h as usize, w as usize], data.into_boxed_slice()))?)
}

// --- Background removal (IS-Net) ------------------------------------------------------------

fn remove_bg(src: &Path, progress: Progress) -> Result<PathBuf> {
    static S: OnceLock<Mutex<Session>> = OnceLock::new();
    let session = session(&S, "isnet-general-use.onnx")?;
    let img = open(src)?;
    let (w, h) = img.dimensions();
    progress(0.2);
    let input = img.resize_exact(1024, 1024, imageops::FilterType::Triangle).to_rgb8();
    let t = tensor(&input, [0.5; 3], [1.0; 3], 255.0)?;
    let mut s = session.lock().unwrap();
    let out = s.run(ort::inputs![t])?;
    let (_, mask) = out[0].try_extract_tensor::<f32>()?;
    progress(0.8);
    let (lo, hi) = mask.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let range = (hi - lo).max(1e-6);
    let small = image::GrayImage::from_fn(1024, 1024, |x, y| image::Luma([(((mask[(y * 1024 + x) as usize] - lo) / range) * 255.0) as u8]));
    let alpha = imageops::resize(&small, w, h, imageops::FilterType::CatmullRom);
    let mut rgba = img.to_rgba8();
    for (p, a) in rgba.pixels_mut().zip(alpha.pixels()) {
        p[3] = ((p[3] as u16 * a[0] as u16) / 255) as u8;
    }
    let out = out_path(src, Some("No background"), "png");
    save(&DynamicImage::ImageRgba8(rgba), &out, "png", 100)?;
    Ok(out)
}

// --- Face blurring (UltraFace) ----------------------------------------------------------------

fn faces(img: &DynamicImage) -> Result<Vec<[f32; 4]>> {
    static S: OnceLock<Mutex<Session>> = OnceLock::new();
    let session = session(&S, "version-RFB-320.onnx")?;
    let input = img.resize_exact(320, 240, imageops::FilterType::Triangle).to_rgb8();
    let t = tensor(&input, [127.0 / 128.0; 3], [1.0; 3], 128.0)?;
    let mut s = session.lock().unwrap();
    let out = s.run(ort::inputs![t])?;
    let (_, scores) = out[0].try_extract_tensor::<f32>()?;
    let (_, boxes) = out[1].try_extract_tensor::<f32>()?;
    let mut found: Vec<([f32; 4], f32)> = Vec::new();
    for i in 0..scores.len() / 2 {
        let score = scores[i * 2 + 1];
        if score > 0.6 {
            found.push(([boxes[i * 4], boxes[i * 4 + 1], boxes[i * 4 + 2], boxes[i * 4 + 3]], score));
        }
    }
    // keep the strongest of overlapping boxes
    found.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut keep: Vec<[f32; 4]> = Vec::new();
    for (b, _) in found {
        if keep.iter().all(|k| iou(k, &b) < 0.3) {
            keep.push(b);
        }
    }
    Ok(keep)
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let ix = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let iy = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let inter = ix * iy;
    inter / ((a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - inter).max(1e-6)
}

/// Pixelates every face found, generously padded.
fn blur_faces(src: &Path) -> Result<PathBuf> {
    let img = open(src)?;
    let found = faces(&img)?;
    if found.is_empty() {
        bail!("no faces found");
    }
    let (w, h) = img.dimensions();
    let mut rgba = img.to_rgba8();
    for b in &found {
        let pad = 0.15 * (b[2] - b[0]).max(b[3] - b[1]);
        let x0 = ((b[0] - pad).max(0.0) * w as f32) as u32;
        let y0 = ((b[1] - pad).max(0.0) * h as f32) as u32;
        let x1 = ((b[2] + pad).min(1.0) * w as f32) as u32;
        let y1 = ((b[3] + pad).min(1.0) * h as f32) as u32;
        pixelate(&mut rgba, x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0));
    }
    let fmt = super::image::keep_fmt(src);
    let out = out_path(src, Some("Faces blurred"), &fmt);
    save(&DynamicImage::ImageRgba8(rgba), &out, &fmt, 92)?;
    note(&format!("{} face{} blurred", found.len(), if found.len() == 1 { "" } else { "s" }));
    Ok(out)
}

fn pixelate(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32) {
    let block = (w.max(h) / 10).max(6);
    for by in (y..y + h).step_by(block as usize) {
        for bx in (x..x + w).step_by(block as usize) {
            let (ex, ey) = ((bx + block).min(x + w).min(img.width()), (by + block).min(y + h).min(img.height()));
            let mut sum = [0u64; 4];
            let mut n = 0u64;
            for yy in by..ey {
                for xx in bx..ex {
                    let p = img.get_pixel(xx, yy);
                    for c in 0..4 {
                        sum[c] += p[c] as u64;
                    }
                    n += 1;
                }
            }
            if n == 0 {
                continue;
            }
            let avg = Rgba(sum.map(|v| (v / n) as u8));
            for yy in by..ey {
                for xx in bx..ex {
                    img.put_pixel(xx, yy, avg);
                }
            }
        }
    }
}

// --- Upscaling (Real-ESRGAN, runs on the GPU through Vulkan) ---------------------------------

fn upscale(src: &Path, scale: u32) -> Result<PathBuf> {
    let exe = security::tool_path("upscale").context("Upscaling isn't installed yet: Settings › Add-ons")?;
    let scale = scale.clamp(2, 4);
    let img = open(src)?;
    // past ~100 megapixels the result can't be opened again (and takes many minutes of GPU time)
    if img.width() as u64 * img.height() as u64 * (scale * scale) as u64 > 100_000_000 {
        bail!("this picture is already too big to enlarge {scale}× (the result would pass 100 megapixels)");
    }
    let png = temp_file("png");
    img.save(&png)?;
    let tmp_out = temp_file("png");
    let o = super::tools::output_within(
        super::tools::quiet(&exe)
            .current_dir(exe.parent().context("no folder")?)
            .arg("-i")
            .arg(&png)
            .arg("-o")
            .arg(&tmp_out)
            .args(["-n", "realesrgan-x4plus", "-s", &scale.to_string()]),
        std::time::Duration::from_secs(1200),
    )?;
    let _ = std::fs::remove_file(&png);
    if !o.status.success() || !tmp_out.is_file() {
        bail!("upscaling failed (needs a graphics card with Vulkan): {}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or(""));
    }
    let fmt = super::image::keep_fmt(src);
    let out = out_path(src, Some(&format!("{scale}x")), &fmt);
    save(&image::open(&tmp_out)?, &out, &fmt, 95)?;
    let _ = std::fs::remove_file(&tmp_out);
    Ok(out)
}
