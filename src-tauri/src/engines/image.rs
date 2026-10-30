use super::{media, pdfwrite, Action, Level, Progress};
use crate::naming::{ext, out_path};
use anyhow::{bail, Context, Result};
use image::{
    codecs::{
        jpeg::JpegEncoder,
        png::{CompressionType, FilterType, PngEncoder},
    },
    imageops, DynamicImage, ImageDecoder, ImageReader, Rgba, RgbaImage,
};
use std::{
    fs,
    io::{BufWriter, Cursor},
    path::{Path, PathBuf},
};

/// Opens an image with EXIF orientation applied; falls back to ffmpeg for formats
/// the `image` crate can't decode (AVIF, HEIC…).
pub fn open(path: &Path) -> Result<DynamicImage> {
    let native = || -> Result<DynamicImage> {
        let mut dec = ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
        let orientation = dec.orientation()?;
        let mut img = DynamicImage::from_decoder(dec)?;
        img.apply_orientation(orientation);
        Ok(img)
    };
    match native() {
        Ok(img) => Ok(img),
        Err(_) => {
            // HEIC, camera RAW…: Windows' own codecs first, then ffmpeg
            #[cfg(windows)]
            if let Ok(png) = crate::winrt::decode_to_png(path) {
                return Ok(image::load_from_memory(&png)?);
            }
            let tmp = media::image_to_png(path).context("can't read this image")?;
            let img = image::open(&tmp)?;
            let _ = fs::remove_file(tmp);
            Ok(img)
        }
    }
}

fn flatten(img: &DynamicImage) -> image::RgbImage {
    let rgba = img.to_rgba8();
    let mut out = image::RgbImage::new(rgba.width(), rgba.height());
    for (o, p) in out.pixels_mut().zip(rgba.pixels()) {
        let a = p[3] as u32;
        for c in 0..3 {
            o[c] = ((p[c] as u32 * a + 255 * (255 - a)) / 255) as u8;
        }
    }
    out
}

pub fn jpeg_bytes(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, quality).encode_image(&flatten(img))?;
    Ok(buf)
}

fn png_bytes(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_with_encoder(PngEncoder::new_with_quality(&mut buf, CompressionType::Best, FilterType::Adaptive))?;
    Ok(buf)
}

/// Writes `img` as `fmt`. `quality` applies to lossy formats.
pub fn save(img: &DynamicImage, out: &Path, fmt: &str, quality: u8) -> Result<()> {
    let bytes = match fmt {
        "jpg" | "jpeg" => jpeg_bytes(img, quality)?,
        "png" => png_bytes(img)?,
        "webp" => {
            let rgba = img.to_rgba8();
            let enc = webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height());
            enc.encode(quality as f32).to_vec()
        }
        "ico" => {
            let small = if img.width() > 256 || img.height() > 256 { img.thumbnail(256, 256) } else { img.clone() };
            let mut buf = Cursor::new(Vec::new());
            small.write_to(&mut buf, image::ImageFormat::Ico)?;
            buf.into_inner()
        }
        "bmp" | "tif" | "tiff" | "gif" => {
            let f = image::ImageFormat::from_extension(fmt).unwrap();
            let mut buf = Cursor::new(Vec::new());
            // BMP/GIF can't hold 16-bit; TIFF is fine with 8-bit too.
            DynamicImage::ImageRgba8(img.to_rgba8()).write_to(&mut buf, f)?;
            buf.into_inner()
        }
        "avif" => {
            // the AVIF encoder here has no transparency: flatten on white like JPG does
            let tmp = crate::naming::temp_file("png");
            fs::write(&tmp, png_bytes(&DynamicImage::ImageRgb8(flatten(img)))?)?;
            let crf = 63 - (quality as u32 * 63 / 100).min(63);
            let r = media::png_to_avif(&tmp, out, crf.clamp(18, 50));
            let _ = fs::remove_file(tmp);
            return r;
        }
        "pdf" => {
            let page = pdfwrite::Page::from_image(img, 90)?;
            return pdfwrite::write(out, &[page]);
        }
        _ => bail!("can't save as {fmt}"),
    };
    let mut w = BufWriter::new(fs::File::create(out)?);
    std::io::Write::write_all(&mut w, &bytes)?;
    Ok(())
}

/// Output format that keeps the source format when we can write it.
pub fn keep_fmt(src: &Path) -> String {
    let e = ext(src);
    match e.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tif" | "tiff" | "gif" | "ico" => e,
        _ => "png".into(),
    }
}

/// GIF, WebP or PNG with more than one frame.
fn animated(src: &Path) -> bool {
    let r = || -> Result<bool> {
        let file = std::io::BufReader::new(fs::File::open(src)?);
        Ok(match ext(src).as_str() {
            "gif" => {
                use image::AnimationDecoder;
                image::codecs::gif::GifDecoder::new(file)?.into_frames().take(2).count() > 1
            }
            "webp" => image::codecs::webp::WebPDecoder::new(file)?.has_animation(),
            "png" | "apng" => image::codecs::png::PngDecoder::new(file)?.is_apng()?,
            _ => false,
        })
    };
    r().unwrap_or(false)
}

pub fn run(src: &Path, action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    progress(0.1);
    // re-encoding would silently keep only the first frame
    if !matches!(action, Action::Convert { .. }) && animated(src) {
        bail!("this is an animated picture; this action would keep only its first frame");
    }
    let fmt = keep_fmt(src);
    let out = match action {
        Action::Convert { to } => {
            let img = open(src)?;
            let out = out_path(src, None, to);
            save(&img, &out, to, 90)?;
            out
        }
        Action::Compress { level } => compress(src, *level)?,
        Action::Resize { value } => {
            let img = open(src)?;
            let (w, h) = (img.width() * value / 100, img.height() * value / 100);
            let small = img.resize_exact(w.max(1), h.max(1), imageops::FilterType::Lanczos3);
            let out = out_path(src, Some(&format!("{value}%")), &fmt);
            save(&small, &out, &fmt, 90)?;
            out
        }
        Action::Rotate { deg } => {
            let img = open(src)?;
            let r = match deg {
                90 => img.rotate90(),
                180 => img.rotate180(),
                270 => img.rotate270(),
                _ => bail!("unsupported angle"),
            };
            let out = out_path(src, Some("Rotated"), &fmt);
            save(&r, &out, &fmt, 92)?;
            out
        }
        Action::StripMetadata => strip(src)?,
        Action::Crop { rect } => {
            let img = open(src)?;
            let (w, h) = (img.width() as f64, img.height() as f64);
            let x = (rect[0].clamp(0.0, 1.0) * w).round() as u32;
            let y = (rect[1].clamp(0.0, 1.0) * h).round() as u32;
            let cw = ((rect[2] * w).round() as u32).clamp(1, img.width() - x.min(img.width() - 1));
            let ch = ((rect[3] * h).round() as u32).clamp(1, img.height() - y.min(img.height() - 1));
            let out = out_path(src, Some("Cropped"), &fmt);
            save(&img.crop_imm(x, y, cw, ch), &out, &fmt, 92)?;
            out
        }
        Action::AddBg { color, padding, radius, shadow } => {
            let img = open(src)?;
            let composed = add_background(&img, parse_hex(color)?, *padding, *radius, *shadow);
            let out = out_path(src, Some("With Background"), "png");
            save(&composed, &out, "png", 100)?;
            out
        }
        Action::Redact { boxes } => {
            let mut img = open(src)?.to_rgba8();
            let (w, h) = (img.width() as f64, img.height() as f64);
            for b in boxes {
                let x0 = (b[0] * w).max(0.0) as u32;
                let y0 = (b[1] * h).max(0.0) as u32;
                let x1 = ((b[0] + b[2]) * w).min(w) as u32;
                let y1 = ((b[1] + b[3]) * h).min(h) as u32;
                for y in y0..y1 {
                    for x in x0..x1 {
                        img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                    }
                }
            }
            let out = out_path(src, Some("Redacted"), &fmt);
            save(&DynamicImage::ImageRgba8(img), &out, &fmt, 92)?;
            out
        }
        _ => bail!("not available for images"),
    };
    progress(1.0);
    Ok(vec![out])
}

fn compress(src: &Path, level: Level) -> Result<PathBuf> {
    let fmt = ext(src);
    let img = open(src)?;
    let max_side = match level {
        Level::Low => u32::MAX,
        Level::Medium => 3840,
        Level::High => 2048,
    };
    let img = if img.width().max(img.height()) > max_side { img.resize(max_side, max_side, imageops::FilterType::Lanczos3) } else { img };
    let (bytes, out_fmt) = match fmt.as_str() {
        "png" => {
            let preset = match level {
                Level::Low => 2,
                Level::Medium => 4,
                Level::High => 6,
            };
            let mut opts = oxipng::Options::from_preset(preset);
            opts.strip = oxipng::StripChunks::Safe;
            (oxipng::optimize_from_memory(&png_bytes(&img)?, &opts)?, "png")
        }
        "webp" => {
            let q = match level {
                Level::Low => 82.0,
                Level::Medium => 70.0,
                Level::High => 55.0,
            };
            let rgba = img.to_rgba8();
            (webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height()).encode(q).to_vec(), "webp")
        }
        _ => {
            let q = match level {
                Level::Low => 82,
                Level::Medium => 70,
                Level::High => 55,
            };
            (jpeg_bytes(&img, q)?, "jpg")
        }
    };
    let out = out_path(src, Some("Compressed"), out_fmt);
    fs::write(&out, bytes)?;
    Ok(out)
}

/// Removes EXIF/XMP/comments. JPEG and PNG are cleaned without re-encoding when possible.
fn strip(src: &Path) -> Result<PathBuf> {
    let fmt = ext(src);
    let data = fs::read(src)?;
    let out = out_path(src, Some("Clean"), &fmt);
    match fmt.as_str() {
        "jpg" | "jpeg" if !jpeg_is_rotated(src) => fs::write(&out, strip_jpeg(&data)?)?,
        "png" => {
            let mut opts = oxipng::Options::from_preset(0);
            opts.strip = oxipng::StripChunks::Safe;
            fs::write(&out, oxipng::optimize_from_memory(&data, &opts)?)?;
        }
        _ => {
            // HEIC, AVIF, RAW… are written as PNG, so the name has to say .png
            let keep = keep_fmt(src);
            let out = out_path(src, Some("Clean"), &keep);
            save(&open(src)?, &out, &keep, 95)?;
            return Ok(out);
        }
    }
    Ok(out)
}

fn jpeg_is_rotated(src: &Path) -> bool {
    (|| -> Result<bool> {
        let mut dec = ImageReader::open(src)?.with_guessed_format()?.into_decoder()?;
        Ok(dec.orientation()? != image::metadata::Orientation::NoTransforms)
    })()
    .unwrap_or(false)
}

/// Drops APP1 (EXIF/XMP), APP12–APP15 and COM segments, keeps ICC (APP2) and the rest.
fn strip_jpeg(d: &[u8]) -> Result<Vec<u8>> {
    if d.len() < 4 || d[0] != 0xFF || d[1] != 0xD8 {
        bail!("not a JPEG");
    }
    let mut out = vec![0xFF, 0xD8];
    let mut i = 2;
    while i + 4 <= d.len() {
        if d[i] != 0xFF {
            bail!("corrupt JPEG");
        }
        let marker = d[i + 1];
        if marker == 0xDA {
            out.extend_from_slice(&d[i..]); // start of scan: copy the rest verbatim
            return Ok(out);
        }
        let len = u16::from_be_bytes([d[i + 2], d[i + 3]]) as usize;
        let end = (i + 2 + len).min(d.len());
        let drop = matches!(marker, 0xE1 | 0xEC..=0xEF | 0xFE);
        if !drop {
            out.extend_from_slice(&d[i..end]);
        }
        i = end;
    }
    bail!("corrupt JPEG")
}

fn parse_hex(s: &str) -> Result<[u8; 3]> {
    let h = s.trim_start_matches('#');
    if h.len() != 6 {
        bail!("bad colour");
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16);
    Ok([p(0)?, p(2)?, p(4)?])
}

/// Places the image on a solid canvas with padding, rounded corners and a soft shadow.
/// `padding` and `radius` are fractions of the image's longer / shorter side; `shadow` is 0–1.
fn add_background(img: &DynamicImage, bg: [u8; 3], padding: f64, radius: f64, shadow: f64) -> DynamicImage {
    let src = img.to_rgba8();
    let (w, h) = src.dimensions();
    let pad = (padding.clamp(0.0, 0.5) * w.max(h) as f64).round() as u32;
    let r = (radius.clamp(0.0, 0.5) * w.min(h) as f64) as f32;
    let (cw, ch) = (w + pad * 2, h + pad * 2);
    let mut canvas = RgbaImage::from_pixel(cw, ch, Rgba([bg[0], bg[1], bg[2], 255]));

    let inside = |x: u32, y: u32| -> f32 {
        // coverage of a rounded-rectangle mask, anti-aliased on the corner arcs
        if r <= 0.0 {
            return 1.0;
        }
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let cx = fx.clamp(r, w as f32 - r);
        let cy = fy.clamp(r, h as f32 - r);
        let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
        (r - d + 0.5).clamp(0.0, 1.0)
    };

    if shadow > 0.0 && pad > 0 {
        // Blur a small copy of the mask (fast), then scale it up.
        let scale = 4u32;
        let (sw, sh) = ((cw / scale).max(1), (ch / scale).max(1));
        let mut m = image::GrayImage::new(sw, sh);
        let off_y = (pad as f32 * 0.25) as u32 / scale;
        for y in 0..sh {
            for x in 0..sw {
                let (ix, iy) = ((x * scale) as i64 - pad as i64, (y * scale) as i64 - pad as i64 - (off_y * scale) as i64);
                if ix >= 0 && iy >= 0 && (ix as u32) < w && (iy as u32) < h {
                    m.put_pixel(x, y, image::Luma([(inside(ix as u32, iy as u32) * 255.0) as u8]));
                }
            }
        }
        let blurred = imageops::blur(&m, (pad as f32 / scale as f32 * 0.45).max(1.0));
        let big = imageops::resize(&blurred, cw, ch, imageops::FilterType::Triangle);
        let strength = shadow.clamp(0.0, 1.0) as f32 * 0.45;
        for (p, s) in canvas.pixels_mut().zip(big.pixels()) {
            let a = s[0] as f32 / 255.0 * strength;
            for c in 0..3 {
                p[c] = (p[c] as f32 * (1.0 - a)) as u8;
            }
        }
    }

    for y in 0..h {
        for x in 0..w {
            let s = src.get_pixel(x, y);
            let a = s[3] as f32 / 255.0 * inside(x, y);
            if a <= 0.0 {
                continue;
            }
            let d = canvas.get_pixel_mut(x + pad, y + pad);
            for c in 0..3 {
                d[c] = (s[c] as f32 * a + d[c] as f32 * (1.0 - a)) as u8;
            }
        }
    }
    DynamicImage::ImageRgba8(canvas)
}
