// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Binary formats for SQL PICTURE only. XLSX and the ballot still get PNG;
//! neither has to load a decoder for a device-specific image format.

use super::{
    decode_jpeg, decode_png, encode_raster, require_upright_exif, Source,
    MAX_SQL_BYTES, SQL_TOTAL_PIXELS_ERROR,
};
const BAD_WEBP: &str = "supply a valid static WebP within the SQL photo limits";
const BAD_AVIF: &str =
    "supply a valid single-image AVIF within the SQL photo limits";
const BAD_FORMAT: &str = "supply a valid static PNG, JPEG, WebP or AVIF";
const ANIMATED: &str = "SQL candidate photos must be static, not animated";
const WEBP_GROUP_LIMIT: usize = 128;
const TOO_MANY_WEBP_GROUPS: &str =
    "the WebP lossless image or alpha contains too many entropy groups";
const BAD_AVIF_COLOR: &str =
    "export the AVIF as standard SDR without an embedded ICC profile";
// Permit landscape/portrait camera exports wider than 1024 without raising
// the historical path's one-mebipixel per-portrait pixel allocation ceiling.
const EXTENDED_MAX_SIDE: u32 = 1280;
const EXTENDED_MAX_PIXELS: u32 = 1024 * 1024;

type Raster = (Vec<u8>, u32, u32);

pub(super) fn decode(
    raw: &[u8],
    remaining: u32,
) -> Result<Raster, &'static str> {
    if raw.starts_with(b"\x89PNG\r\n\x1a\n") {
        decode_png(raw, remaining, Source::Sql)
    } else if raw.starts_with(&[0xff, 0xd8, 0xff]) {
        decode_jpeg(raw, remaining, Source::Sql)
    } else if raw.starts_with(b"RIFF") && raw.get(8..12) == Some(b"WEBP") {
        decode_webp(raw, remaining)
    } else if raw.get(4..8) == Some(b"ftyp") {
        decode_avif(raw, remaining)
    } else {
        Err(BAD_FORMAT)
    }
}

fn extended_pixel_count(
    width: u32,
    height: u32,
    remaining: u32,
) -> Result<u32, &'static str> {
    if width == 0
        || height == 0
        || width > EXTENDED_MAX_SIDE
        || height > EXTENDED_MAX_SIDE
        || width * height > EXTENDED_MAX_PIXELS
    {
        return Err("SQL AVIF and WebP photos must fit 1280 pixels per side and 1,048,576 total pixels");
    }
    let count = width * height;
    if count > remaining {
        return Err(SQL_TOTAL_PIXELS_ERROR);
    }
    Ok(count)
}

fn decode_webp(raw: &[u8], remaining: u32) -> Result<Raster, &'static str> {
    let webp = webp_chunks(raw)?;
    let (w, h) = webp.dimensions;
    let count = extended_pixel_count(w, h, remaining)?;
    if let Some(lossless) = webp.lossless {
        if webp.alpha.is_some() {
            return Err(BAD_WEBP);
        }
        check_lossless_webp_groups(&lossless[5..], w, h)?;
    }
    if let Some(alpha) = webp.alpha {
        let (&header, data) = alpha.split_first().ok_or(BAD_WEBP)?;
        match header & 3 {
            0 => {}
            // Compressed ALPH uses the same VP8L stream, without its header.
            1 => check_lossless_webp_groups(data, w, h)?,
            _ => return Err(BAD_WEBP),
        }
    }
    let options = oxideav_webp::DecodeOptions::new()
        .with_strict(true)
        .with_max_bytes(Some(MAX_SQL_BYTES as u64))
        .with_max_width(Some(EXTENDED_MAX_SIDE))
        .with_max_height(Some(EXTENDED_MAX_SIDE))
        .with_max_pixels(Some(u64::from(count)));
    let image =
        oxideav_webp::decode_with(raw, &options).map_err(|_| BAD_WEBP)?;
    if (image.width, image.height) != (w, h) {
        return Err(BAD_WEBP);
    }
    let pixels = image.to_rgba8();
    encode_raster(
        &pixels,
        w,
        h,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        Source::Sql,
    )
}

// The dimensions do not bound the number of VP8L Huffman groups: an entropy
// image can name a large group ID even when only a few are used. Walk the
// transform headers with the same decoder primitives as the full decode.
// The transform bodies and the entropy image each use only one group and are
// at most 320×320 pixels under our dimension cap. Check the total group count
// before the decoder reserves any per-group tables.
fn check_lossless_webp_groups(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<(), &'static str> {
    if lossless_webp_group_count(bytes, width, height)? > WEBP_GROUP_LIMIT {
        return Err(TOO_MANY_WEBP_GROUPS);
    }
    Ok(())
}

fn lossless_webp_group_count(
    bytes: &[u8],
    mut width: u32,
    height: u32,
) -> Result<usize, &'static str> {
    use oxideav_webp::{
        meta_prefix::{ImageRole, MetaPrefixCodes, MetaPrefixHeader},
        vp8l::bit_reader::BitReader,
        vp8l_decode::{decode_entropy_coded_image, decode_entropy_image},
        vp8l_transform::color_indexing_width_bits,
    };
    let mut reader = BitReader::new(bytes);
    let mut seen = 0u8;
    while reader.read_bit().map_err(|_| BAD_WEBP)? {
        let kind = reader.read_bits(2).map_err(|_| BAD_WEBP)?;
        let mask = 1 << kind;
        if seen & mask != 0 {
            return Err(BAD_WEBP);
        }
        seen |= mask;
        match kind {
            0 | 1 => {
                let step =
                    1 << (reader.read_bits(3).map_err(|_| BAD_WEBP)? + 2);
                decode_entropy_coded_image(
                    &mut reader,
                    width.div_ceil(step),
                    height.div_ceil(step),
                )
                .map_err(|_| BAD_WEBP)?;
            }
            2 => {} // subtract-green has no transform data
            3 => {
                let colors = reader.read_bits(8).map_err(|_| BAD_WEBP)? + 1;
                decode_entropy_coded_image(&mut reader, colors, 1)
                    .map_err(|_| BAD_WEBP)?;
                width = width
                    .div_ceil(1 << color_indexing_width_bits(colors as usize));
            }
            _ => return Err(BAD_WEBP),
        }
    }
    let header =
        MetaPrefixHeader::read(&mut reader, ImageRole::Argb, width, height)
            .map_err(|_| BAD_WEBP)?;
    if let MetaPrefixCodes::EntropyImagePending {
        prefix_bits,
        image_width,
        image_height,
        ..
    } = header.codes
    {
        let index = decode_entropy_image(
            &mut reader,
            prefix_bits,
            image_width,
            image_height,
        )
        .map_err(|_| BAD_WEBP)?;
        return Ok(index.num_prefix_groups());
    }
    Ok(1)
}

struct WebpChunks<'a> {
    dimensions: (u32, u32),
    lossless: Option<&'a [u8]>,
    alpha: Option<&'a [u8]>,
}

// Check coded and canvas geometry, disallow multiple images, and verify chunk
// lengths before any allocation-sized data is passed to the full decoder.
fn webp_chunks(raw: &[u8]) -> Result<WebpChunks<'_>, &'static str> {
    let file_size = u32::from_le_bytes(
        raw.get(4..8)
            .ok_or(BAD_WEBP)?
            .try_into()
            .map_err(|_| BAD_WEBP)?,
    ) as usize;
    if raw.len().checked_sub(8) != Some(file_size) {
        return Err(BAD_WEBP);
    }
    let mut chunks = raw.get(12..).ok_or(BAD_WEBP)?;
    let mut canvas = None;
    let mut coded = None;
    let mut exif = None;
    let mut lossless = None;
    let mut alpha = None;
    // At most 64 chunks. The final iteration is only for validation.
    for index in 0..=64 {
        if chunks.is_empty() {
            let coded = coded.ok_or(BAD_WEBP)?;
            if canvas.is_some_and(|canvas| canvas != coded) {
                return Err(BAD_WEBP);
            }
            require_upright_exif(exif)?;
            return Ok(WebpChunks {
                dimensions: coded,
                lossless,
                alpha,
            });
        }
        if index == 64 {
            return Err(BAD_WEBP);
        }
        let (header, data) = chunks.split_at_checked(8).ok_or(BAD_WEBP)?;
        let len =
            u32::from_le_bytes(header[4..].try_into().map_err(|_| BAD_WEBP)?)
                as usize;
        let (payload, tail) = data.split_at_checked(len).ok_or(BAD_WEBP)?;
        if len % 2 == 1 && tail.first() != Some(&0) {
            return Err(BAD_WEBP);
        }
        chunks = tail.get(len % 2..).ok_or(BAD_WEBP)?;
        let dimension = match &header[..4] {
            b"VP8X" => {
                if index != 0 || payload.len() != 10 {
                    return Err(BAD_WEBP);
                }
                if payload[0] & 2 != 0 {
                    return Err(ANIMATED);
                }
                let read24 =
                    |s: &[u8]| u32::from_le_bytes([s[0], s[1], s[2], 0]);
                canvas = Some((
                    read24(&payload[4..7]) + 1,
                    read24(&payload[7..10]) + 1,
                ));
                continue;
            }
            b"VP8 " => {
                let h = payload.get(..10).ok_or(BAD_WEBP)?;
                if h[0] & 1 != 0 || h[3..6] != [0x9d, 0x01, 0x2a] {
                    return Err(BAD_WEBP);
                }
                let side =
                    |l, hi| u32::from(u16::from_le_bytes([l, hi]) & 0x3fff);
                (side(h[6], h[7]), side(h[8], h[9]))
            }
            b"VP8L" => {
                let h = payload.get(..5).ok_or(BAD_WEBP)?;
                if h[0] != 0x2f || h[4] >> 5 != 0 {
                    return Err(BAD_WEBP);
                }
                let dim = u32::from_le_bytes([h[1], h[2], h[3], h[4]]);
                lossless = Some(payload);
                ((dim & 0x3fff) + 1, ((dim >> 14) & 0x3fff) + 1)
            }
            b"ANIM" | b"ANMF" => return Err(ANIMATED),
            b"ALPH" => {
                if alpha.replace(payload).is_some() {
                    return Err(BAD_WEBP);
                }
                continue;
            }
            b"EXIF" => {
                let tiff = payload.strip_prefix(b"Exif\0\0").unwrap_or(payload);
                if exif.replace(tiff).is_some() {
                    return Err(BAD_WEBP);
                }
                continue;
            }
            _ => continue,
        };
        if coded.replace(dimension).is_some() {
            return Err(BAD_WEBP);
        }
    }
    Err(BAD_WEBP)
}

fn avif_limits(
    pixels: u32,
) -> Result<avif_rust::NativeDecodeLimits, &'static str> {
    let side = EXTENDED_MAX_SIDE as usize;
    avif_rust::NativeDecodeLimits::new(
        MAX_SQL_BYTES,
        side,
        side,
        pixels.min(EXTENDED_MAX_PIXELS) as usize,
        8 * 1024 * 1024, // one plane
        256 * 1024,      // metadata
        256 * 1024,      // ICC
        16,              // items
        64,              // properties
        0,               // grid cells
        0,               // derived image depth
        1,               // frame
    )
    .with_max_live_allocation_bytes(48 * 1024 * 1024)
    .map_err(|_| BAD_AVIF)
}

fn is_standard_sdr(primaries: u16, transfer: u16, matrix: u16) -> bool {
    matches!(primaries, 1 | 2 | 5 | 6 | 7)
        && matches!(transfer, 1 | 2 | 6 | 7 | 13)
        && matches!(matrix, 0 | 1 | 2 | 5 | 6 | 7)
}

fn decode_avif(raw: &[u8], remaining: u32) -> Result<Raster, &'static str> {
    still_avif_brand(raw)?;
    let info = avif_rust::parse_native_info(raw, &avif_limits(u32::MAX)?)
        .map_err(|_| BAD_AVIF)?;
    let (width, height) = (
        info.info().width.ok_or(BAD_AVIF)?,
        info.info().height.ok_or(BAD_AVIF)?,
    );
    extended_pixel_count(width, height, remaining)?;
    // Explicitly check the container too: an AV1 sequence can omit its
    // color description while a separate HEIF colr is still definitive.
    // Do not silently drop or combine arbitrary ICC and nclx profiles.
    let container_color = info.color_information();
    if container_color.icc_profile.is_some()
        || !container_color.unknown_colr.is_empty()
        || container_color.nclx.is_some_and(|color| {
            !is_standard_sdr(
                color.color_primaries,
                color.transfer_characteristics,
                color.matrix_coefficients,
            )
        })
    {
        return Err(BAD_AVIF_COLOR);
    }
    // HEIF presentation is expressed by container properties, not EXIF.
    // This bounded pixel API does not apply those transforms. Reject them
    // rather than silently changing how the portrait is supposed to look.
    if info.info().rotation.is_some_and(|r| r.angle != 0)
        || info.info().mirror.is_some()
        || info.info().clean_aperture.is_some()
        || info
            .pixel_aspect_ratio()
            .is_some_and(|(w, h)| w != h || w == 0)
    {
        return Err("export the AVIF without rotation, mirroring, cropping or nonsquare pixels");
    }
    drop(info);
    let decoded = avif_rust::decode_frame_bytes_strict_with_limits(
        raw,
        &avif_limits(remaining)?,
    )
    .map_err(|_| BAD_AVIF)?;
    let frame = decoded.frame();
    // The output PNG has no profile. Accept common SDR CICP (sRGB/BT.709
    // and the BT.601 families) or untagged stills; reject wide-gamut/HDR
    // rather than quietly changing how a candidate looks. The decoder still
    // handles sample range, the identity matrix and straight-alpha output.
    if frame.color_config.color_description.is_some_and(|color| {
        !is_standard_sdr(
            color.color_primaries.into(),
            color.transfer_characteristics.into(),
            color.matrix_coefficients.into(),
        )
    }) {
        return Err(BAD_AVIF_COLOR);
    }
    let w = u32::try_from(frame.width).map_err(|_| BAD_AVIF)?;
    let h = u32::try_from(frame.height).map_err(|_| BAD_AVIF)?;
    let count = extended_pixel_count(w, h, remaining)?;
    if frame.width != frame.render_width || frame.height != frame.render_height
    {
        return Err(BAD_AVIF);
    }
    let image = frame.to_rgba8().map_err(|_| BAD_AVIF)?;
    drop(decoded);
    if image.width != w as usize
        || image.height != h as usize
        || image.rgba.len() != count as usize * 4
    {
        return Err(BAD_AVIF);
    }
    encode_raster(
        &image.rgba,
        w,
        h,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        Source::Sql,
    )
}

// An arbitrary ISO BMFF ftyp (HEIC, MP4, etc.) is not an AVIF. The decoder
// also checks the still structure and the AV1 headers, not just these brands.
fn still_avif_brand(raw: &[u8]) -> Result<(), &'static str> {
    let size = u32::from_be_bytes(
        raw.get(..4)
            .ok_or(BAD_AVIF)?
            .try_into()
            .map_err(|_| BAD_AVIF)?,
    );
    let (header, size) = match size {
        0 => (8, raw.len()),
        1 => {
            let extended = raw.get(8..16).ok_or(BAD_AVIF)?;
            let size =
                u64::from_be_bytes(extended.try_into().map_err(|_| BAD_AVIF)?);
            (16, usize::try_from(size).map_err(|_| BAD_AVIF)?)
        }
        n => (8, n as usize),
    };
    let payload = raw.get(header..size).ok_or(BAD_AVIF)?;
    let major = payload.get(..4).ok_or(BAD_AVIF)?;
    let compatible = payload.get(8..).ok_or(BAD_AVIF)?.chunks_exact(4);
    if !compatible.remainder().is_empty() {
        return Err(BAD_AVIF);
    }
    if major == b"avis" || compatible.clone().any(|b| b == b"avis") {
        return Err(ANIMATED);
    }
    if major == b"avif" || compatible.clone().any(|b| b == b"avif") {
        Ok(())
    } else {
        Err(BAD_FORMAT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const AVIF: &[u8] = include_bytes!("fixtures/blocks.avif");
    const WEBP: &[u8] = include_bytes!("fixtures/blocks.webp");
    const REMAINING: u32 = 1024 * 1024;

    #[test]
    fn accepts_real_static_avif_and_webp_bitstreams_and_outputs_clean_png() {
        for (input, size, thumb) in [
            (AVIF, (64, 64), (64, 64)),
            (include_bytes!("fixtures/blocks10.avif"), (64, 64), (64, 64)),
            (
                include_bytes!("fixtures/max-side.avif"),
                (1024, 768),
                (160, 120),
            ),
            (
                include_bytes!("fixtures/blocks-wide.avif"),
                (1040, 760),
                (160, 116),
            ),
            (WEBP, (64, 64), (64, 64)),
            (
                include_bytes!("fixtures/blocks-wide.webp"),
                (1040, 760),
                (160, 116),
            ),
            (
                include_bytes!("fixtures/blocks-lossless.webp"),
                (64, 64),
                (64, 64),
            ),
        ] {
            let (normalized, stored, decoded) =
                decode(input, REMAINING).unwrap();
            assert_eq!(stored, thumb.0 * thumb.1);
            assert_eq!(decoded, size.0 * size.1);
            let mut png = png::Decoder::new(Cursor::new(normalized))
                .read_info()
                .unwrap();
            assert_eq!((png.info().width, png.info().height), thumb);
            let mut pixels = vec![0; png.output_buffer_size().unwrap()];
            let frame = png.next_frame(&mut pixels).unwrap();
            png.finish().unwrap();
            assert!(png.info().exif_metadata.is_none());
            assert!(png.info().icc_profile.is_none());
            let channels = frame.color_type.samples();
            let sample =
                |x, y| &pixels[((y * thumb.0 + x) as usize) * channels..][..3];
            let red = sample(thumb.0 / 4, thumb.1 / 4);
            let blue = sample(thumb.0 / 4, thumb.1 * 3 / 4);
            assert!(
                red[0] > 150 && red[2] < 120 && blue[2] > 150 && blue[0] < 120
            );
        }
    }

    #[test]
    fn refuses_truncated_animated_wrong_brand_and_out_of_budget_inputs() {
        let mut heic = AVIF.to_vec();
        // FFmpeg's sample declares "avif" once in each of the major and
        // compatible lists, and the internal AV1 bytes are otherwise intact.
        for brand in heic[8..32].chunks_exact_mut(4) {
            if brand == b"avif" {
                brand.copy_from_slice(b"heic");
            }
        }
        assert_eq!(decode(&heic, REMAINING), Err(BAD_FORMAT));
        for animated in [
            include_bytes!("fixtures/blocks-animated.avif").as_slice(),
            include_bytes!("fixtures/blocks-animated.webp").as_slice(),
        ] {
            assert_eq!(decode(animated, REMAINING), Err(ANIMATED));
        }
        for invalid in [
            &AVIF[..AVIF.len() - 1],
            &WEBP[..WEBP.len() - 1],
            include_bytes!("fixtures/too-wide.avif"),
            include_bytes!("fixtures/too-wide.webp"),
            include_bytes!("fixtures/over-area.webp"),
        ] {
            assert!(decode(invalid, REMAINING).is_err());
        }
        for valid in [AVIF, WEBP] {
            assert!(decode(valid, 64 * 64 - 1)
                .unwrap_err()
                .contains("megapixels"));
        }
    }

    #[test]
    fn rejects_animation_even_with_a_separate_fallback_png_or_without_avis_brand(
    ) {
        let mut apng = Vec::new();
        let mut encoder = png::Encoder::new(&mut apng, 1, 1);
        encoder.set_animated(1, 0).unwrap();
        encoder.set_sep_def_img(true).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[17]).unwrap(); // separate, non-animated fallback
        writer.write_image_data(&[51]).unwrap();
        writer.finish().unwrap();
        assert!(decode(&apng, REMAINING).is_err());

        let mut sequence =
            include_bytes!("fixtures/blocks-animated.avif").to_vec();
        let ftyp_size =
            u32::from_be_bytes(sequence[..4].try_into().unwrap()) as usize;
        for brand in sequence[8..ftyp_size].chunks_exact_mut(4) {
            if brand == b"avis" {
                brand.copy_from_slice(b"avif");
            }
        }
        assert!(still_avif_brand(&sequence).is_ok());
        assert!(decode(&sequence, REMAINING).is_err());
    }

    #[test]
    fn avif_does_not_silently_render_hdr_as_standard_sdr_png() {
        let hdr = include_bytes!("fixtures/blocks-hdr.avif");
        assert_eq!(decode(hdr, REMAINING), Err(BAD_AVIF_COLOR));

        // Leave the valid SDR AV1 bitstream unchanged. Test a container-only
        // HDR declaration, then an embedded-profile declaration.
        let nclx = AVIF.windows(8).position(|tag| tag == b"colrnclx").unwrap();
        let mut container_hdr = AVIF.to_vec();
        container_hdr[nclx + 8..nclx + 14]
            .copy_from_slice(&[0, 9, 0, 16, 0, 9]);
        assert_eq!(decode(&container_hdr, REMAINING), Err(BAD_AVIF_COLOR));
        let mut with_icc = AVIF.to_vec();
        with_icc[nclx + 4..nclx + 8].copy_from_slice(b"prof");
        assert_eq!(decode(&with_icc, REMAINING), Err(BAD_AVIF_COLOR));
    }

    #[test]
    fn webp_bounds_lossless_entropy_and_keeps_compressed_alpha() {
        let encoded = include_bytes!("fixtures/blocks-entropy.webp");
        let header = webp_chunks(encoded).unwrap();
        let (w, h) = header.dimensions;
        let lossless = &header.lossless.unwrap()[5..];
        let group_count = lossless_webp_group_count(lossless, w, h).unwrap();
        assert!((2..=WEBP_GROUP_LIMIT).contains(&group_count));
        assert!(decode(encoded, REMAINING).is_ok());

        let alpha = include_bytes!("fixtures/blocks-alpha.webp");
        let header = webp_chunks(alpha).unwrap();
        assert_eq!(header.alpha.unwrap()[0] & 3, 1);
        let (normalized, _, _) = decode(alpha, REMAINING).unwrap();
        let mut png = png::Decoder::new(Cursor::new(normalized))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; png.output_buffer_size().unwrap()];
        let frame = png.next_frame(&mut pixels).unwrap();
        assert_eq!(frame.color_type, png::ColorType::Rgba);
        assert_eq!(pixels[3], 128);
        assert_eq!(pixels[63 * 4 + 3], 255);
    }

    #[test]
    fn webp_validates_coded_canvas_and_exif_before_losing_metadata() {
        fn extended(width: u8, orientation: u8) -> Vec<u8> {
            let mut out = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0".to_vec();
            out.extend([8, 0, 0, 0, width - 1, 0, 0, 63, 0, 0]); // EXIF flag
            out.extend(&WEBP[12..]);
            let mut exif =
                b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0"
                    .to_vec();
            exif.extend([orientation, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(b"EXIF");
            out.extend((exif.len() as u32).to_le_bytes());
            out.extend(exif);
            let size = (out.len() - 8) as u32;
            out[4..8].copy_from_slice(&size.to_le_bytes());
            out
        }
        assert!(decode(&extended(64, 1), REMAINING).is_ok());
        assert_eq!(decode(&extended(32, 1), REMAINING), Err(BAD_WEBP));
        assert_eq!(decode(&extended(96, 1), REMAINING), Err(BAD_WEBP));
        assert!(decode(&extended(64, 6), REMAINING)
            .unwrap_err()
            .contains("EXIF"));
    }
}
