//! One bounded, local refinement of a compact popup from an existing owner frame.
//! No capture, network, input, credentials or persistent OCR output.
use image::RgbaImage;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    process::{Command, Stdio},
};
const UNAVAILABLE: &str = "browser_native_login_refinement_unavailable";
fn bounded_process(deadline: &str, program: &str) -> Command {
    let mut command = Command::new("/usr/bin/timeout");
    command
        .env_clear()
        .env("LANG", "C.UTF-8")
        .args(["--kill-after=0.25s", deadline, program])
        .stderr(Stdio::null());
    command
}
pub(crate) fn lines(image: &RgbaImage) -> Result<Vec<Value>, String> {
    if image.width() > 960 || image.height() > 360 {
        return Err(UNAVAILABLE.into());
    }
    let model_path = "/usr/share/tesseract-ocr/5/tessdata/eng.traineddata";
    if std::fs::metadata(model_path)
        .map_err(|_| UNAVAILABLE)?
        .len()
        > 32 * 1024 * 1024
    {
        return Err(UNAVAILABLE.into());
    }
    let model = std::fs::read(model_path).map_err(|_| UNAVAILABLE)?;
    if format!("{:x}", Sha256::digest(&model))
        != "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    {
        return Err(UNAVAILABLE.into());
    }
    let version = bounded_process("1s", "/usr/bin/tesseract")
        .arg("--version")
        .stderr(Stdio::null())
        .output()
        .map_err(|_| UNAVAILABLE)?;
    if !version.status.success() || !version.stdout.starts_with(b"tesseract 5.3.4\n") {
        return Err(UNAVAILABLE.into());
    }
    let mut png = Cursor::new(vec![]);
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| UNAVAILABLE)?;
    let mut child = bounded_process("2s", "/usr/bin/tesseract")
        .args([
            "stdin",
            "stdout",
            "--tessdata-dir",
            "/usr/share/tesseract-ocr/5/tessdata",
            "-l",
            "eng",
            "--psm",
            "11",
            "tsv",
        ])
        .env("OMP_THREAD_LIMIT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| UNAVAILABLE)?;
    let written = child
        .stdin
        .take()
        .ok_or(UNAVAILABLE)?
        .write_all(png.get_ref());
    let mut bytes = vec![];
    let read = child
        .stdout
        .take()
        .ok_or(UNAVAILABLE)?
        .take(1_048_577)
        .read_to_end(&mut bytes);
    let status = child.wait().map_err(|_| UNAVAILABLE)?;
    if written.is_err() || read.is_err() || !status.success() || bytes.len() > 1_048_576 {
        return Err(UNAVAILABLE.into());
    }
    parse_tsv(
        std::str::from_utf8(&bytes).map_err(|_| UNAVAILABLE)?,
        image.width(),
        image.height(),
    )
}
fn parse_tsv(raw: &str, width: u32, height: u32) -> Result<Vec<Value>, String> {
    type Word = (String, f64, u32, u32, u32, u32);
    let mut groups: BTreeMap<(u32, u32, u32, u32), Vec<Word>> = BTreeMap::new();
    let mut count = 0;
    for (i, line) in raw.lines().enumerate() {
        if i == 0 {
            if line!="level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext"{return Err(UNAVAILABLE.into());}
            continue;
        }
        let f = line.splitn(12, '\t').collect::<Vec<_>>();
        if f.len() != 12 {
            return Err(UNAVAILABLE.into());
        }
        if f[0] != "5" || f[11].trim().is_empty() {
            continue;
        }
        count += 1;
        if count > 4096 {
            return Err(UNAVAILABLE.into());
        }
        let n = |i: usize| f[i].parse::<u32>().map_err(|_| UNAVAILABLE.to_string());
        let (x, y, w, h) = (n(6)?, n(7)?, n(8)?, n(9)?);
        let confidence = f[10].parse::<f64>().map_err(|_| UNAVAILABLE)?;
        if !confidence.is_finite()
            || !(0.0..=100.0).contains(&confidence)
            || w == 0
            || h == 0
            || x.checked_add(w).is_none_or(|r| r > width)
            || y.checked_add(h).is_none_or(|b| b > height)
            || f[11].len() > 512
            || f[11].chars().any(char::is_control)
        {
            return Err(UNAVAILABLE.into());
        }
        groups
            .entry((n(1)?, n(2)?, n(3)?, n(4)?))
            .or_default()
            .push((f[11].into(), confidence, x, y, w, h));
    }
    if groups.len() > 512 {
        return Err(UNAVAILABLE.into());
    }
    Ok(groups
        .values()
        .map(|words| {
            let x = words.iter().map(|w| w.2).min().unwrap();
            let y = words.iter().map(|w| w.3).min().unwrap();
            let right = words.iter().map(|w| w.2 + w.4).max().unwrap();
            let bottom = words.iter().map(|w| w.3 + w.5).max().unwrap();
            json!({"text":words.iter().map(|w|w.0.as_str()).collect::<Vec<_>>().join(" "),
            "confidence":words.iter().map(|w|w.1).fold(100.0,f64::min),
            "bounds":{"x":x,"y":y,"width":right-x,"height":bottom-y}})
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER:&str="level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n";
    #[test]
    fn compact_ocr_parser_preserves_minimum_confidence_and_original_word_bounds() {
        let raw=format!("{HEADER}5\t1\t1\t1\t1\t1\t4\t8\t12\t9\t95\tManage\n5\t1\t1\t1\t1\t2\t20\t8\t20\t9\t83\tpasswords...\n");
        let r = parse_tsv(&raw, 100, 40).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["confidence"], 83.0);
        assert_eq!(r[0]["bounds"], json!({"x":4,"y":8,"width":36,"height":9}));
    }
    #[test]
    fn compact_ocr_parser_refuses_unbounded_malformed_and_nonfinite_evidence() {
        for tail in [
            "5\t1\t1\t1\t1\t1\t99\t8\t12\t9\t95\taccount",
            "5\t1\t1\t1\t1\t1\t4\t8\t12\t9\tNaN\taccount",
            "5\t1\t1\t1\t1\t1\t4\t8\t12\t9\t95\taccount\tprivate",
        ] {
            assert!(parse_tsv(&format!("{HEADER}{tail}\n"), 100, 40).is_err());
        }
        assert!(parse_tsv("wrong schema", 100, 40).is_err());
    }
    #[test]
    #[cfg(unix)]
    fn compact_ocr_timeout_kills_a_child_that_ignores_term() {
        use std::os::unix::process::ExitStatusExt;
        // Even the broken timeout finishes after this finite sleep. A missing
        // hard-kill grace therefore fails the assertion without orphaning work.
        let status = bounded_process("0.05s", "/bin/sh")
            .args(["-c", "trap '' TERM; exec sleep 1"])
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(9));
    }
}
