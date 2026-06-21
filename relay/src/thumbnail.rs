use crate::error::RelayError;
use crate::types::{FileType, MediaMeta};
use image::GenericImageView;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;

const THUMB_SIZE: u32 = 220;
const RAR_MAGIC: &[u8] = b"Rar!\x1a\x07";

pub(crate) fn contains_rarjpeg(bytes: &[u8]) -> bool {
    matches!(
        FileType::detect(bytes),
        Some(FileType::Jpeg | FileType::Png)
    ) && bytes.windows(RAR_MAGIC.len()).any(|w| w == RAR_MAGIC)
}

pub(crate) fn validate(data: &[u8]) -> Result<FileType, RelayError> {
    let ft = FileType::detect(data)
        .ok_or_else(|| RelayError::SponsorBuild("unsupported media format".into()))?;
    if contains_rarjpeg(data) {
        return Err(RelayError::SponsorBuild("rarjpeg rejected".into()));
    }
    Ok(ft)
}

fn to_image(data: &[u8], path: &Path, ft: FileType) -> Result<image::DynamicImage, RelayError> {
    match ft {
        FileType::Jpeg | FileType::Png | FileType::WebP => image::load_from_memory(data)
            .map_err(|e| RelayError::SponsorBuild(format!("image decode: {e}"))),
        FileType::Mp4 | FileType::WebM => extract_frame(path),
    }
}

fn extract_frame(path: &Path) -> Result<image::DynamicImage, RelayError> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            path.to_str().unwrap(),
            "-vframes",
            "1",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "pipe:1",
        ])
        .output()
        .map_err(|e| RelayError::SponsorBuild(format!("ffmpeg: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RelayError::SponsorBuild(format!("ffmpeg: {stderr}")));
    }

    image::load_from_memory(&output.stdout)
        .map_err(|e| RelayError::SponsorBuild(format!("frame decode: {e}")))
}

pub(crate) fn generate(data: &[u8], path: &Path) -> Result<Vec<u8>, RelayError> {
    let ft = validate(data)?;
    let img = to_image(data, path, ft)?;
    let (w, h) = img.dimensions();
    let thumb = if w <= THUMB_SIZE && h <= THUMB_SIZE {
        img
    } else {
        img.thumbnail(THUMB_SIZE, THUMB_SIZE)
    };
    let mut buf = Vec::new();
    thumb
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg)
        .map_err(|e| RelayError::SponsorBuild(format!("jpeg encode: {e}")))?;
    Ok(buf)
}

fn probe_duration(path: &Path) -> Option<u64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path.to_str()?,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let secs: f64 = String::from_utf8_lossy(&output.stdout).trim().parse().ok()?;
    Some((secs * 1000.0).round() as u64)
}

pub(crate) fn compute_meta(data: &[u8], path: &Path) -> Result<MediaMeta, RelayError> {
    let ft = validate(data)?;
    let size = data.len() as u64;
    match ft {
        FileType::Jpeg | FileType::Png | FileType::WebP => {
            let img = image::load_from_memory(data)
                .map_err(|e| RelayError::SponsorBuild(format!("image decode: {e}")))?;
            let (w, h) = img.dimensions();
            Ok(MediaMeta {
                mime: ft.mime().to_string(),
                width: w,
                height: h,
                duration_ms: None,
                size,
            })
        }
        FileType::Mp4 | FileType::WebM => {
            let img = extract_frame(path)?;
            let (w, h) = img.dimensions();
            Ok(MediaMeta {
                mime: ft.mime().to_string(),
                width: w,
                height: h,
                duration_ms: probe_duration(path),
                size,
            })
        }
    }
}
