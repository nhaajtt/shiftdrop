use super::{note, tools, Action, Level, Progress};
use crate::naming::{ext, kind, out_dir, out_path, temp_file, Kind};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

const EVEN: &str = "scale=trunc(iw/2)*2:trunc(ih/2)*2";

/// Encoder arguments for a target container/format.
fn codec_args(to: &str) -> Result<Vec<String>> {
    Ok(match to {
        "mp4" | "m4v" | "mov" | "mkv" => s(&[
            "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p", "-vf", EVEN,
            "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart",
        ]),
        "webm" => s(&[
            "-c:v", "libvpx-vp9", "-crf", "31", "-b:v", "0", "-row-mt", "1", "-deadline", "good", "-cpu-used", "2",
            "-c:a", "libopus", "-b:a", "128k",
        ]),
        "avi" => s(&["-c:v", "mpeg4", "-q:v", "3", "-c:a", "libmp3lame", "-q:a", "2"]),
        "gif" => s(&[
            "-an",
            "-vf",
            "fps=12,scale='min(720,iw)':-2:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4",
            "-loop",
            "0",
        ]),
        "mp3" => s(&["-vn", "-c:a", "libmp3lame", "-q:a", "2"]),
        "m4a" | "aac" => s(&["-vn", "-c:a", "aac", "-b:a", "192k"]),
        "wav" => s(&["-vn", "-c:a", "pcm_s16le"]),
        "flac" => s(&["-vn", "-c:a", "flac"]),
        "ogg" => s(&["-vn", "-c:a", "libvorbis", "-q:a", "5"]),
        "opus" => s(&["-vn", "-c:a", "libopus", "-b:a", "128k"]),
        _ => bail!("can't convert to {to}"),
    })
}

/// Keeps the source container when it is a common one, otherwise `fallback`.
fn same_or(src: &Path, fallback: &str) -> String {
    let e = ext(src);
    match kind(src) {
        Kind::Video if matches!(e.as_str(), "mp4" | "mov" | "mkv" | "webm" | "m4v") => e,
        Kind::Audio if matches!(e.as_str(), "mp3" | "wav" | "m4a" | "flac" | "ogg" | "opus") => e,
        _ => fallback.into(),
    }
}

fn with_vf(mut c: Vec<String>, vf: String) -> Vec<String> {
    match c.iter().position(|x| x == "-vf") {
        Some(i) => c[i + 1] = vf,
        None => c.extend(["-vf".to_string(), vf]),
    }
    c
}

/// atempo only takes 0.5–2, so bigger changes are chained.
fn atempo(mut f: f64) -> String {
    let mut parts = Vec::new();
    while f > 2.0 {
        parts.push("atempo=2.0".to_string());
        f /= 2.0;
    }
    while f < 0.5 {
        parts.push("atempo=0.5".to_string());
        f /= 0.5;
    }
    parts.push(format!("atempo={f:.4}"));
    parts.join(",")
}

fn trim_num(f: f64) -> String {
    let s = format!("{f:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn run(src: &Path, action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    match action {
        Action::TargetSize { mb } => return target_size(src, *mb, progress),
        Action::Frames { every } => return frames(src, *every, progress),
        Action::Transcribe { burn } => return transcribe(src, *burn, progress),
        _ => {}
    }
    let is_video = kind(src) == Kind::Video;
    let src_s = src.to_string_lossy().into_owned();
    let mut total = tools::duration(src);
    let mut args = s(&["-i", &src_s]);

    let out = match action {
        Action::Convert { to } => {
            args.extend(codec_args(to)?);
            out_path(src, None, to)
        }
        Action::ExtractAudio { to } => {
            args.extend(codec_args(to)?);
            out_path(src, Some("Audio"), to)
        }
        Action::Compress { level } if is_video => {
            let (crf, ab, max_h) = match level {
                Level::Low => ("24", "160k", None),
                Level::Medium => ("28", "128k", Some(1080)),
                Level::High => ("32", "96k", Some(720)),
            };
            let vf = match max_h {
                Some(h) => format!("scale=-2:'min(ih,{h})',{EVEN}"),
                None => EVEN.into(),
            };
            args.extend(s(&[
                "-c:v", "libx264", "-preset", "medium", "-crf", crf, "-pix_fmt", "yuv420p", "-vf", &vf,
                "-c:a", "aac", "-b:a", ab, "-movflags", "+faststart",
            ]));
            out_path(src, Some("Compressed"), "mp4")
        }
        Action::Compress { level } => {
            let br = match level {
                Level::Low => "192k",
                Level::Medium => "128k",
                Level::High => "96k",
            };
            args.extend(s(&["-vn", "-c:a", "libmp3lame", "-b:a", br]));
            out_path(src, Some("Compressed"), "mp3")
        }
        Action::Resize { value } if is_video => {
            let to = same_or(src, "mp4");
            args.extend(with_vf(codec_args(&to)?, format!("scale=-2:'min(ih,{value})',{EVEN}")));
            out_path(src, Some(&format!("{value}p")), &to)
        }
        Action::Mute if is_video => {
            args.extend(s(&["-map", "0:v", "-c", "copy", "-an"]));
            out_path(src, Some("Muted"), &ext(src))
        }
        Action::StripMetadata => {
            args.extend(s(&["-map", "0", "-map_metadata", "-1", "-map_chapters", "-1", "-c", "copy"]));
            out_path(src, Some("Clean"), &ext(src))
        }
        Action::Trim { start, end } => {
            if end <= start {
                bail!("the end must be after the start");
            }
            total = end - start;
            let to = same_or(src, if is_video { "mp4" } else { "mp3" });
            args = s(&["-ss", &format!("{start:.3}"), "-i", &src_s, "-t", &format!("{total:.3}")]);
            args.extend(codec_args(&to)?);
            out_path(src, Some("Trimmed"), &to)
        }
        Action::Speed { factor } => {
            if !(0.1..=10.0).contains(factor) {
                bail!("speed must be between 0.1× and 10×");
            }
            total /= factor;
            let to = same_or(src, if is_video { "mp4" } else { "mp3" });
            let mut c = codec_args(&to)?;
            if is_video {
                c = with_vf(c, format!("setpts=PTS/{factor},{EVEN}"));
            }
            if !is_video || tools::has_audio(src) {
                c.extend(s(&["-af", &atempo(*factor)]));
            }
            args.extend(c);
            out_path(src, Some(&format!("{}x", trim_num(*factor))), &to)
        }
        Action::Flip { dir } if is_video => {
            let to = same_or(src, "mp4");
            let f = if dir == "v" { "vflip" } else { "hflip" };
            args.extend(with_vf(codec_args(&to)?, format!("{f},{EVEN}")));
            out_path(src, Some("Flipped"), &to)
        }
        Action::Rotate { deg } if is_video => {
            let to = same_or(src, "mp4");
            let f = match deg {
                90 => "transpose=1",
                270 => "transpose=2",
                _ => "hflip,vflip",
            };
            args.extend(with_vf(codec_args(&to)?, format!("{f},{EVEN}")));
            out_path(src, Some("Rotated"), &to)
        }
        Action::Thumbnail if is_video => {
            args = s(&["-ss", &format!("{:.2}", total * 0.1), "-i", &src_s, "-frames:v", "1", "-q:v", "2"]);
            total = 0.0;
            out_path(src, Some("Thumbnail"), "jpg")
        }
        Action::AudioFix { mode } => {
            let af = if mode == "denoise" { "afftdn=nf=-25,highpass=f=80" } else { "loudnorm=I=-16:TP=-1.5:LRA=11" };
            let to = same_or(src, if is_video { "mp4" } else { "mp3" });
            let c = codec_args(&to)?;
            if is_video {
                // keep the picture as it is, re-encode only the sound
                args.extend(s(&["-c:v", "copy"]));
                let i = c.iter().position(|x| x == "-c:a").unwrap_or(c.len());
                args.extend(c[i..].iter().cloned());
            } else {
                args.extend(c);
            }
            args.extend(s(&["-af", af]));
            out_path(src, Some(if mode == "denoise" { "Clean audio" } else { "Normalized" }), &to)
        }
        _ => bail!("not available for this file"),
    };
    args.push(out.to_string_lossy().into_owned());
    tools::ffmpeg(&args, total, progress)?;
    Ok(vec![out])
}

/// Two-pass encode at the bitrate that lands just under `mb` megabytes.
fn target_size(src: &Path, mb: f64, progress: Progress) -> Result<Vec<PathBuf>> {
    let dur = tools::duration(src);
    if dur <= 0.0 {
        bail!("couldn't read the length");
    }
    let budget_kbit = mb * 8.0 * 1024.0 * 0.95; // 5% for container overhead
    let src_s = src.to_string_lossy().into_owned();
    let label = format!("{} MB", trim_num(mb));
    if kind(src) == Kind::Audio {
        let kbps = (budget_kbit / dur).clamp(16.0, 320.0) as u32;
        let out = out_path(src, Some(&label), "mp3");
        let args = s(&["-i", &src_s, "-vn", "-c:a", "libmp3lame", "-b:a", &format!("{kbps}k"), &out.to_string_lossy()]);
        tools::ffmpeg(&args, dur, progress)?;
        return Ok(vec![out]);
    }
    let audio = if tools::has_audio(src) { (budget_kbit / dur * 0.12).clamp(32.0, 128.0) } else { 0.0 };
    let video = budget_kbit / dur - audio;
    if video < 50.0 {
        bail!("{label} is too small for a video this long");
    }
    // lower the resolution when the bitrate can't carry it
    let h = if video < 400.0 { 480 } else if video < 1200.0 { 720 } else { 1080 };
    let vf = format!("scale=-2:'min(ih,{h})',{EVEN}");
    let v = format!("{}k", video as u32);
    let log = temp_file("log");
    let log_s = log.to_string_lossy().into_owned();
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let pass1 = s(&["-i", &src_s, "-c:v", "libx264", "-preset", "medium", "-b:v", &v, "-vf", &vf, "-pass", "1", "-passlogfile", &log_s, "-an", "-f", "null", null]);
    tools::ffmpeg(&pass1, dur, &|p| progress(p * 0.5))?;
    let out = out_path(src, Some(&label), "mp4");
    let mut pass2 = s(&["-i", &src_s, "-c:v", "libx264", "-preset", "medium", "-b:v", &v, "-vf", &vf, "-pass", "2", "-passlogfile", &log_s, "-pix_fmt", "yuv420p"]);
    if audio > 0.0 {
        pass2.extend(s(&["-c:a", "aac", "-b:a", &format!("{}k", audio as u32)]));
    } else {
        pass2.push("-an".into());
    }
    pass2.extend(s(&["-movflags", "+faststart", &out.to_string_lossy()]));
    let r = tools::ffmpeg(&pass2, dur, &|p| progress(0.5 + p * 0.5));
    for e in ["-0.log", "-0.log.mbtree"] {
        let _ = std::fs::remove_file(format!("{log_s}{e}"));
    }
    r?;
    let size = std::fs::metadata(&out)?.len() as f64 / 1048576.0;
    note(&format!("{size:.1} MB (limit {label})"));
    Ok(vec![out])
}

fn frames(src: &Path, every: f64, progress: Progress) -> Result<Vec<PathBuf>> {
    let dir = out_dir(src, "frames")?;
    let pattern = dir.join("frame %04d.png");
    let args = s(&["-i", &src.to_string_lossy(), "-vf", &format!("fps=1/{every}"), &pattern.to_string_lossy()]);
    tools::ffmpeg(&args, tools::duration(src), progress)?;
    Ok(vec![dir])
}

/// concat-demuxer line for a path ('…' quoting, single quotes escaped).
fn concat_line(p: &Path) -> String {
    format!("file '{}'\n", p.to_string_lossy().replace('\'', "'\\''"))
}

/// Images → MP4, each shown for `seconds`, letterboxed to the first image's size.
pub fn slideshow(paths: &[PathBuf], seconds: f64, progress: Progress) -> Result<Vec<PathBuf>> {
    let list = temp_file("txt");
    let mut txt = String::new();
    for p in paths {
        txt += &concat_line(p);
        txt += &format!("duration {seconds}\n");
    }
    // the demuxer only honours the last duration when the file is listed again
    txt += &concat_line(paths.last().context("no images")?);
    std::fs::write(&list, txt)?;
    let (w, h) = image::image_dimensions(&paths[0]).unwrap_or((1920, 1080));
    let (w, h) = (w.min(3840) / 2 * 2, h.min(2160) / 2 * 2);
    let out = out_path(&paths[0], Some("Slideshow"), "mp4");
    let vf = format!("scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p");
    let args = s(&[
        "-f", "concat", "-safe", "0", "-i", &list.to_string_lossy(), "-vf", &vf, "-t", &format!("{:.3}", seconds * paths.len() as f64),
        "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-movflags", "+faststart", &out.to_string_lossy(),
    ]);
    let r = tools::ffmpeg(&args, seconds * paths.len() as f64, progress);
    let _ = std::fs::remove_file(&list);
    r?;
    Ok(vec![out])
}

/// Speech to subtitles with whisper.cpp (downloaded in Settings). Optionally burnt into the video.
fn transcribe(src: &Path, burn: bool, progress: Progress) -> Result<Vec<PathBuf>> {
    let cli = super::security::tool_path("whisper").context("Speech to text isn't installed yet: Settings › Speech to text")?;
    let model = super::security::whisper_model().context("the speech model is missing: Settings › Speech to text")?;
    let dur = tools::duration(src);
    let wav = temp_file("wav");
    tools::ffmpeg(&s(&["-i", &src.to_string_lossy(), "-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", &wav.to_string_lossy()]), dur, &|p| progress(p * 0.1))?;
    let base = temp_file("sub");
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8).to_string();
    let o = tools::quiet(&cli)
        .arg("-m")
        .arg(&model)
        .args(["-l", "auto", "-t", &threads, "-osrt", "-of"])
        .arg(&base)
        .arg("-f")
        .arg(&wav)
        .output()?;
    let _ = std::fs::remove_file(&wav);
    let srt_tmp = PathBuf::from(format!("{}.srt", base.display()));
    if !o.status.success() || !srt_tmp.is_file() {
        bail!("speech recognition failed: {}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or(""));
    }
    progress(0.7);
    let srt = out_path(src, None, "srt");
    std::fs::copy(&srt_tmp, &srt)?;
    if !burn || kind(src) != Kind::Video {
        let _ = std::fs::remove_file(&srt_tmp);
        return Ok(vec![srt]);
    }
    // subtitles= can't take a drive letter easily, so ffmpeg runs from the temp folder
    let dir = srt_tmp.parent().context("no folder")?.to_path_buf();
    let rel = srt_tmp.file_name().context("no name")?.to_string_lossy().into_owned();
    let out = out_path(src, Some("Subtitled"), "mp4");
    let mut args = s(&["-i", &src.to_string_lossy()]);
    args.extend(with_vf(codec_args("mp4")?, format!("subtitles={rel}:force_style='FontName=Segoe UI,FontSize=20,Outline=1.2,Shadow=0',{EVEN}")));
    args.push(out.to_string_lossy().into_owned());
    let r = tools::ffmpeg_in(Some(&dir), &args, dur, &|p| progress(0.7 + p * 0.3));
    let _ = std::fs::remove_file(&srt_tmp);
    r?;
    Ok(vec![srt, out])
}

pub fn merge(paths: &[PathBuf], progress: Progress) -> Result<Vec<PathBuf>> {
    let video = kind(&paths[0]) == Kind::Video;
    let n = paths.len();
    let durs: Vec<f64> = paths.iter().map(|p| tools::duration(p)).collect();
    let total: f64 = durs.iter().sum();
    let mut args = Vec::new();
    for p in paths {
        args.extend(s(&["-i", &p.to_string_lossy()]));
    }
    let mut filter = String::new();

    let out = if video {
        let (w, h) = tools::video_size(&paths[0]).unwrap_or((1920, 1080));
        let (w, h) = (w / 2 * 2, h / 2 * 2);
        let mut next_input = n; // silent inputs are appended after the real files
        for (i, p) in paths.iter().enumerate() {
            filter += &format!(
                "[{i}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p[v{i}];"
            );
            let a = if tools::has_audio(p) {
                format!("{i}:a")
            } else {
                args.extend(s(&["-f", "lavfi", "-t", &format!("{:.3}", durs[i].max(0.1)), "-i", "anullsrc=r=48000:cl=stereo"]));
                next_input += 1;
                format!("{}:a", next_input - 1)
            };
            filter += &format!("[{a}]aresample=48000,aformat=channel_layouts=stereo[a{i}];");
        }
        for i in 0..n {
            filter += &format!("[v{i}][a{i}]");
        }
        filter += &format!("concat=n={n}:v=1:a=1[v][a]");
        args.extend(s(&["-filter_complex", &filter, "-map", "[v]", "-map", "[a]"]));
        args.extend(s(&[
            "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart",
        ]));
        out_path(&paths[0], Some("Merged"), "mp4")
    } else {
        for i in 0..n {
            filter += &format!("[{i}:a]aresample=48000,aformat=channel_layouts=stereo[a{i}];");
        }
        for i in 0..n {
            filter += &format!("[a{i}]");
        }
        filter += &format!("concat=n={n}:v=0:a=1[a]");
        let to = same_or(&paths[0], "mp3");
        args.extend(s(&["-filter_complex", &filter, "-map", "[a]"]));
        args.extend(codec_args(&to)?);
        out_path(&paths[0], Some("Merged"), &to)
    };
    args.push(out.to_string_lossy().into_owned());
    tools::ffmpeg(&args, total, progress)?;
    Ok(vec![out])
}

/// Converts any still image ffmpeg can read into a temporary PNG.
pub fn image_to_png(src: &Path) -> Result<PathBuf> {
    let tmp = crate::naming::temp_file("png");
    let args = s(&["-i", &src.to_string_lossy(), "-frames:v", "1", &tmp.to_string_lossy()]);
    tools::ffmpeg(&args, 0.0, &|_| {})?;
    Ok(tmp)
}

/// Encodes a PNG into AVIF with ffmpeg (libaom).
pub fn png_to_avif(png: &Path, out: &Path, crf: u32) -> Result<()> {
    let args = s(&[
        "-i", &png.to_string_lossy(), "-c:v", "libaom-av1", "-still-picture", "1", "-crf", &crf.to_string(),
        "-b:v", "0", "-cpu-used", "6", &out.to_string_lossy(),
    ]);
    tools::ffmpeg(&args, 0.0, &|_| {})
}
