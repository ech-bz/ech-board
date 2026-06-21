use crate::error::RelayError;
use crate::types::FileType;
use image::GenericImageView;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;

const THUMB_SIZE: u32 = 220;
const RAR_MAGIC: &[u8] = b"Rar!\x1a\x07";

pub(crate) fn contains_rarjpeg(bytes: &[u8]) -> bool {
    matches!(FileType::detect(bytes), Some(FileType::Jpeg | FileType::Png))
        && bytes.windows(RAR_MAGIC.len()).any(|w| w == RAR_MAGIC)
}

pub(crate) fn validate(data: &[u8]) -> Result<FileType, RelayError> {
    let ft = FileType::detect(data).ok_or_else(|| {
        RelayError::SponsorBuild("unsupported media format".into())
    })?;
    if contains_rarjpeg(data) {
        return Err(RelayError::SponsorBuild("rarjpeg rejected".into()));
    }
    Ok(ft)
}

fn to_image(data: &[u8], path: &Path, ft: FileType) -> Result<image::DynamicImage, RelayError> {
    match ft {
        FileType::Jpeg | FileType::Png | FileType::WebP => {
            image::load_from_memory(data).map_err(|e| {
                RelayError::SponsorBuild(format!("image decode: {e}"))
            })
        }
        FileType::Mp4 | FileType::WebM => extract_frame(path),
    }
}

fn extract_frame(path: &Path) -> Result<image::DynamicImage, RelayError> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            path.to_str().unwrap(),
            "-vframes", "1",
            "-f", "image2pipe",
            "-vcodec", "png",
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
