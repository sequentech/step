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
#[cfg(test)]
const MAX_CELL_BYTES: usize = 32_767;
const MAX_DIMENSION: u32 = 512;
const MAX_PNG_BYTES: usize = 128 * 1024;
const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const MAX_TOTAL_PIXELS: u32 = 16 * 1024 * 1024;
const DECODE_MEMORY_BYTES: usize = 2 * 1024 * 1024;
const BAD_PNG: &str =
    "use a valid static PNG encoded as data:image/png;base64,...";

pub(super) fn present(value: &Value) -> bool {
    !value.is_null() && !value.as_str().is_some_and(|s| s.trim().is_empty())
}

pub(super) fn problem(row: &Row, reason: &str) -> Problem {
    let origin = row.origin(Some(COLUMN));
    Problem::error(Code::InvalidValue, origin.to_string(), reason)
        .at(&origin)
        .id("candidate.inline-photo")
}

/// Prepared once per selected SQL candidate. The normal workbook representation
/// survives direct building, Architect editing, and XLSX serialization.
pub(super) struct PreparedPhoto {
    pub(super) url: String,
    bytes: usize,
    pixels: u32,
}
const MAX_SQL_BYTES: usize = 512 * 1024;
const MAX_SQL_DIMENSION: u32 = 1024;
const SQL_THUMBNAIL: u32 = 160;

/// Cells retain the existing Excel limits. BLOBs accept larger originals and
/// become thumbnails; the original upload stays unchanged.
#[derive(Clone, Copy)]
enum Source {
    Excel,
    Sql,
}
impl Source {
    fn max_dimension(self) -> u32 {
        match self {
            Self::Excel => MAX_DIMENSION,
            Self::Sql => MAX_SQL_DIMENSION,
        }
    }
}

const TOTAL_BYTES_ERROR: &str =
    "candidate photos exceed the workbook's 32 MiB total limit";
const TOTAL_PIXELS_ERROR: &str =
    "candidate photos exceed the workbook's 16 megapixel total limit";
const SQL_TOTAL_PIXELS_ERROR: &str =
    "SQL candidate photos exceed 64 megapixels of original image data";
const BAD_PHOTO: &str = "supply a valid static PNG or JPEG";
const JPEG_PREFIX: &str = "data:image/jpeg;base64,";

#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
mod sql_formats;

#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[derive(Default)]
pub(super) struct SqlBudget {
    used_bytes: usize,
    used_pixels: u32,
}

#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
impl SqlBudget {
    pub(super) fn decode(
        &mut self,
        raw: &[u8],
    ) -> Result<PreparedPhoto, &'static str> {
        if raw.len() > MAX_SQL_BYTES {
            return Err("the SQL PICTURE must be at most 512 KiB");
        }
        if raw.len() > 64 * 1024 * 1024 - self.used_bytes {
            return Err(
                "SQL candidate photos exceed 64 MiB of original image data",
            );
        }
        let remaining = 64 * 1024 * 1024 - self.used_pixels;
        let (bytes, pixels, decoded) = sql_formats::decode(raw, remaining)?;
        self.used_bytes += raw.len();
        self.used_pixels += decoded;
        Ok(PreparedPhoto {
            url: format!("{PREFIX}{}", STANDARD.encode(&bytes)),
            pixels,
            bytes: bytes.len(),
        })
    }
}

/// Charge the combined budget for every ballot placement, not just unique BLOBs.
#[derive(Default)]
pub(super) struct Budget {
    used_bytes: usize,
    used_pixels: u32,
}

impl Budget {
    /// Decode a supplemental source once before fan-out, keeping only the clean
    /// PNG in derived workbooks (not the original JPEG/PNG metadata).
    #[cfg(all(
        feature = "election_config_xlsx",
        feature = "election_config_archive"
    ))]
    pub(super) fn prepare_cell(
        &mut self,
        row: &Row,
    ) -> Result<Option<PreparedPhoto>, Problem> {
        let Some(cell) = row.get(COLUMN).filter(|v| present(v)) else {
            return Ok(None);
        };
        let source = cell.as_str().ok_or_else(|| problem(row, BAD_PHOTO))?;
        let (bytes, pixels) =
            canonical_png(source, MAX_TOTAL_PIXELS - self.used_pixels)
                .map_err(|why| problem(row, why))?;
        self.charge(bytes.len(), pixels)
            .map_err(|why| problem(row, why))?;
        Ok(Some(PreparedPhoto {
            url: format!("{PREFIX}{}", STANDARD.encode(&bytes)),
            bytes: bytes.len(),
            pixels,
        }))
    }
    fn charge(
        &mut self,
        bytes: usize,
        pixels: u32,
    ) -> Result<(), &'static str> {
        if bytes > MAX_TOTAL_BYTES - self.used_bytes {
            return Err(TOTAL_BYTES_ERROR);
        }
        if pixels > MAX_TOTAL_PIXELS - self.used_pixels {
            return Err(TOTAL_PIXELS_ERROR);
        }
        self.used_bytes += bytes;
        self.used_pixels += pixels;
        Ok(())
    }

    #[cfg(all(
        feature = "election_config_xlsx",
        feature = "election_config_archive"
    ))]
    pub(super) fn charge_picture(
        &mut self,
        photo: &PreparedPhoto,
    ) -> Result<(), &'static str> {
        self.charge(photo.bytes, photo.pixels)
    }

    pub(super) fn read(
        &mut self,
        row: &Row,
    ) -> Result<Option<CandidateImage>, Problem> {
        let Some(cell) = row.get(COLUMN).filter(|value| present(value)) else {
            return Ok(None);
        };
        if row.get("image_document_id").is_some_and(present) {
            return Err(problem(
                row,
                "use a photo or an image document, not both",
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
        if self.used_bytes == MAX_TOTAL_BYTES {
            return Err(problem(row, TOTAL_BYTES_ERROR));
        }
        let name = row.text("external_id").ok_or_else(|| {
            problem(row, "a photo needs its candidate external ID")
        })?;
        let source = cell.as_str().ok_or_else(|| problem(row, BAD_PHOTO))?;
        let (bytes, pixels) =
            canonical_png(source, MAX_TOTAL_PIXELS - self.used_pixels)
                .map_err(|why| problem(row, why))?;
        self.charge(bytes.len(), pixels)
            .map_err(|why| problem(row, why))?;
        Ok(Some(CandidateImage {
            file_name: photo_name(name),
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
    // Larger normalized photographs are emitted across Excel's standard
    // adjacent continuation cells; they must still have a bounded total size.
    if source.len() > super::sheet::MAX_PHOTO_URL {
        return Err(
            "the image data URL exceeds the 128 KiB decoded input limit",
        );
    }
    let source = source.trim();
    let (encoded, jpeg) = if let Some(s) = source.strip_prefix(PREFIX) {
        (s, false)
    } else if let Some(s) = source.strip_prefix(JPEG_PREFIX) {
        (s, true)
    } else {
        return Err(BAD_PHOTO);
    };
    let raw = STANDARD.decode(encoded).map_err(|_| BAD_PHOTO)?;
    if raw.len() > MAX_PNG_BYTES || STANDARD.encode(&raw) != encoded {
        return Err(BAD_PHOTO);
    }
    let (bytes, pixels, _) = if jpeg {
        decode_jpeg(&raw, remaining_pixels, Source::Excel)?
    } else {
        decode_png(&raw, remaining_pixels, Source::Excel)?
    };
    Ok((bytes, pixels))
}

fn pixel_count(
    width: u32,
    height: u32,
    remaining: u32,
    source: Source,
) -> Result<u32, &'static str> {
    if width == 0
        || height == 0
        || width > source.max_dimension()
        || height > source.max_dimension()
    {
        return Err(match source {
            Source::Excel => "the photo must be at most 512 × 512 pixels",
            Source::Sql => "the SQL photo must be at most 1024 × 1024 pixels",
        });
    }
    let count = width * height;
    if count > remaining {
        return Err(match source {
            Source::Sql => SQL_TOTAL_PIXELS_ERROR,
            Source::Excel => TOTAL_PIXELS_ERROR,
        });
    }
    Ok(count)
}

/// The primary TIFF directory controls the displayed orientation. Read only
/// its fixed-size Orientation entry; do not walk thumbnails, GPS, or maker tags.
/// Re-encoding strips metadata, so do not silently turn a portrait sideways.
fn require_upright_exif(exif: Option<&[u8]>) -> Result<(), &'static str> {
    let Some(raw) = exif else { return Ok(()) };
    const INVALID: &str = "invalid EXIF orientation metadata; export an upright image without EXIF";
    let little = match raw.get(..4) {
        Some(b"II\x2a\x00") => true,
        Some(b"MM\x00\x2a") => false,
        _ => return Err(INVALID),
    };
    let short = |x: &[u8]| {
        if little {
            u16::from_le_bytes([x[0], x[1]])
        } else {
            u16::from_be_bytes([x[0], x[1]])
        }
    };
    let word = |x: [u8; 4]| {
        if little {
            u32::from_le_bytes(x)
        } else {
            u32::from_be_bytes(x)
        }
    };
    let offset = word(
        raw.get(4..8)
            .ok_or(INVALID)?
            .try_into()
            .map_err(|_| INVALID)?,
    ) as usize;
    let directory = raw.get(offset..).ok_or(INVALID)?;
    let count = usize::from(short(directory.get(..2).ok_or(INVALID)?));
    let entries = directory.get(2..2 + 12 * count).ok_or(INVALID)?;
    for entry in entries.chunks_exact(12) {
        if short(&entry[..2]) == 0x0112 {
            if short(&entry[2..4]) != 3
                || word(entry[4..8].try_into().map_err(|_| INVALID)?) != 1
            {
                return Err(INVALID);
            }
            if short(&entry[8..10]) != 1 {
                return Err("rotate the source pixels upright and remove the EXIF orientation before importing");
            }
        }
    }
    Ok(())
}
fn decode_jpeg(
    raw: &[u8],
    remaining: u32,
    source: Source,
) -> Result<(Vec<u8>, u32, u32), &'static str> {
    use zune_core::{colorspace::ColorSpace, options::DecoderOptions};
    if !raw.starts_with(&[0xff, 0xd8, 0xff]) || !raw.ends_with(&[0xff, 0xd9]) {
        return Err(BAD_PHOTO);
    }
    let options = DecoderOptions::default()
        .set_strict_mode(true)
        .set_use_unsafe(false)
        .set_max_width(source.max_dimension() as usize)
        .set_max_height(source.max_dimension() as usize)
        .jpeg_set_max_scans(32)
        .jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(raw, options);
    decoder.decode_headers().map_err(|_| BAD_PHOTO)?;
    require_upright_exif(decoder.exif().map(Vec::as_slice))?;
    let info = decoder.info().ok_or(BAD_PHOTO)?;
    let (w, h) = (u32::from(info.width), u32::from(info.height));
    let count = pixel_count(w, h, remaining, source)?;
    let (color, channels) = match decoder.get_output_colorspace() {
        Some(ColorSpace::Luma) => (png::ColorType::Grayscale, 1),
        Some(ColorSpace::RGB) => (png::ColorType::Rgb, 3),
        _ => return Err(BAD_PHOTO),
    };
    let mut pixels = vec![0; count as usize * channels];
    decoder.decode_into(&mut pixels).map_err(|_| BAD_PHOTO)?;
    encode_raster(&pixels, w, h, color, png::BitDepth::Eight, source)
}

fn decode_png(
    raw: &[u8],
    remaining_pixels: u32,
    source: Source,
) -> Result<(Vec<u8>, u32, u32), &'static str> {
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(raw),
        png::Limits {
            bytes: match source {
                Source::Excel => DECODE_MEMORY_BYTES,
                Source::Sql => 8 * 1024 * 1024,
            },
        },
    );
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| BAD_PNG)?;
    let info = reader.info();
    pixel_count(info.width, info.height, remaining_pixels, source)?;
    if info.animation_control.is_some() {
        return Err(BAD_PNG);
    }
    let size = reader.output_buffer_size().ok_or(BAD_PNG)?;
    let mut pixels = vec![0; size];
    let frame = reader.next_frame(&mut pixels).map_err(|_| BAD_PNG)?;
    reader.finish().map_err(|_| BAD_PNG)?;
    // A PNG can carry eXIf after its IDAT chunks.
    require_upright_exif(reader.info().exif_metadata.as_deref())?;

    encode_raster(
        &pixels[..frame.buffer_size()],
        frame.width,
        frame.height,
        frame.color_type,
        frame.bit_depth,
        source,
    )
}

/// Area-average into a small ballot thumbnail without stretching, cropping,
/// or upscaling. The pixel decoder has already checked the input dimensions.
fn encode_raster(
    raw: &[u8],
    w: u32,
    h: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    source: Source,
) -> Result<(Vec<u8>, u32, u32), &'static str> {
    let side = w.max(h);
    if matches!(source, Source::Excel) || side <= SQL_THUMBNAIL {
        return Ok((encode_png(raw, w, h, color, depth)?, w * h, w * h));
    }
    if depth != png::BitDepth::Eight {
        return Err(BAD_PHOTO);
    }
    let (ow, oh) = (
        (w * SQL_THUMBNAIL / side).max(1),
        (h * SQL_THUMBNAIL / side).max(1),
    );
    let channels = color.samples();
    let mut dst = Vec::with_capacity((ow * oh) as usize * channels);
    for y in 0..oh {
        let (top, bottom) = (y * h / oh, (y + 1) * h / oh);
        for x in 0..ow {
            let (left, right) = (x * w / ow, (x + 1) * w / ow);
            let area = (bottom - top) * (right - left);
            let mut sums = [0u32; 4];
            for iy in top..bottom {
                for ix in left..right {
                    let start = (iy * w + ix) as usize * channels;
                    for (sum, sample) in
                        sums.iter_mut().zip(&raw[start..start + channels])
                    {
                        *sum += u32::from(*sample);
                    }
                }
            }
            dst.extend(
                sums[..channels]
                    .iter()
                    .map(|sum| ((sum + area / 2) / area) as u8),
            );
        }
    }
    Ok((encode_png(&dst, ow, oh, color, depth)?, ow * oh, w * h))
}

fn encode_png(
    pixels: &[u8],
    w: u32,
    h: u32,
    color: png::ColorType,
    depth: png::BitDepth,
) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, w, h);
    encoder.set_color(color);
    encoder.set_depth(depth);
    let mut writer = encoder.write_header().map_err(|_| BAD_PNG)?;
    writer.write_image_data(pixels).map_err(|_| BAD_PNG)?;
    writer.finish().map_err(|_| BAD_PNG)?;
    if bytes.len() > MAX_PNG_BYTES {
        return Err("the normalized PNG must be at most 128 KiB");
    }
    Ok(bytes)
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

    fn candidate_row(
        id: Option<&str>,
        photo: Value,
        other: &[(&str, Value)],
    ) -> Row {
        let mut cells = vec![(COLUMN.to_string(), photo)];
        if let Some(id) = id {
            cells.push(("external_id".to_string(), Value::String(id.into())));
        }
        cells.extend(
            other
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone())),
        );
        Row {
            sheet: "Candidates".into(),
            number: 7,
            cells,
        }
    }

    fn rejected(budget: &mut Budget, row: &Row) -> Problem {
        let problem = budget.read(row).err().expect("photo must be rejected");
        assert_eq!(problem.code, Code::InvalidValue);
        assert_eq!(problem.id.as_deref(), Some("candidate.inline-photo"));
        let at = problem.at.as_ref().expect("photo error names the cell");
        assert_eq!(at.sheet, row.sheet);
        assert_eq!(at.row, Some(row.number));
        assert_eq!(at.column.as_deref(), Some(COLUMN));
        problem
    }

    #[test]
    fn optional_photo_is_blank_but_conflicting_inputs_are_refused() {
        let mut budget = Budget::default();
        for blank in [Value::Null, Value::String(" \t ".into())] {
            assert!(budget
                .read(&candidate_row(Some("007"), blank, &[]))
                .unwrap()
                .is_none());
        }
        let url = Value::String(test_png_url(3, 3));
        let document = Value::String("already-named-image".into());
        assert!(rejected(
            &mut budget,
            &candidate_row(
                Some("007"),
                url.clone(),
                &[("image_document_id", document)],
            )
        )
        .message
        .contains("not both"));
        assert!(rejected(
            &mut budget,
            &candidate_row(
                Some("007"),
                url.clone(),
                &[("presentation.is_write_in", Value::Bool(true))],
            )
        )
        .message
        .contains("write-in"));
        assert_eq!(
            rejected(
                &mut budget,
                &candidate_row(Some("007"), Value::Bool(true), &[])
            )
            .message,
            BAD_PHOTO
        );
        assert_eq!((budget.used_bytes, budget.used_pixels), (0, 0));
        assert!(rejected(&mut budget, &candidate_row(None, url, &[]))
            .message
            .contains("external ID"));
    }

    #[test]
    fn workbook_byte_and_pixel_budgets_fail_closed() {
        let row =
            candidate_row(Some("007"), Value::String(test_png_url(4, 3)), &[]);
        let known = Budget::default().read(&row).unwrap().unwrap();
        let mut full = Budget {
            used_bytes: MAX_TOTAL_BYTES,
            used_pixels: 0,
        };
        assert!(rejected(&mut full, &row).message.contains("32 MiB"));
        assert_eq!((full.used_bytes, full.used_pixels), (MAX_TOTAL_BYTES, 0));

        let mut one_byte_short = Budget {
            used_bytes: MAX_TOTAL_BYTES - known.bytes.len() + 1,
            used_pixels: 0,
        };
        assert!(rejected(&mut one_byte_short, &row)
            .message
            .contains("32 MiB"));
        assert_eq!(one_byte_short.used_pixels, 0);

        let mut one_pixel_short = Budget {
            used_bytes: 0,
            used_pixels: MAX_TOTAL_PIXELS - 11,
        };
        assert!(rejected(&mut one_pixel_short, &row)
            .message
            .contains("16 megapixel"));
        assert_eq!(one_pixel_short.used_bytes, 0);

        let mut fits = Budget {
            used_bytes: MAX_TOTAL_BYTES - known.bytes.len(),
            used_pixels: MAX_TOTAL_PIXELS - 12,
        };
        assert_eq!(fits.read(&row).unwrap().unwrap().bytes, known.bytes);
        assert_eq!(
            (fits.used_bytes, fits.used_pixels),
            (MAX_TOTAL_BYTES, MAX_TOTAL_PIXELS)
        );
        assert!(rejected(&mut fits, &row).message.contains("32 MiB"));
    }

    #[test]
    fn rejects_animated_png_and_corrupted_compressed_frame_data() {
        let mut image = Vec::new();
        let mut encoder = png::Encoder::new(&mut image, 2, 2);
        encoder.set_animated(1, 0).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0, 1, 2, 3]).unwrap();
        writer.finish().unwrap();
        let animated = format!("{PREFIX}{}", STANDARD.encode(&image));
        assert_eq!(canonical_png(&animated, MAX_TOTAL_PIXELS), Err(BAD_PNG));

        let valid = test_png_url(3, 2);
        let mut raw = STANDARD
            .decode(valid.strip_prefix(PREFIX).unwrap())
            .unwrap();
        let idat = raw.windows(4).position(|chunk| chunk == b"IDAT").unwrap();
        raw[idat + 4] = 0; // Illegal zlib header inside an otherwise valid PNG.
        let damaged = format!("{PREFIX}{}", STANDARD.encode(raw));
        assert_eq!(canonical_png(&damaged, MAX_TOTAL_PIXELS), Err(BAD_PNG));
    }

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
    #[cfg(all(
        feature = "election_config_xlsx",
        feature = "election_config_archive"
    ))]
    #[test]
    fn historical_jpeg_is_decoded_bounded_and_scaled_and_never_treated_as_text()
    {
        // Synthetic grayscale 600 x 300 JPEG; no source photographs.
        let input = STANDARD.decode(concat!(
            "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDABALDA4MChAODQ4SERATGCgaGBYWGDEjJR0oOjM9PDkz",
            "ODdASFxOQERXRTc4UG1RV19iZ2hnPk1xeXBkeFxlZ2P/wAALCAEsAlgBAREA/8QAFQABAQAAAAAA",
            "AAAAAAAAAAAAAAL/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/9oACAEBAAA/AIAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAB/9k="
        )).unwrap();
        let mut budget = SqlBudget::default();
        let photo = budget.decode(&input).unwrap();
        assert_eq!(budget.used_pixels, 600 * 300);
        assert_eq!(photo.pixels, 160 * 80);
        let png = STANDARD.decode(&photo.url[PREFIX.len()..]).unwrap();
        let mut reader =
            png::Decoder::new(Cursor::new(&png)).read_info().unwrap();
        assert_eq!((reader.info().width, reader.info().height), (160, 80));
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels).unwrap();
        assert!(pixels.iter().all(|p| (121..=125).contains(p)));
        assert!(budget.decode(&input[..input.len() - 2]).is_err());
        assert!(budget.decode(&vec![0; MAX_SQL_BYTES + 1]).is_err());
        let large_cell = format!("{JPEG_PREFIX}{}", STANDARD.encode(&input));
        assert!(
            canonical_png(&large_cell, MAX_TOTAL_PIXELS).is_err(),
            "Excel keeps its stricter image dimensions"
        );
        let wrong_mime =
            format!("{JPEG_PREFIX}{}", &test_png_url(2, 2)[PREFIX.len()..]);
        assert!(canonical_png(&wrong_mime, MAX_TOTAL_PIXELS).is_err());
    }
    #[test]
    fn exif_orientation_cannot_be_silently_discarded() {
        let big = b"MM\0\x2a\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\x01\0\0";
        let mut little =
            *b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x01\0\0\0";
        assert!(require_upright_exif(Some(big)).is_ok());
        assert!(require_upright_exif(Some(&little)).is_ok());
        little[18] = 6;
        assert!(require_upright_exif(Some(&little))
            .unwrap_err()
            .contains("rotate"));
        assert!(require_upright_exif(Some(&little[..11])).is_err());
    }
    #[test]
    fn a_late_png_orientation_is_not_silently_stripped() {
        let mut bytes = Vec::new();
        let encoder = png::Encoder::new(&mut bytes, 2, 2);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0u8; 4]).unwrap();
        writer
            .write_chunk(
                png::chunk::eXIf,
                b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0",
            )
            .unwrap();
        writer.finish().unwrap();
        assert!(decode_png(&bytes, MAX_TOTAL_PIXELS, Source::Excel)
            .unwrap_err()
            .contains("rotate"));
    }
}
