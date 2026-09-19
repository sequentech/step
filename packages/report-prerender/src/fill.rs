// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{stream::PdfStream, Field, Prepared, VERSION};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use serde_json::Value;
use std::fmt::Write as FmtWrite;
use std::io::{Seek, Write};
use ttf_parser::{Face, OutlineBuilder};

const MAX_BACKGROUND: usize = 24_000_000;
const MAX_RECORDS: usize = 100_000;
fn inherited(doc: &Document, mut id: ObjectId, key: &[u8]) -> Result<Object> {
    for _ in 0..100 {
        let d = doc.get_dictionary(id)?;
        if let Ok(v) = d.get(key) {
            return Ok(v.clone());
        }
        id = d.get(b"Parent")?.as_reference()?;
    }
    bail!("PDF page inheritance cycle")
}
fn resolved<'a>(doc: &'a Document, value: &'a Object) -> Result<&'a Object> {
    match value {
        Object::Reference(id) => Ok(doc.get_object(*id)?),
        _ => Ok(value),
    }
}

type PageData = (u32, f32, f32, Dictionary, Vec<u8>);

fn valid_pointer(path: &str) -> bool {
    if !path.starts_with('/') || path.len() > 512 {
        return false;
    }
    let mut chars = path.chars();
    while let Some(character) = chars.next() {
        if character == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}

/// Check the immutable artifact before publishing it as ready. Runtime values
/// are intentionally absent: their type, font and overflow checks happen at fill.
pub fn validate_background(background: &[u8], prepared: &Prepared) -> Result<()> {
    load_background(background, prepared).map(|_| ())
}

fn load_background(background: &[u8], prepared: &Prepared) -> Result<(Document, Vec<PageData>)> {
    if prepared.version != VERSION || background.len() > MAX_BACKGROUND {
        bail!("Unsupported manifest or oversized background");
    }
    let doc = Document::load_mem(background).context("Invalid background PDF")?;
    if doc.is_encrypted() {
        bail!("Cache backgrounds must not be encrypted");
    }
    if prepared.fields.len() > 20_000 {
        bail!("Too many runtime fields");
    }
    let mut conditional_pages = std::collections::BTreeSet::new();
    for field in &prepared.fields {
        if !matches!(
            field.kind.as_str(),
            "text" | "qr" | "mark" | "image" | "page"
        ) {
            bail!("Unknown runtime field kind {}", field.kind);
        }
        if !valid_pointer(&field.path) {
            bail!("Invalid runtime field JSON pointer {}", field.path);
        }
        if field.kind == "page" && !conditional_pages.insert(field.page) {
            bail!("Only one page condition is allowed per page");
        }
    }
    let pages = doc.get_pages();
    if pages.is_empty() || pages.len() > 1000 {
        bail!("Background must contain 1–1000 pages");
    }
    let mut page_data = Vec::new();
    for (number, id) in pages {
        let media = inherited(&doc, id, b"MediaBox")?;
        let rect = resolved(&doc, &media)?.as_array()?;
        let number_at = |index: usize| -> Result<f32> {
            Ok(rect
                .get(index)
                .ok_or_else(|| anyhow!("Invalid MediaBox"))?
                .as_float()?)
        };
        let width = number_at(2)?;
        let height = number_at(3)?;
        if rect.len() != 4
            || number_at(0)? != 0.0
            || number_at(1)? != 0.0
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
        {
            bail!("Pre-render requires a zero-origin, positive MediaBox");
        }
        if inherited(&doc, id, b"Rotate")
            .ok()
            .and_then(|v| v.as_i64().ok())
            .unwrap_or(0)
            != 0
        {
            bail!("Rotated background pages are unsupported");
        }
        for f in prepared.fields.iter().filter(|f| f.page == number) {
            if ![f.x, f.y, f.width, f.height, f.font_size]
                .iter()
                .all(|v| v.is_finite())
                || f.x < 0.0
                || f.y < 0.0
                || f.width <= 0.0
                || f.height <= 0.0
                || !(1.0..=200.0).contains(&f.font_size)
                || f.x + f.width > width + 0.01
                || f.y + f.height > height + 0.01
            {
                bail!("Field {} extends outside its page", f.path);
            }
        }
        let resources = inherited(&doc, id, b"Resources")?;
        let resources = resolved(&doc, &resources)?.as_dict()?.clone();
        // A q/Q pair around the old contents isolates its graphics state.
        let content = doc.get_page_content(id)?;
        page_data.push((number, width, height, resources, content));
    }
    if prepared
        .fields
        .iter()
        .any(|f| !page_data.iter().any(|(number, ..)| *number == f.page))
    {
        bail!("Field refers to a missing page");
    }
    Ok((doc, page_data))
}

/// The PDF background is parsed once. Each input record and overlay is dropped
/// before the next. RAM is O(background + one record + page IDs/xref offsets).
/// The caller must publish the temporary output only after this succeeds.
pub fn fill_many<W, I>(
    background: &[u8],
    prepared: &Prepared,
    font: &[u8],
    records: I,
    output: W,
) -> Result<W>
where
    W: Write + Seek,
    I: IntoIterator<Item = Result<Value>>,
{
    let (doc, page_data) = load_background(background, prepared)?;
    let face = Face::parse(font, 0).context("Invalid runtime font")?;
    let shaped_face =
        rustybuzz::Face::from_slice(font, 0).ok_or_else(|| anyhow!("Invalid shaping font"))?;
    let mut pdf = PdfStream::new(output, doc.max_id)?;
    // Copy immutable fonts/images once, preserving original object references.
    for (id, object) in &doc.objects {
        pdf.object(*id, object)?;
    }
    let root = pdf.allocate();
    let tree = pdf.allocate();
    let extraction_font = pdf.allocate();
    pdf.object(
        extraction_font,
        &Object::Dictionary(
            dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" },
        ),
    )?;
    let mut backgrounds = Vec::new();
    for (_, width, height, resources, content) in &page_data {
        let id = pdf.allocate();
        let mut form = Stream::new(
            dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), (*width).into(), (*height).into()], "Resources" => resources.clone() },
            content.clone(),
        );
        form.compress()?;
        pdf.object(id, &Object::Stream(form))?;
        backgrounds.push(id);
    }
    let mut children: Vec<Object> = Vec::new();
    for (index, data) in records.into_iter().enumerate() {
        if index >= MAX_RECORDS {
            bail!("Batch exceeds 100000 records");
        }
        let data = data?;
        if data.to_string().len() > 8_000_000 {
            bail!("Runtime record exceeds 8 MB");
        }
        for (page_index, (number, width, height, _, _)) in page_data.iter().enumerate() {
            if let Some(condition) = prepared
                .fields
                .iter()
                .find(|f| f.page == *number && f.kind == "page")
            {
                let enabled = data
                    .pointer(&condition.path)
                    .and_then(Value::as_bool)
                    .ok_or_else(|| anyhow!("Missing boolean page condition {}", condition.path))?;
                if !enabled {
                    continue;
                }
            }
            let mut content = String::from("q /Background Do Q\n");
            let mut xobjects = dictionary! { "Background" => backgrounds[page_index] };
            for (field_index, field) in prepared
                .fields
                .iter()
                .enumerate()
                .filter(|(_, f)| f.page == *number && f.kind != "page")
            {
                let value = data
                    .pointer(&field.path)
                    .ok_or_else(|| anyhow!("Missing runtime field {}", field.path))?;
                if field.kind == "image" {
                    image(
                        &mut pdf,
                        &mut xobjects,
                        &mut content,
                        field_index,
                        field,
                        value,
                        *height,
                    )?;
                } else {
                    draw(&mut content, field, value, &face, &shaped_face, *height)?;
                }
            }
            let content_id = pdf.allocate();
            let mut stream = Stream::new(Dictionary::new(), content.into_bytes());
            stream.compress()?;
            pdf.object(content_id, &Object::Stream(stream))?;
            let page_id = pdf.allocate();
            pdf.object(page_id, &Object::Dictionary(dictionary! { "Type" => "Page", "Parent" => tree, "MediaBox" => vec![0.into(), 0.into(), (*width).into(), (*height).into()], "Resources" => dictionary! { "XObject" => xobjects, "Font" => dictionary! { "Extraction" => extraction_font } }, "Contents" => content_id }))?;
            children.push(page_id.into());
        }
    }
    if children.is_empty() {
        bail!("At least one runtime record is required");
    }
    pdf.object(
        tree,
        &Object::Dictionary(
            dictionary! { "Type" => "Pages", "Count" => children.len() as i64, "Kids" => children },
        ),
    )?;
    pdf.object(
        root,
        &Object::Dictionary(dictionary! { "Type" => "Catalog", "Pages" => tree }),
    )?;
    pdf.finish(root)
}

fn text(value: &Value) -> Result<String> {
    match value {
        Value::String(v) => Ok(v.clone()),
        Value::Number(v) => Ok(v.to_string()),
        _ => bail!("Text/QR fields require a string or number"),
    }
}
fn draw(
    output: &mut String,
    field: &Field,
    value: &Value,
    face: &Face<'_>,
    shaped_face: &rustybuzz::Face<'_>,
    page_height: f32,
) -> Result<()> {
    let x = field.x;
    let y = page_height - field.y - field.height;
    writeln!(
        output,
        "q 0 g {x} {y} {} {} re W n",
        field.width, field.height
    )?;
    match field.kind.as_str() {
        "mark" => {
            let selected = value
                .as_bool()
                .ok_or_else(|| anyhow!("Ballot marks require an explicit boolean"))?;
            if selected {
                // A solid oval is the conventional optical-scan ballot mark.
                let rx = field.width / 2.0;
                let ry = field.height / 2.0;
                let cx = x + rx;
                let cy = y + ry;
                let k = 0.5522848;
                writeln!(output, "{} {cy} m {} {} {} {} {cx} {} c {} {} {} {} {} {cy} c {} {} {} {} {cx} {} c {} {} {} {} {} {cy} c f", cx+rx, cx+rx, cy+ry*k, cx+rx*k, cy+ry, cy+ry, cx-rx*k, cy+ry, cx-rx, cy+ry*k, cx-rx, cx-rx, cy-ry*k, cx-rx*k, cy-ry, cy-ry, cx+rx*k, cy-ry, cx+rx, cy-ry*k, cx+rx)?;
            }
        }
        "qr" => {
            let value = text(value)?;
            if value.len() > 2000 {
                bail!("QR content exceeds 2000 bytes");
            }
            let qr =
                qrcode::QrCode::with_error_correction_level(value.as_bytes(), qrcode::EcLevel::M)?;
            let size = qr.width();
            let cell = field.width.min(field.height) / (size + 8) as f32;
            writeln!(
                output,
                "1 g {x} {y} {} {} re f 0 g",
                field.width, field.height
            )?;
            for row in 0..size {
                for col in 0..size {
                    if qr[(col, row)] == qrcode::Color::Dark {
                        writeln!(
                            output,
                            "{} {} {cell} {cell} re f",
                            x + (col + 4) as f32 * cell,
                            y + (size + 3 - row) as f32 * cell
                        )?;
                    }
                }
            }
        }
        "text" => {
            let value = text(value)?;
            if value.len() > 4096 || value.contains(['\n', '\r']) {
                bail!("Text fields are single-line and limited to 4096 bytes");
            }
            let scale = field.font_size / face.units_per_em() as f32;
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(&value);
            buffer.guess_segment_properties();
            let shaped = rustybuzz::shape(shaped_face, &[], buffer);
            let mut glyphs = Vec::new();
            let mut advance = 0.0;
            for (info, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                if info.glyph_id == 0 {
                    bail!("Runtime font cannot display part of field {}", field.path);
                }
                glyphs.push((
                    ttf_parser::GlyphId(info.glyph_id as u16),
                    advance + position.x_offset as f32 * scale,
                    position.y_offset as f32 * scale,
                ));
                advance += position.x_advance as f32 * scale;
            }
            if advance > field.width
                || (face.ascender() - face.descender()) as f32 * scale > field.height
            {
                bail!("Runtime text overflows field {}", field.path);
            }
            // Outlines preserve Unicode glyphs without viewer font substitution;
            // ActualText retains the original text for copying/accessibility.
            output.push_str("/Span << /ActualText <FEFF");
            for unit in value.encode_utf16() {
                write!(output, "{unit:04X}")?;
            }
            output.push_str("> >> BDC\n");
            let baseline = page_height - field.y - face.ascender() as f32 * scale;
            for (glyph, advance, offset_y) in glyphs {
                writeln!(
                    output,
                    "q {scale} 0 0 {scale} {} {} cm",
                    x + advance,
                    baseline + offset_y
                )?;
                let mut path = Outline(String::new(), (0.0, 0.0));
                if face.outline_glyph(glyph, &mut path).is_some() {
                    output.push_str(&path.0);
                    output.push_str("f\n");
                }
                output.push_str("Q\n");
            }
            // Text extractors require a text-showing operator inside ActualText.
            // This invisible glyph supplies its bounds; the outlines above are
            // the only visible rendering and retain the shaped runtime font.
            if !value.is_empty() {
                writeln!(
                    output,
                    "BT /Extraction {} Tf 3 Tr {} Tz 1 0 0 1 {x} {baseline} Tm (x) Tj ET",
                    field.font_size,
                    advance / (field.font_size * 0.5) * 100.0
                )?;
            }
            output.push_str("EMC\n");
        }
        _ => bail!("Unknown runtime field kind"),
    }
    output.push_str("Q\n");
    Ok(())
}
struct Outline(String, (f32, f32));
impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        let _ = writeln!(self.0, "{x} {y} m");
        self.1 = (x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let _ = writeln!(self.0, "{x} {y} l");
        self.1 = (x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x0, y0) = self.1;
        self.curve_to(
            x0 + (x1 - x0) * 2.0 / 3.0,
            y0 + (y1 - y0) * 2.0 / 3.0,
            x + (x1 - x) * 2.0 / 3.0,
            y + (y1 - y) * 2.0 / 3.0,
            x,
            y,
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let _ = writeln!(self.0, "{x1} {y1} {x2} {y2} {x} {y} c");
        self.1 = (x, y);
    }
    fn close(&mut self) {
        self.0.push_str("h\n");
    }
}
fn image<W: Write + Seek>(
    pdf: &mut PdfStream<W>,
    objects: &mut Dictionary,
    output: &mut String,
    index: usize,
    field: &Field,
    value: &Value,
    page_height: f32,
) -> Result<()> {
    let encoded = value
        .as_str()
        .ok_or_else(|| anyhow!("Image requires base64 PNG/JPEG"))?;
    if encoded.len() > 4_000_000 {
        bail!("Runtime image exceeds 3 MB");
    }
    let bytes = STANDARD.decode(encoded)?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(32_000_000);
    reader.limits(limits);
    let rgba = reader.decode()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for pixel in rgba.pixels() {
        for c in &pixel.0[..3] {
            rgb.push(
                ((*c as u32 * pixel.0[3] as u32 + 255 * (255 - pixel.0[3] as u32)) / 255) as u8,
            );
        }
    }
    let id = pdf.allocate();
    let mut stream = Stream::new(
        dictionary! {"Type"=>"XObject","Subtype"=>"Image","Width"=>width as i64,"Height"=>height as i64,"ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8},
        rgb,
    );
    stream.compress()?;
    pdf.object(id, &Object::Stream(stream))?;
    let name = format!("RuntimeImage{index}");
    objects.set(name.as_str(), id);
    writeln!(
        output,
        "q {} 0 0 {} {} {} cm /{name} Do Q",
        field.width,
        field.height,
        field.x,
        page_height - field.y - field.height
    )?;
    Ok(())
}
