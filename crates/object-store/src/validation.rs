//! Media validation for imported assets.
//!
//! Validates image decodability and dimensions, attempts video metadata extraction
//! via ffprobe, and rejects common executable/malicious signatures.

use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;

/// Validation result returned for a media object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaInfo {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub detected_format: String,
    pub declared_media_type: String,
}

/// Reasons a media object can be rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaValidationFailure {
    Empty,
    MalwareDetected,
    UnsupportedMediaType,
    DecodeFailed,
    MetadataMissing,
    IoError(String),
    Timeout,
}

impl std::fmt::Display for MediaValidationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty payload"),
            Self::MalwareDetected => write!(f, "malware signature detected"),
            Self::UnsupportedMediaType => write!(f, "unsupported or mismatched media type"),
            Self::DecodeFailed => write!(f, "media decode failed"),
            Self::MetadataMissing => write!(f, "metadata extraction failed"),
            Self::IoError(s) => write!(f, "io error: {s}"),
            Self::Timeout => write!(f, "media validation timed out"),
        }
    }
}

impl std::error::Error for MediaValidationFailure {}

/// Port for media validation.
#[async_trait::async_trait]
pub trait MediaValidator: Send + Sync {
    async fn validate(
        &self,
        bytes: &[u8],
        declared_media_type: &str,
    ) -> Result<MediaInfo, MediaValidationFailure>;
}

/// Default validator that decodes images, probes videos, and scans for malware signatures.
#[derive(Debug, Clone, Default)]
pub struct DefaultMediaValidator;

#[async_trait::async_trait]
impl MediaValidator for DefaultMediaValidator {
    async fn validate(
        &self,
        bytes: &[u8],
        declared_media_type: &str,
    ) -> Result<MediaInfo, MediaValidationFailure> {
        if bytes.is_empty() {
            return Err(MediaValidationFailure::Empty);
        }

        scan_malware_signatures(bytes)?;

        if declared_media_type.starts_with("image/") {
            validate_image(bytes, declared_media_type)
        } else if declared_media_type.starts_with("video/") {
            validate_video(bytes, declared_media_type).await
        } else if declared_media_type.starts_with("application/")
            || declared_media_type == "text/plain"
            || declared_media_type == "text/csv"
        {
            // Non-media assets are not decoded but must still pass malware scan.
            Ok(MediaInfo {
                width: None,
                height: None,
                duration_seconds: None,
                detected_format: "unknown".to_string(),
                declared_media_type: declared_media_type.to_string(),
            })
        } else {
            Err(MediaValidationFailure::UnsupportedMediaType)
        }
    }
}

fn scan_malware_signatures(bytes: &[u8]) -> Result<(), MediaValidationFailure> {
    let signatures: &[&[u8]] = &[
        b"MZ",      // Windows executables
        b"\x7fELF", // ELF binaries
        b"<?php",   // PHP
        b"<?=",
        b"<script",
        b"%PDF",             // PDFs may embed scripts; reject as media
        b"PK\x03\x04",       // ZIP/JAR
        b"\xca\xfe\xba\xbe", // Java class
        b"\xd0\xcf\x11\xe0", // MS Office legacy (OLE)
    ];
    for sig in signatures {
        if bytes.len() >= sig.len() && bytes.starts_with(sig) {
            return Err(MediaValidationFailure::MalwareDetected);
        }
    }
    // Reject HTML/JS payloads disguised as media by looking for JS event handlers.
    let prefix = std::str::from_utf8(&bytes[..bytes.len().min(512)]).unwrap_or("");
    let prefix_lower = prefix.to_lowercase();
    if prefix_lower.contains("javascript:") || prefix_lower.contains("<script") {
        return Err(MediaValidationFailure::MalwareDetected);
    }
    Ok(())
}

fn validate_image(bytes: &[u8], declared: &str) -> Result<MediaInfo, MediaValidationFailure> {
    let format = image::guess_format(bytes).map_err(|_| MediaValidationFailure::DecodeFailed)?;
    let dynamic =
        image::load_from_memory(bytes).map_err(|_| MediaValidationFailure::DecodeFailed)?;
    let (width, height) = dynamic.dimensions();

    let detected = format_to_media_type(format);
    if !media_types_match(declared, &detected) {
        return Err(MediaValidationFailure::UnsupportedMediaType);
    }

    Ok(MediaInfo {
        width: Some(width),
        height: Some(height),
        duration_seconds: None,
        detected_format: format!("{:?}", format),
        declared_media_type: declared.to_string(),
    })
}

fn format_to_media_type(format: image::ImageFormat) -> String {
    use image::ImageFormat;
    match format {
        ImageFormat::Png => "image/png".to_string(),
        ImageFormat::Jpeg => "image/jpeg".to_string(),
        ImageFormat::Gif => "image/gif".to_string(),
        ImageFormat::WebP => "image/webp".to_string(),
        ImageFormat::Bmp => "image/bmp".to_string(),
        ImageFormat::Tiff => "image/tiff".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

const FFPROBE_TIMEOUT: Duration = Duration::from_secs(30);

async fn validate_video(bytes: &[u8], declared: &str) -> Result<MediaInfo, MediaValidationFailure> {
    let mut child = tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
            "-",
        ])
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| MediaValidationFailure::MetadataMissing)?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(bytes)
            .await
            .map_err(|e| MediaValidationFailure::IoError(e.to_string()))?;
    }

    let output = tokio::time::timeout(FFPROBE_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| MediaValidationFailure::Timeout)?
        .map_err(|e| MediaValidationFailure::IoError(e.to_string()))?;
    if !output.status.success() {
        return Err(MediaValidationFailure::DecodeFailed);
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| MediaValidationFailure::MetadataMissing)?;

    let mut width = None;
    let mut height = None;
    let mut duration = None;
    let mut detected = declared.to_string();

    if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            if stream.get("codec_type").and_then(|c| c.as_str()) == Some("video") {
                width = stream
                    .get("width")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| u32::try_from(v).ok());
                height = stream
                    .get("height")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| u32::try_from(v).ok());
                if let Some(codec) = stream.get("codec_name").and_then(|c| c.as_str()) {
                    detected = format!("video/{}", codec.to_lowercase());
                }
                break;
            }
        }
    }
    if let Some(fmt) = json.get("format").and_then(|f| f.as_object()) {
        duration = fmt.get("duration").and_then(|d| d.as_str()).and_then(|s| s.parse::<f64>().ok());
    }

    if width.is_none() || height.is_none() {
        return Err(MediaValidationFailure::MetadataMissing);
    }

    Ok(MediaInfo {
        width,
        height,
        duration_seconds: duration,
        detected_format: detected,
        declared_media_type: declared.to_string(),
    })
}

fn media_types_match(declared: &str, detected: &str) -> bool {
    if declared == detected {
        return true;
    }
    if declared.starts_with("image/") && detected.starts_with("image/") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_payload() {
        let validator = DefaultMediaValidator;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(validator.validate(&[], "image/png"));
        assert!(matches!(result, Err(MediaValidationFailure::Empty)));
    }

    #[test]
    fn rejects_malware_signature() {
        let validator = DefaultMediaValidator;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(validator.validate(b"MZ\x90\x00", "image/png"));
        assert!(matches!(
            result,
            Err(MediaValidationFailure::MalwareDetected)
        ));
    }

    #[test]
    fn validates_png() {
        let img = image::DynamicImage::new_rgba8(1, 1);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let bytes = buf.into_inner();
        let validator = DefaultMediaValidator;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let info = rt.block_on(validator.validate(&bytes, "image/png")).unwrap();
        assert_eq!(info.width, Some(1));
        assert_eq!(info.height, Some(1));
    }
}
