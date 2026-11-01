//! Windows' own imaging and text recognition: reads HEIC, camera RAW and anything else with a
//! codec installed, and recognises text with the OCR built into Windows (works offline).

use anyhow::{Context, Result};
use std::path::Path;
use windows::{
    core::HSTRING,
    Globalization::Language,
    Graphics::Imaging::{BitmapAlphaMode, BitmapDecoder, BitmapEncoder, BitmapPixelFormat, BitmapTransform, ColorManagementMode, ExifOrientationMode, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::{
        FileAccessMode, StorageFile,
        Streams::{DataReader, InMemoryRandomAccessStream},
    },
    Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
};

fn init() {
    // already initialised (or STA) is fine: the calls below work either way
    let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
}

fn bitmap(path: &Path) -> Result<SoftwareBitmap> {
    init();
    let full = std::fs::canonicalize(path)?;
    let full = full.to_string_lossy();
    // canonical paths start with \\?\ (\\?\UNC\ on network shares), which WinRT won't take
    let full = match full.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => full.trim_start_matches(r"\\?\").to_string(),
    };
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(full))?.join()?;
    let stream = file.OpenAsync(FileAccessMode::Read)?.join()?;
    let dec = BitmapDecoder::CreateAsync(&stream)?.join().context("Windows has no codec for this image")?;
    // upright and in sRGB: iPhone HEIC photos are Display P3 and would look washed out otherwise
    Ok(dec
        .GetSoftwareBitmapTransformedAsync(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Straight, &BitmapTransform::new()?, ExifOrientationMode::RespectExifOrientation, ColorManagementMode::ColorManageToSRgb)?
        .join()?)
}

/// Decodes with Windows' codecs (HEIC, RAW, JXR…) and returns PNG bytes.
pub fn decode_to_png(path: &Path) -> Result<Vec<u8>> {
    within(path, decode_to_png_now)
}

/// Runs a Windows imaging call on its own thread with a time limit: a file that is only in
/// OneDrive or a codec that hangs must not hold up the job queue for good.
fn within<T: Send + 'static>(path: &Path, f: fn(&Path) -> Result<T>) -> Result<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let _ = tx.send(f(&path));
    });
    rx.recv_timeout(std::time::Duration::from_secs(120)).unwrap_or_else(|_| Err(anyhow::anyhow!("Windows took too long to read this picture")))
}

fn decode_to_png_now(path: &Path) -> Result<Vec<u8>> {
    let bmp = bitmap(path)?;
    let bmp = SoftwareBitmap::Convert(&bmp, BitmapPixelFormat::Bgra8)?;
    let mem = InMemoryRandomAccessStream::new()?;
    let enc = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &mem)?.join()?;
    enc.SetSoftwareBitmap(&bmp)?;
    enc.FlushAsync()?.join()?;
    let size = mem.Size()? as u32;
    let reader = DataReader::CreateDataReader(&mem.GetInputStreamAt(0)?)?;
    reader.LoadAsync(size)?.join()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(buf)
}

/// Text in the image, line by line. Uses the user's Windows languages, English as a fallback.
pub fn ocr(path: &Path) -> Result<String> {
    within(path, ocr_now)
}

fn ocr_now(path: &Path) -> Result<String> {
    let bmp = bitmap(path)?;
    let bmp = SoftwareBitmap::Convert(&bmp, BitmapPixelFormat::Bgra8)?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .or_else(|_| OcrEngine::TryCreateFromLanguage(&Language::CreateLanguage(&HSTRING::from("en-US"))?))
        .context("no OCR language is installed in Windows")?;
    let result = engine.RecognizeAsync(&bmp)?.join()?;
    let mut lines = Vec::new();
    for line in result.Lines()? {
        lines.push(line.Text()?.to_string());
    }
    Ok(lines.join("\n"))
}

/// Largest side OCR accepts.
pub fn ocr_max_side() -> u32 {
    OcrEngine::MaxImageDimension().unwrap_or(4096)
}
