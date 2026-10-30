//! Actions that work on any file: zip, extract, SHA-256, copy, strings, and a PE (exe/dll) report.

use super::{Action, Progress};
use crate::naming::{out_path, stem, unique};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

/// Actions over the whole selection at once (one job unit).
pub fn is_group(action: &Action) -> bool {
    matches!(action, Action::Zip | Action::Hash | Action::CopyPath | Action::CopyText)
}

pub fn run(paths: &[PathBuf], action: &Action, progress: Progress) -> Result<Vec<PathBuf>> {
    match action {
        Action::Zip => zip(paths).map(|p| vec![p]),
        Action::Hash => {
            let mut lines = Vec::new();
            for (i, p) in paths.iter().enumerate() {
                let h = sha256(p)?;
                lines.push(if paths.len() > 1 { format!("{h}  {}", name(p)) } else { h });
                progress((i + 1) as f32 / paths.len() as f32);
            }
            crate::clipboard::set(&lines.join("\r\n"))?;
            Ok(vec![])
        }
        Action::CopyPath => {
            crate::clipboard::set(&paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\r\n"))?;
            Ok(vec![])
        }
        Action::CopyText => {
            let mut text = String::new();
            for p in paths {
                text += &String::from_utf8_lossy(&std::fs::read(p)?);
            }
            crate::clipboard::set(&text)?;
            Ok(vec![])
        }
        Action::Extract => extract(&paths[0]).map(|p| vec![p]),
        Action::Strings => strings(&paths[0]).map(|p| vec![p]),
        Action::PeInfo => pe_report(&paths[0]).map(|p| vec![p]),
        _ => bail!("not available for this file"),
    }
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Windows' own bsdtar (zip, tar, gz, xz, bz2, 7z, rar); Git's tar on PATH can't write zips.
fn tar() -> Command {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let mut cmd = Command::new(Path::new(&root).join("System32").join("tar.exe"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(super::tools::CREATE_NO_WINDOW);
    }
    cmd
}

fn zip(paths: &[PathBuf]) -> Result<PathBuf> {
    let first = &paths[0];
    let dir = first.parent().context("no folder")?;
    let base = if paths.len() == 1 {
        if first.is_dir() { name(first) } else { stem(first) }
    } else {
        dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Archive".into())
    };
    let out = unique(dir, &base, "zip");
    let mut cmd = tar();
    cmd.arg("-a").arg("-c").arg("-f").arg(&out);
    for p in paths {
        cmd.arg("-C").arg(p.parent().context("no folder")?).arg(name(p));
    }
    let o = super::tools::output_within(&mut cmd, std::time::Duration::from_secs(1800))?;
    if !o.status.success() {
        bail!("zip failed: {}", String::from_utf8_lossy(&o.stderr).trim());
    }
    Ok(out)
}

fn extract(src: &Path) -> Result<PathBuf> {
    let dir = src.parent().context("no folder")?;
    let mut base = stem(src);
    if base.to_lowercase().ends_with(".tar") {
        base.truncate(base.len() - 4);
    }
    let mut out = dir.join(&base);
    let mut n = 2;
    while out.exists() {
        out = dir.join(format!("{base} ({n})"));
        n += 1;
    }
    std::fs::create_dir(&out)?;
    if let Err(e) = untar(src, &out) {
        let _ = std::fs::remove_dir_all(&out);
        return Err(e);
    }
    Ok(out)
}

/// Extracts any archive Windows' tar understands into `dir`.
pub fn untar(src: &Path, dir: &Path) -> Result<()> {
    let o = super::tools::output_within(tar().arg("-x").arg("-f").arg(src).arg("-C").arg(dir), std::time::Duration::from_secs(1800))?;
    if !o.status.success() {
        bail!("can't extract: {}", String::from_utf8_lossy(&o.stderr).trim());
    }
    Ok(())
}

/// Extracts just one member of an archive (a .docx's document.xml), whatever else is packed in it.
pub fn untar_one(src: &Path, dir: &Path, member: &str) -> Result<()> {
    let o = super::tools::output_within(tar().arg("-x").arg("-f").arg(src).arg("-C").arg(dir).arg(member), std::time::Duration::from_secs(120))?;
    if !o.status.success() {
        bail!("can't extract: {}", String::from_utf8_lossy(&o.stderr).trim());
    }
    Ok(())
}

pub fn sha256(p: &Path) -> Result<String> {
    let mut f = std::fs::File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Printable ASCII and UTF-16LE runs of 5+ characters, like `strings -n 5`.
fn strings(src: &Path) -> Result<PathBuf> {
    let data = std::fs::read(src)?;
    let printable = |b: u8| (0x20..0x7f).contains(&b) || b == b'\t';
    let mut out = String::new();
    let mut run = String::new();
    for &b in &data {
        if printable(b) {
            run.push(b as char);
        } else {
            if run.len() >= 5 {
                out += &run;
                out.push('\n');
            }
            run.clear();
        }
    }
    out += "\n--- UTF-16 ---\n";
    run.clear();
    for c in data.as_chunks::<2>().0 {
        if c[1] == 0 && printable(c[0]) {
            run.push(c[0] as char);
        } else {
            if run.len() >= 5 {
                out += &run;
                out.push('\n');
            }
            run.clear();
        }
    }
    let o = out_path(src, Some("strings"), "txt");
    std::fs::write(&o, out)?;
    Ok(o)
}

pub struct Pe {
    pub is64: bool,
    machine: u16,
    timestamp: u32,
    subsystem: u16,
    entry: u32,
    image_base: u64,
    dotnet: bool,
    sections: Vec<(String, u32, u32, u32, f64)>, // name, rva, virtual size, raw size, entropy
    imports: Vec<String>,
    /// "dll.function" pairs (lowercase, extension dropped) as used by imphash
    pub functions: Vec<String>,
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn entropy(d: &[u8]) -> f64 {
    if d.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    d.iter().for_each(|&b| counts[b as usize] += 1);
    let n = d.len() as f64;
    counts.iter().filter(|&&c| c > 0).map(|&c| -(c as f64 / n) * (c as f64 / n).log2()).sum()
}

/// Reads just enough of a PE header to tell 32/64-bit (used to pick x32dbg or x64dbg).
pub fn pe_is_64(p: &Path) -> Option<bool> {
    let mut head = vec![0u8; 4096];
    let n = std::fs::File::open(p).ok()?.read(&mut head).ok()?;
    head.truncate(n);
    let pe = u32_at(&head, 0x3c)? as usize;
    (head.get(pe..pe + 4)? == b"PE\0\0").then_some(())?;
    Some(u16_at(&head, pe + 24)? == 0x20b)
}

pub fn parse_pe(d: &[u8]) -> Option<Pe> {
    (d.get(0..2)? == b"MZ").then_some(())?;
    let pe = u32_at(d, 0x3c)? as usize;
    (d.get(pe..pe + 4)? == b"PE\0\0").then_some(())?;
    let coff = pe + 4;
    let nsec = u16_at(d, coff + 2)? as usize;
    let opt = coff + 20;
    let opt_size = u16_at(d, coff + 16)? as usize;
    let is64 = u16_at(d, opt)? == 0x20b;
    let dirs = opt + if is64 { 112 } else { 96 };
    let ndirs = u32_at(d, opt + if is64 { 108 } else { 92 })? as usize;
    let dir = |i: usize| if i < ndirs { Some((u32_at(d, dirs + i * 8)?, u32_at(d, dirs + i * 8 + 4)?)) } else { None };
    let mut sections = Vec::new();
    let sec0 = opt + opt_size;
    for i in 0..nsec {
        let s = sec0 + i * 40;
        let raw_name = d.get(s..s + 8)?;
        let name = String::from_utf8_lossy(raw_name).trim_end_matches('\0').to_string();
        let (vsize, rva, rsize, rptr) = (u32_at(d, s + 8)?, u32_at(d, s + 12)?, u32_at(d, s + 16)?, u32_at(d, s + 20)? as usize);
        let body = d.get(rptr..(rptr + rsize as usize).min(d.len())).unwrap_or(&[]);
        sections.push((name, rva, vsize, rsize, entropy(body)));
    }
    let to_off = |rva: u32| {
        sections.iter().zip(0..).find(|((_, va, vs, rs, _), _)| rva >= *va && rva < va + (*vs).max(*rs)).and_then(|(_, i)| {
            let s = sec0 + i * 40;
            Some((rva - u32_at(d, s + 12)? + u32_at(d, s + 20)?) as usize)
        })
    };
    let mut imports = Vec::new();
    let mut functions = Vec::new();
    let cstr = |o: usize| d.get(o..).map(|t| String::from_utf8_lossy(&t[..t.iter().position(|&b| b == 0).unwrap_or(0)]).into_owned());
    if let Some((rva, _)) = dir(1).filter(|(r, s)| *r != 0 && *s != 0) {
        if let Some(mut o) = to_off(rva) {
            while let Some(name_rva) = u32_at(d, o + 12).filter(|&r| r != 0) {
                if let Some(dll) = to_off(name_rva).and_then(cstr) {
                    let lib = dll.to_lowercase();
                    let lib = lib.rsplit_once('.').filter(|(_, e)| ["dll", "ocx", "sys"].contains(e)).map(|(b, _)| b.to_string()).unwrap_or(lib);
                    // walk the lookup table (or the address table when there is none)
                    let thunk = u32_at(d, o).filter(|&r| r != 0).or_else(|| u32_at(d, o + 16));
                    if let Some(mut t) = thunk.and_then(to_off) {
                        let w = if is64 { 8 } else { 4 };
                        for _ in 0..4096 {
                            let v = if is64 { d.get(t..t + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())) } else { u32_at(d, t).map(u64::from) };
                            let Some(v) = v.filter(|&v| v != 0) else { break };
                            let ordinal = if is64 { v >> 63 == 1 } else { v >> 31 == 1 };
                            let f = if ordinal { format!("ord{}", v & 0xffff) } else { to_off(v as u32 & 0x7fff_ffff).and_then(|n| cstr(n + 2)).unwrap_or_default().to_lowercase() };
                            functions.push(format!("{lib}.{f}"));
                            t += w;
                        }
                    }
                    imports.push(dll);
                }
                o += 20;
                if imports.len() >= 512 {
                    break;
                }
            }
        }
    }
    Some(Pe {
        is64,
        machine: u16_at(d, coff)?,
        timestamp: u32_at(d, coff + 4)?,
        subsystem: u16_at(d, opt + 68)?,
        entry: u32_at(d, opt + 16)?,
        image_base: if is64 { u32_at(d, opt + 24)? as u64 | (u32_at(d, opt + 28)? as u64) << 32 } else { u32_at(d, opt + 28)? as u64 },
        dotnet: dir(14).is_some_and(|(r, _)| r != 0),
        sections,
        imports,
        functions,
    })
}

/// Mandiant's import hash: MD5 of the comma-joined "dll.function" list. Groups malware families.
pub fn imphash(p: &Path) -> Option<String> {
    use md5::Digest;
    let pe = parse_pe(&std::fs::read(p).ok()?)?;
    if pe.functions.is_empty() {
        return None;
    }
    Some(md5::Md5::digest(pe.functions.join(",").as_bytes()).iter().map(|b| format!("{b:02x}")).collect())
}

fn pe_report(src: &Path) -> Result<PathBuf> {
    let data = std::fs::read(src)?;
    let pe = parse_pe(&data).context("not a Windows executable (PE)")?;
    let machine = match pe.machine {
        0x14c => "x86 (32-bit)",
        0x8664 => "x64",
        0xaa64 => "ARM64",
        0x1c4 => "ARM",
        _ => "unknown",
    };
    let subsystem = match pe.subsystem {
        2 => "GUI",
        3 => "Console",
        1 => "Native / driver",
        10..=13 => "EFI",
        _ => "other",
    };
    let mut r = format!(
        "File        {}\nSize        {} bytes\nSHA-256     {}\nMachine     {machine}\nFormat      {}\nSubsystem   {subsystem}\nTimestamp   {} (unix)\nImage base  0x{:X}\nEntry RVA   0x{:X}\n.NET        {}\nSignature   {}\n\nSections\n",
        name(src),
        data.len(),
        sha256(src)?,
        if pe.is64 { "PE32+" } else { "PE32" },
        pe.timestamp,
        pe.image_base,
        pe.entry,
        if pe.dotnet { "yes (managed code: use dnSpy / ILSpy)" } else { "no" },
        super::security::signature(src),
    );
    for (n, rva, vs, rs, e) in &pe.sections {
        let note = if *e > 7.2 { "  high entropy: packed or encrypted?" } else { "" };
        r += &format!("  {n:<8} rva 0x{rva:08X}  vsize {vs:>9}  raw {rs:>9}  entropy {e:.2}{note}\n");
    }
    if let Some(ih) = imphash(src) {
        r += &format!("\nimphash     {ih}\n");
    }
    if let Some(w) = super::security::disguise(src) {
        r += &format!("WARNING     {w}\n");
    }
    r += "\nImports\n";
    for i in &pe.imports {
        r += &format!("  {i}\n");
    }
    let o = out_path(src, Some("info"), "txt");
    std::fs::write(&o, r)?;
    Ok(o)
}
