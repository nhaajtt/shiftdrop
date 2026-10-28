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

    check(&[txt.clone()], conv("pdf"));
    check(&[md.clone()], conv("html"));
    check(&[md], conv("pdf"));
    let j = check(&[csv], conv("json"));
    assert!(std::fs::read_to_string(&j[0]).unwrap().contains("a, \\\"quoted\\\" note"));
    check(&[json.clone()], conv("csv"));
    check(&[json.clone()], Action::FormatJson { pretty: true });
    check(&[json.clone()], Action::FormatJson { pretty: false });

    let zip = check(&[txt.clone(), json], Action::Zip);
    let out = check(&zip, Action::Extract);
    assert!(out[0].join("Notes.txt").is_file());

    let exe = d.join("notepad.exe");
    std::fs::copy(r"C:\Windows\System32\notepad.exe", &exe).unwrap();
    let info = check(&[exe.clone()], Action::PeInfo);
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

    let report = check(&[exe.clone()], Action::Hashes { algo: "all".into() });
    let r = std::fs::read_to_string(&report[0]).unwrap();
    assert!(r.contains("imphash") && r.contains("SHA-512"), "{r}");
    assert!(super::security::signature(Path::new(r"C:\Windows\explorer.exe")).starts_with("signed"));
    run(&[txt.clone()], &Action::Locks, &|_| {}).unwrap();

    let enc = check(&[txt.clone()], Action::Encrypt { password: "s3cret!".into() });
    assert!(run(&enc, &Action::Decrypt { password: "wrong".into() }, &|_| {}).is_err());
    let dec = check(&enc, Action::Decrypt { password: "s3cret!".into() });
    assert_eq!(std::fs::read_to_string(&dec[0]).unwrap(), "hello");

    let gone = d.join("gone.txt");
    std::fs::write(&gone, "secret".repeat(1000)).unwrap();
    run(&[gone.clone()], &Action::Shred, &|_| {}).unwrap();
    assert!(!gone.exists());

    let b64 = d.join("data.txt");
    std::fs::write(&b64, "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==").unwrap();
    let png = check(&[b64], Action::Base64 { decode: true });
    assert!(png[0].extension().unwrap() == "png");
    assert_eq!(super::dev::path_as(Path::new(r"C:\Users\me\a b.txt"), "wsl"), "/mnt/c/Users/me/a b.txt");

    let yaml = d.join("conf.yaml");
    std::fs::write(&yaml, "name: demo\nports: [80, 443]\nnested:\n  on: true\n").unwrap();
    let j = check(&[yaml.clone()], conv("json"));
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
    check(&[a.clone()], Action::Speed { factor: 1.5 });
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
    let out = check(&[png.clone()], Action::Chain { steps });
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
