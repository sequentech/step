// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! An optional self-contained candidate portrait. Decode a bounded raster to
//! pixels, then write a clean, static PNG as a normal public image member. Never
//! turn a workbook cell into a fetched URL or a literal ballot src.

use std::io::Cursor;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::architect::CandidateImage;
use super::problem::{Code, Problem};
use super::sheet::Row;

pub(super) const COLUMN: &str = "image_data_url";
const PREFIX: &str = "data:image/png;base64,";
const MAX_CELL_BYTES: usize = 32_767;
const MAX_DIMENSION: u32 = 512;
const MAX_PNG_BYTES: usize = 128 * 1024;
const MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_PIXELS: u32 = 16 * 1024 * 1024;
const DECODE_MEMORY_BYTES: usize = 2 * 1024 * 1024;
const BAD_PNG: &str =
    "use a valid static PNG encoded as data:image/png;base64,...";

fn present(value: &Value) -> bool {
    !value.is_null() && !value.as_str().is_some_and(|s| s.trim().is_empty())
}

pub(super) fn problem(row: &Row, reason: &str) -> Problem {
    let origin = row.origin(Some(COLUMN));
    Problem::error(Code::InvalidValue, origin.to_string(), reason)
        .at(&origin)
        .id("candidate.inline-photo")
}

/// Applies to inline images only; existing uploaded files are unchanged.
#[derive(Default)]
pub(super) struct Budget {
    used_bytes: usize,
    used_pixels: u32,
}

impl Budget {
    pub(super) fn read(
        &mut self,
        row: &Row,
    ) -> Result<Option<CandidateImage>, Problem> {
        let Some(value) = row.get(COLUMN).filter(|value| present(value)) else {
            return Ok(None);
        };

        if row.get("image_document_id").is_some_and(present) {
            return Err(problem(
                row,
                "use an inline PNG or an image document, not both",
            ));
        }
        if super::plan_from_workbook::flag(row, "presentation.is_write_in")
            == Some(true)
        {
            return Err(problem(
                row,
                "a write-in slot cannot have a candidate photo",
            ));
        }

        let source = value.as_str().ok_or_else(|| problem(row, BAD_PNG))?;
        let remaining = MAX_TOTAL_BYTES - self.used_bytes;
        if remaining == 0 {
            return Err(problem(
                row,
                "candidate photos exceed the workbook's 4 MiB total limit",
            ));
        }
        let (bytes, pixels) =
            canonical_png(source, MAX_TOTAL_PIXELS - self.used_pixels)
                .map_err(|reason| problem(row, reason))?;
        if bytes.len() > remaining {
            return Err(problem(
                row,
                "candidate photos exceed the workbook's 4 MiB total limit",
            ));
        }
        self.used_bytes += bytes.len();
        self.used_pixels += pixels;
        Ok(Some(CandidateImage {
            file_name: photo_name(row.text("external_id").ok_or_else(
                || problem(row, "a photo needs its candidate external ID"),
            )?),
            bytes,
        }))
    }
}

/// The same source candidate can appear in several election/contest contexts.
/// The plan indexes files by name, so each context needs a stable unique name.
fn photo_name(external_id: &str) -> String {
    format!(
        "candidate-photo-{}.png",
        hex::encode(Sha256::digest(external_id.trim().as_bytes()))
    )
}

fn canonical_png(
    source: &str,
    remaining_pixels: u32,
) -> Result<(Vec<u8>, u32), &'static str> {
    if source.len() > MAX_CELL_BYTES {
        return Err(
            "the PNG data URL must fit in one Excel cell (32,767 characters)",
        );
    }
    let encoded = source.trim().strip_prefix(PREFIX).ok_or(BAD_PNG)?;
    let raw = STANDARD.decode(encoded).map_err(|_| BAD_PNG)?;
    if STANDARD.encode(&raw) != encoded {
        return Err(BAD_PNG);
    }

    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(&raw),
        png::Limits {
            bytes: DECODE_MEMORY_BYTES,
        },
    );
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| BAD_PNG)?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > MAX_DIMENSION
        || info.height > MAX_DIMENSION
    {
        return Err("the PNG must be at most 512 × 512 pixels");
    }
    let pixel_count = info.width * info.height;
    if pixel_count > remaining_pixels {
        return Err(
            "candidate photos exceed the workbook's 16 megapixel total limit",
        );
    }
    if info.animation_control.is_some() {
        return Err(BAD_PNG);
    }
    let size = reader.output_buffer_size().ok_or(BAD_PNG)?;
    let mut pixels = vec![0; size];
    let frame = reader.next_frame(&mut pixels).map_err(|_| BAD_PNG)?;
    reader.finish().map_err(|_| BAD_PNG)?;

    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, frame.width, frame.height);
    encoder.set_color(frame.color_type);
    encoder.set_depth(frame.bit_depth);
    let mut writer = encoder.write_header().map_err(|_| BAD_PNG)?;
    writer
        .write_image_data(&pixels[..frame.buffer_size()])
        .map_err(|_| BAD_PNG)?;
    writer.finish().map_err(|_| BAD_PNG)?;
    if bytes.len() > MAX_PNG_BYTES {
        return Err("the normalized PNG must be at most 128 KiB");
    }
    Ok((bytes, pixel_count))
}

#[cfg(test)]
pub(super) fn test_png_url(width: u32, height: u32) -> String {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width, height);
    encoder.set_color(png::ColorType::Rgb);
    let mut writer = encoder.write_header().unwrap();
    writer
        .write_image_data(&vec![42; (width * height * 3) as usize])
        .unwrap();
    writer.finish().unwrap();
    format!("{PREFIX}{}", STANDARD.encode(png))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_source_photo_has_different_stable_context_filenames() {
        let first = photo_name("post:one:contest:example:candidate:007");
        assert_eq!(first, photo_name("post:one:contest:example:candidate:007"));
        assert_ne!(first, photo_name("post:two:contest:example:candidate:007"));
        assert!(
            first.starts_with("candidate-photo-") && first.ends_with(".png")
        );
    }

    #[test]
    fn only_canonical_bounded_static_png_is_accepted() {
        let good = test_png_url(3, 2);
        let mut reader = png::Decoder::new(Cursor::new(
            canonical_png(&good, MAX_TOTAL_PIXELS).unwrap().0,
        ))
        .read_info()
        .unwrap();
        let mut rgb = vec![0; reader.output_buffer_size().unwrap()];
        let frame = reader.next_frame(&mut rgb).unwrap();
        assert_eq!(
            (frame.width, frame.height, frame.color_type),
            (3, 2, png::ColorType::Rgb)
        );
        assert_eq!(rgb, vec![42; 18]);
        assert!(canonical_png(&good, 5).is_err());
        assert_eq!(canonical_png(&good, 6).unwrap().1, 6);

        for invalid in [
            good.replacen("image/png", "image/svg+xml", 1),
            "https://unreachable.invalid/photo.png".to_string(),
            "data:image/png;base64,Sm90IGEgcGhvdG8=".to_string(),
            "data:image/png;base64,!!!!".to_string(),
            test_png_url(513, 1),
            format!("{PREFIX}{}", "A".repeat(MAX_CELL_BYTES)),
        ] {
            assert!(canonical_png(&invalid, MAX_TOTAL_PIXELS).is_err());
        }

        let mut raw =
            STANDARD.decode(good.strip_prefix(PREFIX).unwrap()).unwrap();
        raw.truncate(raw.len() - 6);
        assert!(canonical_png(
            &format!("{PREFIX}{}", STANDARD.encode(raw)),
            MAX_TOTAL_PIXELS
        )
        .is_err());
    }
}
