//! Minimal PDF writer: one JPEG image per page. Used for images → PDF and for
//! flattened PDFs (compress, redact), where every page becomes a picture.

use anyhow::Result;
use image::DynamicImage;
use std::{fs, path::Path};

pub struct Page {
    pub jpeg: Vec<u8>,
    pub px: (u32, u32),
    /// Page size in PDF points (1/72 inch).
    pub pt: (f64, f64),
}

impl Page {
    /// Page sized at 96 dpi, the size the image has on screen at 100 %.
    pub fn from_image(img: &DynamicImage, quality: u8) -> Result<Self> {
        let px = (img.width(), img.height());
        Ok(Self { jpeg: super::image::jpeg_bytes(img, quality)?, px, pt: (px.0 as f64 * 0.75, px.1 as f64 * 0.75) })
    }
}

pub fn write(out: &Path, pages: &[Page]) -> Result<()> {
    let mut buf: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    let n = pages.len();
    // objects: 1 catalog, 2 pages, then per page: page, image, content
    let obj = |buf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    };
    obj(&mut buf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i * 3)).collect();
    obj(&mut buf, &mut offsets, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes());

    for (i, p) in pages.iter().enumerate() {
        let (page_id, img_id, content_id) = (3 + i * 3, 4 + i * 3, 5 + i * 3);
        let (w, h) = p.pt;
        obj(
            &mut buf,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w:.2} {h:.2}] /Resources << /XObject << /Im0 {img_id} 0 R >> >> /Contents {content_id} 0 R >>"
            )
            .as_bytes(),
        );
        debug_assert_eq!(offsets.len(), page_id);
        let mut img = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            p.px.0,
            p.px.1,
            p.jpeg.len()
        )
        .into_bytes();
        img.extend_from_slice(&p.jpeg);
        img.extend_from_slice(b"\nendstream");
        obj(&mut buf, &mut offsets, &img);
        let draw = format!("q {w:.2} 0 0 {h:.2} 0 0 cm /Im0 Do Q");
        obj(&mut buf, &mut offsets, format!("<< /Length {} >>\nstream\n{draw}\nendstream", draw.len()).as_bytes());
    }

    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for o in &offsets {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).as_bytes());
    fs::write(out, buf)?;
    Ok(())
}
