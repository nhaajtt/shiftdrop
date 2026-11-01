//! End-to-end checks of every action on generated samples. Needs ffmpeg/ffprobe on PATH.

use super::{run, tools, Action, Level};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn workdir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("shiftdrop-tests").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    tools::set_bin_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("bin"));
    d
}

fn ffmpeg(args: &[&str]) {
    let ok = Command::new("ffmpeg").args(["-v", "error", "-y"]).args(args).status().unwrap().success();
    assert!(ok, "sample generation failed: {args:?}");
}

fn check(paths: &[PathBuf], action: Action) -> Vec<PathBuf> {
    let out = run(paths, &action, &|_| {}).unwrap_or_else(|e| panic!("{action:?} failed: {e}"));
    assert!(!out.is_empty(), "{action:?} produced nothing");
    for o in &out {
        let len = std::fs::metadata(o).map(|m| if m.is_dir() { 1 } else { m.len() }).unwrap_or(0);
        assert!(len > 0, "{action:?} wrote an empty file {o:?}");
    }
    out
}

fn conv(to: &str) -> Action {
    Action::Convert { to: to.into() }
}

#[test]
fn images() {
    let d = workdir("images");
    let png = d.join("Photo.png");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=640x400", "-frames:v", "1", png.to_str().unwrap()]);
    let p = vec![png.clone()];
    for to in ["jpg", "webp", "avif", "pdf", "ico", "bmp", "tiff", "gif"] {
        check(&p, conv(to));
    }
    let jpg = vec![d.join("Photo.jpg")];
    check(&jpg, Action::Compress { level: Level::Medium });
    check(&jpg, Action::StripMetadata);
    check(&p, Action::Compress { level: Level::High });
    check(&p, Action::Resize { value: 50 });
    check(&p, Action::Rotate { deg: 90 });
    check(&p, Action::StripMetadata);
    check(&p, Action::Crop { rect: [0.1, 0.1, 0.5, 0.5] });
    check(&p, Action::AddBg { color: "#F3F1EC".into(), padding: 0.1, radius: 0.04, shadow: 0.5 });
    check(&p, Action::Redact { boxes: vec![[0.2, 0.2, 0.3, 0.1]] });
    // AVIF input goes through the ffmpeg fallback decoder
    check(&[d.join("Photo.avif")], conv("png"));
    check(&[png, d.join("Photo.jpg")], Action::Merge);
}

#[test]
fn pdfs() {
    let d = workdir("pdfs");
    let png = d.join("Page.png");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=600x800", "-frames:v", "1", png.to_str().unwrap()]);
    let one = check(&[png.clone(), png.clone()], Action::Merge)[0].clone(); // 2-page PDF
    let p = vec![one.clone()];
    assert_eq!(check(&p, conv("jpg")).len(), 2);
    check(&p, conv("png"));
    check(&p, Action::Compress { level: Level::Medium });
    assert_eq!(check(&p, Action::Split).len(), 2);
    check(&p, Action::Redact { boxes: vec![[0.1, 0.1, 0.2, 0.2]] });
    check(&[one.clone(), one, png], Action::Merge);
}

#[test]
fn media() {
    let d = workdir("media");
    let mp4 = d.join("Clip.mp4");
    let silent = d.join("Silent.mp4");
    let mp3 = d.join("Song.mp3");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=640x360:rate=30:duration=3", "-f", "lavfi", "-i", "sine=f=440:duration=3", "-shortest", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", mp4.to_str().unwrap()]);
    ffmpeg(&["-f", "lavfi", "-i", "testsrc2=size=320x240:rate=25:duration=2", "-c:v", "libx264", "-pix_fmt", "yuv420p", silent.to_str().unwrap()]);
    ffmpeg(&["-f", "lavfi", "-i", "sine=f=330:duration=3", "-c:a", "libmp3lame", mp3.to_str().unwrap()]);
    assert!((tools::duration(&mp4) - 3.0).abs() < 0.2);

    let v = vec![mp4.clone()];
    for to in ["mov", "webm", "mkv", "gif", "avi"] {
        check(&v, conv(to));
    }
    check(&v, Action::Compress { level: Level::High });
    check(&v, Action::Resize { value: 240 });
    check(&v, Action::ExtractAudio { to: "mp3".into() });
    check(&v, Action::Mute);
    check(&v, Action::StripMetadata);
    check(&v, Action::Trim { start: 0.5, end: 2.0 });
    let merged = check(&[mp4, silent], Action::Merge);
    assert!((tools::duration(&merged[0]) - 5.0).abs() < 0.3);

    let a = vec![mp3.clone()];
    for to in ["m4a", "wav", "flac", "ogg", "opus"] {
        check(&a, conv(to));
    }
    check(&a, Action::Compress { level: Level::Low });
    check(&a, Action::Trim { start: 1.0, end: 2.0 });
    check(&a, Action::StripMetadata);
    check(&[mp3.clone(), mp3], Action::Merge);
}

#[test]
fn text_and_files() {
    let d = workdir("text");
    let txt = d.join("Notes.txt");
    std::fs::write(&txt, "xin chào ✓\nline two").unwrap();
    let md = d.join("Read me.md");
    std::fs::write(&md, "# Title\n\n- a\n- b\n\n`code`").unwrap();
    let csv = d.join("Table.csv");
    std::fs::write(&csv, "name,note\r\nAn,\"a, \"\"quoted\"\" note\"\r\nBinh,ok\r\n").unwrap();
    let json = d.join("Data.json");
    std::fs::write(&json, r#"[{"a":1,"b":"x"},{"a":2,"c":null}]"#).unwrap();

    check(std::slice::from_ref(&txt), conv("pdf"));
    check(std::slice::from_ref(&md), conv("html"));
    check(&[md], conv("pdf"));
    let j = check(&[csv], conv("json"));
    assert!(std::fs::read_to_string(&j[0]).unwrap().contains("a, \\\"quoted\\\" note"));
    check(std::slice::from_ref(&json), conv("csv"));
    check(std::slice::from_ref(&json), Action::FormatJson { pretty: true });
    check(std::slice::from_ref(&json), Action::FormatJson { pretty: false });

    let zip = check(&[txt.clone(), json], Action::Zip);
    let out = check(&zip, Action::Extract);
    assert!(out[0].join("Notes.txt").is_file());

    let exe = d.join("notepad.exe");
    std::fs::copy(r"C:\Windows\System32\notepad.exe", &exe).unwrap();
    let info = check(std::slice::from_ref(&exe), Action::PeInfo);
    let report = std::fs::read_to_string(&info[0]).unwrap();
    assert!(report.contains("x64") && report.contains(".dll"), "{report}");
    check(&[exe], Action::Strings);
}

#[test]
fn installed_apps() {
    for a in crate::apps::all() {
        println!("{} -> {}", a.id, a.target.display());
    }
}

#[test]
fn security_and_dev() {
    let d = workdir("security");
    let exe = d.join("invoice.pdf.exe");
    std::fs::copy(r"C:\Windows\System32\notepad.exe", &exe).unwrap();
    assert!(super::security::disguise(&exe).is_some());
    let fake = d.join("photo.jpg");
    std::fs::copy(r"C:\Windows\System32\notepad.exe", &fake).unwrap();
    assert!(super::security::disguise(&fake).unwrap().contains("program"));
    let txt = d.join("plain.txt");
    std::fs::write(&txt, "hello").unwrap();
    assert!(super::security::disguise(&txt).is_none());

    let report = check(std::slice::from_ref(&exe), Action::Hashes { algo: "all".into() });
    let r = std::fs::read_to_string(&report[0]).unwrap();
    assert!(r.contains("imphash") && r.contains("SHA-512"), "{r}");
    assert!(super::security::signature(Path::new(r"C:\Windows\explorer.exe")).starts_with("signed"));
    run(std::slice::from_ref(&txt), &Action::Locks, &|_| {}).unwrap();

    let enc = check(std::slice::from_ref(&txt), Action::Encrypt { password: "s3cret-pass!".into() });
    assert!(run(&enc, &Action::Decrypt { password: "wrong".into() }, &|_| {}).is_err());
    let dec = check(&enc, Action::Decrypt { password: "s3cret-pass!".into() });
    assert_eq!(std::fs::read_to_string(&dec[0]).unwrap(), "hello");

    let gone = d.join("gone.txt");
    std::fs::write(&gone, "secret".repeat(1000)).unwrap();
    run(std::slice::from_ref(&gone), &Action::Shred, &|_| {}).unwrap();
    assert!(!gone.exists());

    let b64 = d.join("data.txt");
    std::fs::write(&b64, "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==").unwrap();
    let png = check(&[b64], Action::Base64 { decode: true });
    assert!(png[0].extension().unwrap() == "png");
    assert_eq!(super::dev::path_as(Path::new(r"C:\Users\me\a b.txt"), "wsl"), "/mnt/c/Users/me/a b.txt");

    let yaml = d.join("conf.yaml");
    std::fs::write(&yaml, "name: demo\nports: [80, 443]\nnested:\n  on: true\n").unwrap();
    let j = check(std::slice::from_ref(&yaml), conv("json"));
    let t = check(&j, conv("toml"));
    assert!(std::fs::read_to_string(&t[0]).unwrap().contains("ports"));
    let xml = d.join("feed.xml");
    std::fs::write(&xml, r#"<feed lang="vi"><item>a</item><item>b</item><empty/></feed>"#).unwrap();
    let x = check(&[xml], conv("json"));
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&x[0]).unwrap()).unwrap();
    assert_eq!(v["feed"]["item"][1], "b");
    assert_eq!(v["feed"]["@lang"], "vi");

    let a = d.join("a.txt");
    let b = d.join("b.txt");
    std::fs::write(&a, "one\ntwo\n").unwrap();
    std::fs::write(&b, "one\nthree\n").unwrap();
    let _ = run(&[a, b], &Action::Diff, &|_| {}).unwrap();
}

#[test]
fn media_extras() {
    let d = workdir("media_extras");
    let mp4 = d.join("Talk.mp4");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=640x360:rate=30:duration=4", "-f", "lavfi", "-i", "sine=f=440:duration=4", "-shortest", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", mp4.to_str().unwrap()]);
    let v = vec![mp4.clone()];
    let small = check(&v, Action::TargetSize { mb: 0.5 });
    assert!(std::fs::metadata(&small[0]).unwrap().len() < 512 * 1024);
    check(&v, Action::Speed { factor: 2.0 });
    check(&v, Action::Speed { factor: 0.25 });
    check(&v, Action::Flip { dir: "h".into() });
    check(&v, Action::Rotate { deg: 90 });
    check(&v, Action::Thumbnail);
    let f = check(&v, Action::Frames { every: 1.0 });
    assert!(std::fs::read_dir(&f[0]).unwrap().count() >= 3);
    check(&v, Action::AudioFix { mode: "denoise".into() });
    check(&v, Action::AudioFix { mode: "normalize".into() });
    let a = d.join("Voice.mp3");
    ffmpeg(&["-f", "lavfi", "-i", "sine=f=330:duration=3", "-c:a", "libmp3lame", a.to_str().unwrap()]);
    check(std::slice::from_ref(&a), Action::Speed { factor: 1.5 });
    check(&[a], Action::TargetSize { mb: 0.02 });
    let png = d.join("Slide.png");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=640x400", "-frames:v", "1", png.to_str().unwrap()]);
    let show = check(&[png.clone(), png], Action::Slideshow { seconds: 1.5 });
    assert!((tools::duration(&show[0]) - 3.0).abs() < 0.5, "{}", tools::duration(&show[0]));
}

#[test]
fn image_extras() {
    let d = workdir("image_extras");
    let txt = d.join("Link.txt");
    std::fs::write(&txt, "https://example.com/xin-chào").unwrap();
    let qr = check(&[txt], Action::QrMake);
    run(&qr, &Action::QrRead, &|_| {}).unwrap();
    assert!(super::take_notes().iter().any(|n| n.contains("example.com")));

    let png = d.join("Logo.png");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=640x400", "-frames:v", "1", png.to_str().unwrap()]);
    let p = vec![png.clone()];
    check(&p, Action::IconSet { web: false });
    let fav = check(&p, Action::IconSet { web: true });
    assert!(fav[0].join("favicon.ico").is_file());
    check(&p, Action::Palette);
    check(&p, Action::Watermark { text: "© Nhaajt 2026".into() });
    check(&[png.clone(), png.clone(), png.clone()], Action::ContactSheet);
    check(&p, Action::Vectorize);

    // text rendered by ffmpeg, read back by Windows OCR
    let words = d.join("Words.png");
    ffmpeg(&["-f", "lavfi", "-i", "color=white:size=900x200", "-frames:v", "1", "-vf", r"drawtext=fontfile='C\:/Windows/Fonts/arial.ttf':text='HELLO OCR 2026':fontsize=72:x=40:y=60", words.to_str().unwrap()]);
    // build servers may have no OCR language installed; on a normal PC this must work
    match super::imagex::ocr_text(&words) {
        Ok(t) => assert!(t.contains("HELLO") && t.contains("2026"), "{t}"),
        Err(e) if std::env::var_os("CI").is_some() => eprintln!("OCR skipped on CI: {e}"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn pdf_extras() {
    let d = workdir("pdf_extras");
    let png = d.join("Page.png");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=600x800", "-frames:v", "1", png.to_str().unwrap()]);
    let pdf = check(&[png.clone(), png.clone(), png.clone()], Action::Merge);
    check(&pdf, Action::Rotate { deg: 90 });
    let edited = check(&pdf, Action::Pages { order: vec![2, 0], rotate: vec![0, 180] });
    assert_eq!(super::pdf::load(&edited[0]).unwrap().pages().len(), 2);
    check(&pdf, Action::PageNumbers);
    check(&pdf, Action::Watermark { text: "BẢN NHÁP".into() });
    let sig = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    check(&pdf, Action::Stamp { page: 0, rect: [0.6, 0.8, 0.3, 0.1], png: sig.into() });
    let imgs = check(&pdf, Action::ExtractImages);
    assert!(std::fs::read_dir(&imgs[0]).unwrap().count() == 3);
    // pages are pictures only, so their text comes from OCR
    let t = check(&pdf, Action::PdfText);
    assert!(std::fs::read_to_string(&t[0]).unwrap().contains("Page 3"));
}

#[test]
fn recipes() {
    let d = workdir("recipes");
    let png = d.join("Shot.png");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=640x400", "-frames:v", "1", png.to_str().unwrap()]);
    let steps = vec![Action::Resize { value: 50 }, conv("jpg"), Action::StripMetadata];
    let out = check(std::slice::from_ref(&png), Action::Chain { steps });
    assert_eq!(out.len(), 1);
    // only the original and the final file are left
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 2, "{out:?}");
    let txt = d.join("note.txt");
    std::fs::write(&txt, "x").unwrap();
    let made = check(&[txt], Action::Custom { command: "copy {file} \"{stem} copy.{ext}\" >nul && echo ok".into() });
    assert!(made[0].ends_with("note copy.txt"), "{made:?}");
}

#[test]
fn docx_to_markdown() {
    let d = workdir("docx");
    let w = d.join("src").join("word");
    std::fs::create_dir_all(&w).unwrap();
    std::fs::write(w.join("document.xml"), r#"<?xml version="1.0"?><w:document xmlns:w="x"><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Báo cáo</w:t></w:r></w:p>
<w:p><w:r><w:t xml:space="preserve">Plain and </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>bold</w:t></w:r></w:p>
<w:p><w:pPr><w:numPr/></w:pPr><w:r><w:t>item one</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr>
<w:tr><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
</w:body></w:document>"#).unwrap();
    let docx = d.join("Report.docx");
    let ok = Command::new(r"C:\Windows\System32\tar.exe").args(["-a", "-c", "-f"]).arg(&docx).arg("-C").arg(d.join("src")).arg("word").status().unwrap().success();
    assert!(ok);
    let md = check(&[docx], conv("md"));
    let t = std::fs::read_to_string(&md[0]).unwrap();
    assert!(t.contains("# Báo cáo") && t.contains("**bold**") && t.contains("- item one") && t.contains("| A | B |"), "{t}");
}

#[test]
fn docx_keeps_literal_marks_and_ignores_tab_stops() {
    let d = workdir("docx-marks");
    let w = d.join("src").join("word");
    std::fs::create_dir_all(&w).unwrap();
    std::fs::write(w.join("document.xml"), r#"<?xml version="1.0"?><w:document xmlns:w="x"><w:body>
<w:p><w:pPr><w:tabs><w:tab w:val="left" w:pos="720"/></w:tabs></w:pPr><w:r><w:t>__init__ and a****b</w:t></w:r></w:p>
<w:p><w:r><w:t>left</w:t><w:tab/><w:t>right</w:t></w:r></w:p>
</w:body></w:document>"#).unwrap();
    let docx = d.join("Marks.docx");
    assert!(Command::new(r"C:\Windows\System32\tar.exe").args(["-a", "-c", "-f"]).arg(&docx).arg("-C").arg(d.join("src")).arg("word").status().unwrap().success());
    let md = check(&[docx], conv("md"));
    let t = std::fs::read_to_string(&md[0]).unwrap();
    assert!(t.starts_with(r"\_\_init\_\_ and a\*\*\*\*b") && t.contains("left\tright"), "{t}");
}

#[test]
fn failed_unit_leaves_no_partial_output() {
    let d = workdir("discard");
    let src = d.join("x.txt");
    std::fs::write(&src, b"original").unwrap();
    crate::naming::begin();
    let out = crate::naming::out_path(&src, Some("Compressed"), "pdf");
    std::fs::write(&out, b"half").unwrap();
    let dir = crate::naming::out_dir(&src, "pages").unwrap();
    std::fs::write(dir.join("p1.png"), b"x").unwrap();
    crate::naming::discard();
    assert!(!out.exists() && !dir.exists() && src.exists());
}

#[test]
fn rename_puts_everything_back_when_one_fails() {
    let d = workdir("rename");
    let (a, b) = (d.join("a.txt"), d.join("b.txt"));
    std::fs::write(&a, b"a").unwrap();
    std::fs::write(&b, b"b").unwrap();
    let s = |p: &Path| p.display().to_string();
    // Windows refuses the second name, so it fails after the first was renamed
    let pairs = vec![(s(&a), s(&d.join("a2.txt"))), (s(&b), s(&d.join("b2<.txt")))];
    assert!(tauri::async_runtime::block_on(crate::shell::rename_files(pairs)).is_err());
    assert!(a.exists() && b.exists() && !d.join("a2.txt").exists());
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 2, "temporary names left behind");
}

/// A solid red PNG as a data URL, for stamping.
fn red_png() -> String {
    use base64::Engine;
    let img = image::RgbaImage::from_pixel(20, 20, image::Rgba([255, 0, 0, 255]));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}

#[test]
fn stamp_goes_where_it_is_shown_on_turned_pages() {
    use pdfium_render::prelude::*;
    let d = workdir("stamp_rotated");
    let png = d.join("Page.png");
    image::RgbImage::from_pixel(600, 800, image::Rgb([255, 255, 255])).save(&png).unwrap();
    let pdf = check(&[png.clone(), png], Action::Merge);
    for deg in [90, 180, 270] {
        let turned = check(&pdf, Action::Rotate { deg });
        // top-left corner of the page as it is shown
        let signed = check(&turned, Action::Stamp { page: 0, rect: [0.0, 0.0, 0.25, 0.25], png: red_png() });
        let doc = super::pdf::load(&signed[0]).unwrap();
        let page = doc.pages().get(0).unwrap();
        let im = page.render_with_config(&PdfRenderConfig::new().set_target_width(400)).unwrap().as_image().unwrap().to_rgb8();
        let (w, h) = (im.width(), im.height());
        let red = |x: u32, y: u32| {
            let p = im.get_pixel(x, y);
            p[0] > 200 && p[1] < 80 && p[2] < 80
        };
        assert!(red(w / 16, h / 16), "{deg}°: stamp not in the top-left corner");
        assert!(!red(w - w / 16, h - h / 16) && !red(w - w / 16, h / 16) && !red(w / 16, h - h / 16), "{deg}°: stamp in the wrong corner");
        drop(page);
        drop(doc);
        let _ = std::fs::remove_file(&signed[0]);
        let _ = std::fs::remove_file(&turned[0]);
    }
}

#[test]
fn wide_gamut_photos_keep_their_colours() {
    use image::{codecs::png::PngEncoder, ImageEncoder};
    let d = workdir("icc");
    let px = image::RgbImage::from_pixel(8, 8, image::Rgb([200, 100, 50]));
    let write = |name: &str, icc: Option<Vec<u8>>| {
        let p = d.join(name);
        let mut enc = PngEncoder::new(std::fs::File::create(&p).unwrap());
        if let Some(icc) = icc {
            enc.set_icc_profile(icc).unwrap();
        }
        enc.write_image(&px, 8, 8, image::ExtendedColorType::Rgb8).unwrap();
        p
    };
    let p3 = write("p3.png", Some(moxcms::ColorProfile::new_display_p3().encode().unwrap()));
    let srgb = write("srgb.png", Some(moxcms::ColorProfile::new_srgb().encode().unwrap()));
    let plain = write("plain.png", None);
    let red = |p: &PathBuf| {
        let out = check(std::slice::from_ref(p), conv("bmp"));
        image::open(&out[0]).unwrap().to_rgb8().get_pixel(4, 4)[0]
    };
    // Display P3 red 200 is a deeper red than sRGB can write as 200
    assert!(red(&p3) >= 210, "P3 picture wasn't converted");
    assert!(red(&srgb).abs_diff(200) <= 1 && red(&plain) == 200, "an sRGB picture was changed");
}

#[test]
fn json_keeps_long_numbers_and_key_order() {
    let d = workdir("json");
    let src = d.join("data.json");
    std::fs::write(&src, "{\"zeta\": 1, \"id\": 12345678901234567890123, \"pi\": 3.14159265358979323846,\n \"empty\": [ ], \"s\": \"a \\\" ] } , :\", \"alpha\": {}}").unwrap();
    let pretty = std::fs::read_to_string(&check(std::slice::from_ref(&src), Action::FormatJson { pretty: true })[0]).unwrap();
    assert!(pretty.contains("12345678901234567890123") && pretty.contains("3.14159265358979323846"), "{pretty}");
    assert!(pretty.find("zeta") < pretty.find("alpha"), "keys were reordered: {pretty}");
    assert!(pretty.contains("\"empty\": []") && pretty.contains("\"s\": \"a \\\" ] } , :\""), "{pretty}");
    let min = std::fs::read_to_string(&check(std::slice::from_ref(&src), Action::FormatJson { pretty: false })[0]).unwrap();
    assert_eq!(min, "{\"zeta\":1,\"id\":12345678901234567890123,\"pi\":3.14159265358979323846,\"empty\":[],\"s\":\"a \\\" ] } , :\",\"alpha\":{}}");
    let yaml = std::fs::read_to_string(&check(std::slice::from_ref(&src), conv("yaml"))[0]).unwrap();
    assert!(yaml.contains("12345678901234567890123") && yaml.find("zeta") < yaml.find("alpha"), "{yaml}");
}

#[test]
fn quitting_mid_job_removes_what_it_was_writing() {
    let d = workdir("quit");
    let src = d.join("clip.mp4");
    std::fs::write(&src, b"original").unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let src2 = src.clone();
    // a job still writing its output when the app quits
    let worker = std::thread::spawn(move || {
        crate::naming::begin();
        let out = crate::naming::out_path(&src2, None, "webm");
        std::fs::write(&out, b"half").unwrap();
        tx.send(out).unwrap();
        let _ = go_rx.recv();
    });
    let out = rx.recv().unwrap();
    // a job that already finished keeps its file
    crate::naming::begin();
    let kept = crate::naming::out_path(&src, Some("Compressed"), "mp4");
    std::fs::write(&kept, b"done").unwrap();
    crate::naming::finish();
    crate::naming::discard_running();
    assert!(!out.exists(), "half-written output left behind");
    assert!(kept.exists() && src.exists());
    go_tx.send(()).unwrap();
    worker.join().unwrap();
}

#[test]
fn xml_too_deep_or_cut_off_is_refused() {
    let deep = "<a>".repeat(150) + &"</a>".repeat(150);
    assert!(super::text::xml_to_value(&deep).unwrap_err().to_string().contains("nested"));
    assert!(super::text::xml_to_value("<a><b>x</b>").unwrap_err().to_string().contains("cut off"));
    assert_eq!(super::text::xml_to_value("<a><b>x</b></a>").unwrap()["a"]["b"], "x");
}

#[test]
fn toml_dates_become_plain_text() {
    let v = super::text::toml_to_json(toml::from_str("d = 1979-05-27T07:32:00Z\n[t]\nday = 2024-01-02\n").unwrap());
    assert_eq!(v["d"], "1979-05-27T07:32:00Z");
    assert_eq!(v["t"]["day"], "2024-01-02");
}

#[test]
fn markdown_and_html_cant_navigate_when_printed() {
    let md = super::text::markdown("hi <meta http-equiv=\"refresh\" content=\"0;url=file:///C:/x\">\n\n<iframe src=\"file:///C:/x\"></iframe>\n");
    assert!(!md.contains("<meta") && !md.contains("<iframe") && md.contains("&lt;meta"), "{md}");
    let html = super::text::lock_html("<!DOCTYPE html><html><head><META content=\"0;url=file:///C:/a>b\" HTTP-EQUIV=refresh><meta charset=utf-8></head><body>x</body></html>", "file:///C:/d/");
    assert!(html.starts_with("<!DOCTYPE html><meta http-equiv=\"Content-Security-Policy\""), "{html}");
    assert!(!html.to_lowercase().contains("refresh") && html.contains("<meta charset=utf-8>") && html.contains("<body>x</body>"), "{html}");
}

#[test]
fn jpeg_strip_cuts_after_the_picture_and_keeps_adobe() {
    let mut j = vec![0xFF, 0xD8];
    j.extend([0xFF, 0xE1, 0x00, 0x08]);
    j.extend(b"Exif\0\0");
    j.extend([0xFF, 0xEE, 0x00, 0x07]);
    j.extend(b"Adobe");
    j.extend([0xFF, 0xDA, 0x00, 0x04, 0x01, 0x02]);
    j.extend([0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56]); // data with stuffing and a restart
    j.extend([0xFF, 0xD9]);
    j.extend([0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x08]); // a second picture appended by the phone
    j.extend(b"Exif\0\0GPS");
    let out = super::image::strip_jpeg(&j).unwrap();
    assert!(out.ends_with(&[0x56, 0xFF, 0xD9]), "{out:02X?}");
    assert!(out.windows(5).any(|w| w == b"Adobe"));
    assert!(!out.windows(4).any(|w| w == b"Exif"));
}

#[test]
fn bad_colours_and_huge_images_are_refused() {
    assert!(super::image::parse_hex("#aééa").is_err());
    assert!(super::image::parse_hex("zzzzzz").is_err());
    assert_eq!(super::image::parse_hex("#FF8000").unwrap(), [255, 128, 0]);
    let d = workdir("caps");
    let png = d.join("small.png");
    image::RgbImage::new(640, 400).save(&png).unwrap();
    let e = run(&[png], &Action::Resize { value: 5000 }, &|_| {}).unwrap_err();
    assert!(e.to_string().contains("too big"), "{e}");
}

#[test]
fn base64_without_padding_decodes() {
    assert_eq!(super::dev::decode_base64("aGVsbG8").unwrap(), b"hello");
    assert_eq!(super::dev::decode_base64("aGVsbG8=").unwrap(), b"hello");
    assert_eq!(super::dev::decode_base64("data:text/plain;base64,aGk_").unwrap(), b"hi?");
    assert!(super::dev::decode_base64("not base64!").is_err());
}

#[test]
fn docx_breaks_in_cells_and_empty_text() {
    let md = super::docx::convert(r#"<w:document xmlns:w="x"><w:body>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>a</w:t><w:br/><w:t>b</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>c</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:t/></w:r><w:r><w:instrText>HIDDEN</w:instrText></w:r><w:r><w:t>shown</w:t></w:r></w:p>
</w:body></w:document>"#)
    .unwrap();
    assert!(md.contains("| a<br>b | c |") && !md.contains("HIDDEN") && md.contains("shown"), "{md}");
}

#[test]
fn encrypt_names_and_dash_names_zip() {
    let d = workdir("names");
    let txt = d.join("note.txt");
    std::fs::write(&txt, "hello").unwrap();
    let enc = check(std::slice::from_ref(&txt), Action::Encrypt { password: "s3cret-pass!".into() });
    assert_eq!(enc[0].file_name().unwrap(), "note.txt.age");
    assert_eq!(super::files::tar_member("-rf.txt"), "./-rf.txt");
    assert_eq!(super::files::tar_member("@list"), "./@list");
    let dash = d.join("-dash.txt");
    std::fs::write(&dash, "x").unwrap();
    let z = check(&[dash], Action::Zip);
    assert_eq!(z[0].file_name().unwrap(), "-dash.zip");
}

#[test]
fn media_past_the_end_is_an_error() {
    let d = workdir("media_end");
    let mp4 = d.join("50% clip.mp4");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=320x240:rate=30:duration=2", "-c:v", "libx264", "-pix_fmt", "yuv420p", mp4.to_str().unwrap()]);
    let v = vec![mp4.clone()];
    assert!(run(&v, &Action::Trim { start: 10.0, end: 12.0 }, &|_| {}).is_err());
    let f = check(&v, Action::Frames { every: 1.0 });
    assert!(std::fs::read_dir(&f[0]).unwrap().count() >= 1);
    // mixed formats in one slideshow
    let png = d.join("a.png");
    let jpg = d.join("b.jpg");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=640x400", "-frames:v", "1", png.to_str().unwrap()]);
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=300x500", "-frames:v", "1", jpg.to_str().unwrap()]);
    let show = check(&[png, jpg], Action::Slideshow { seconds: 1.0 });
    assert!((tools::duration(&show[0]) - 2.0).abs() < 0.5, "{}", tools::duration(&show[0]));
}
