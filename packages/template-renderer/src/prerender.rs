// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Two-stage, fixed-layout PDF templates. Coordinates are PDF points from the
//! top-left of a page. Runtime fields never participate in HTML pagination.
use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderErrorReason,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub const VERSION: u32 = 1;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub kind: String,
    /// A JSON pointer; stable candidate IDs work without array-position coupling.
    pub path: String,
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prepared {
    pub version: u32,
    pub html: String,
    pub fields: Vec<Field>,
}

/// Expand known election data without evaluating any runtime expression, even
/// when a runtime pointer contains a pre-stage expression inside its quotes.
pub fn expand_known(source: &str, known: &Value) -> Result<String, String> {
    const OPEN: &str = "\u{e000}STUDIO_OPEN\u{e000}";
    const CLOSE: &str = "\u{e000}STUDIO_CLOSE\u{e000}";
    if source.contains(OPEN) || source.contains(CLOSE) {
        return Err("Reserved stage marker".into());
    }
    let staged = source
        .replace("{{", OPEN)
        .replace("}}", CLOSE)
        .replace("[[", "{{")
        .replace("]]", "}}");
    let mut registry = crate::helpers::get_registry();
    registry.set_strict_mode(true);
    fn number(value: f64) -> Value {
        if value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_991.0 {
            serde_json::json!(value as i64)
        } else {
            serde_json::json!(value)
        }
    }
    handlebars::handlebars_helper!(add: |a: f64, b: f64| number(a+b));
    handlebars::handlebars_helper!(mul: |a: f64, b: f64| number(a*b));
    registry.register_helper("add", Box::new(add));
    registry.register_helper("mul", Box::new(mul));
    let expanded = crate::bounded_render(
        &registry,
        &staged,
        known.as_object().ok_or("Known data must be an object")?,
    )?;
    // Braces arriving through known data are literal text, never second-stage code.
    Ok(expanded
        .replace("{{", "&#123;&#123;")
        .replace("}}", "&#125;&#125;")
        .replace(OPEN, "{{")
        .replace(CLOSE, "}}"))
}

pub fn prepare(source: &str, known: &Value) -> Result<Prepared, String> {
    let staged = expand_known(source, known)?;
    let fields = Arc::new(Mutex::new(Vec::new()));
    let mut registry = crate::helpers::get_registry();
    registry.set_strict_mode(true);
    // Runtime conditions/loops could change layout; require these in [[ ]].
    let expression = regex::Regex::new(r"\{\{\{?\s*([^\s}]+)").unwrap();
    for capture in expression.captures_iter(&staged) {
        if !matches!(
            &capture[1],
            "pdf_text"
                | "pdf_qr"
                | "pdf_mark"
                | "pdf_image"
                | "pdf_page"
                | "studio_translate"
                | "!"
                | "!--"
        ) {
            return Err(format!("Runtime expression '{}' changes layout. Use [[ ]] for known data, or a pdf_text/pdf_qr/pdf_mark/pdf_image field", &capture[1]));
        }
    }
    for kind in ["text", "qr", "mark", "image", "page"] {
        let collected = fields.clone();
        registry.register_helper(
            &format!("pdf_{kind}"),
            Box::new(
                move |h: &Helper<'_>,
                      _: &Handlebars<'_>,
                      _: &Context,
                      _: &mut RenderContext<'_, '_>,
                      _: &mut dyn Output|
                      -> HelperResult {
                    let fail = |message: &str| RenderErrorReason::Other(message.into());
                    if h.params().len() != 1
                        || h.hash().keys().any(|key| {
                            !["page", "x", "y", "width", "height", "size"].contains(&key.as_ref())
                        })
                    {
                        return Err(fail(
                            "PDF fields accept one pointer and page/x/y/width/height/size options",
                        )
                        .into());
                    }
                    let path = h
                        .param(0)
                        .and_then(|p| p.value().as_str())
                        .ok_or_else(|| fail("PDF field requires a quoted JSON pointer"))?;
                    if !path.starts_with('/') || path.len() > 512 {
                        return Err(
                            fail("PDF field path must be a JSON pointer starting with /").into(),
                        );
                    }
                    let number = |name: &str,
                                  default: Option<f32>|
                     -> Result<f32, handlebars::RenderError> {
                        let value = match h.hash_get(name) {
                            Some(v) => v
                                .value()
                                .as_f64()
                                .or_else(|| v.value().as_str().and_then(|s| s.parse::<f64>().ok()))
                                .map(|v| v as f32),
                            None => default,
                        }
                        .ok_or_else(|| fail(&format!("PDF field requires numeric {name}")))?;
                        if !value.is_finite() {
                            return Err(fail("PDF coordinates must be finite").into());
                        }
                        Ok(value)
                    };
                    let page = number("page", Some(1.0))?;
                    let field = Field {
                        kind: kind.into(),
                        path: path.into(),
                        page: page as u32,
                        x: number("x", if kind == "page" { Some(0.0) } else { None })?,
                        y: number("y", if kind == "page" { Some(0.0) } else { None })?,
                        width: number("width", if kind == "page" { Some(1.0) } else { None })?,
                        height: number("height", if kind == "page" { Some(1.0) } else { None })?,
                        font_size: number("size", Some(12.0))?,
                    };
                    if page.fract() != 0.0
                        || !(1.0..=1000.0).contains(&page)
                        || field.x < 0.0
                        || field.y < 0.0
                        || field.width <= 0.0
                        || field.height <= 0.0
                        || !(1.0..=200.0).contains(&field.font_size)
                    {
                        return Err(fail("Invalid PDF field geometry").into());
                    }
                    let mut output = collected
                        .lock()
                        .map_err(|_| fail("Field collection failed"))?;
                    if output.len() >= 20_000 {
                        return Err(fail("Too many PDF fields").into());
                    }
                    if kind == "page"
                        && output
                            .iter()
                            .any(|f: &Field| f.kind == "page" && f.page == field.page)
                    {
                        return Err(fail("Only one page condition is allowed per page").into());
                    }
                    output.push(field);
                    Ok(())
                },
            ),
        );
    }
    let html = crate::bounded_render(&registry, &staged, &serde_json::Map::new())?;
    let html = regex::Regex::new(r#"data-pdf-page=["']([0-9]+(?:\.0+)?)["']"#)
        .unwrap()
        .replace_all(&html, |capture: &regex::Captures<'_>| {
            format!(
                "data-pdf-page=\"{}\"",
                capture[1].parse::<f64>().unwrap_or(0.0) as u32
            )
        })
        .into_owned();
    let fields = fields.lock().map_err(|e| e.to_string())?.clone();
    for field in fields.iter().filter(|f| f.kind == "page") {
        if !html.contains(&format!("data-pdf-page=\"{}\"", field.page))
            && !html.contains(&format!("data-pdf-page='{}'", field.page))
        {
            return Err(
                "Conditional pages require a data-pdf-page attribute on their HTML page container"
                    .into(),
            );
        }
    }
    Ok(Prepared {
        version: VERSION,
        html,
        fields,
    })
}

/// A standalone HTML download for a document whose final PDF already exists.
/// The PDF entry points recognize it before initializing any browser/transport.
pub fn pdf_document(pdf: &[u8]) -> String {
    use base64::Engine;
    let base64 = base64::engine::general_purpose::STANDARD.encode(pdf);
    format!("<!--SEQUENT_PRE_RENDERED_PDF_V1:{base64}-->\n<!doctype html><html><head><meta charset=\"utf-8\"><title>Report</title></head><body style=\"margin:0\"><object aria-label=\"Printable report\" type=\"application/pdf\" data=\"data:application/pdf;base64,{base64}\" style=\"width:100%;height:100vh\"><p>Open the PDF download to view this report.</p></object></body></html>")
}
pub fn embedded_pdf(html: &str) -> Result<Option<Vec<u8>>, String> {
    use base64::Engine;
    let Some(rest) = html.strip_prefix("<!--SEQUENT_PRE_RENDERED_PDF_V1:") else {
        return Ok(None);
    };
    let (encoded, _) = rest
        .split_once("-->")
        .ok_or("Incomplete pre-rendered PDF")?;
    if encoded.len() > 128_000_000 {
        return Err("Pre-rendered PDF exceeds 96 MB".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"%PDF-") {
        return Err("Invalid pre-rendered PDF".into());
    }
    Ok(Some(bytes))
}

/// Instant editor preview. The PDF filler performs final font/overflow checks;
/// all runtime content here is escaped and remains outside the page layout.
pub fn preview(
    prepared: &Prepared,
    data: &Value,
    page_width: f64,
    page_height: f64,
) -> Result<String, String> {
    use std::fmt::Write;
    if !(18.0..=7200.0).contains(&page_width) || !(18.0..=7200.0).contains(&page_height) {
        return Err("Invalid PDF page dimensions".into());
    }
    let mut hidden = std::collections::BTreeSet::new();
    for field in prepared.fields.iter().filter(|f| f.kind == "page") {
        let enabled = data
            .pointer(&field.path)
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("Missing boolean page condition {}", field.path))?;
        if !enabled {
            hidden.insert(field.page);
        }
    }
    let mut overlay=String::from("<div aria-label=\"Runtime fields\" style=\"position:absolute;inset:0;pointer-events:none\">");
    for field in &prepared.fields {
        if field.kind == "page" || hidden.contains(&field.page) {
            continue;
        }
        let value = data
            .pointer(&field.path)
            .ok_or_else(|| format!("Missing runtime field {}", field.path))?;
        let top = field.y as f64
            + (field.page - 1 - hidden.iter().filter(|page| **page < field.page).count() as u32)
                as f64
                * page_height;
        write!(overlay,"<div data-runtime-field=\"{}\" style=\"position:absolute;left:{}pt;top:{top}pt;width:{}pt;height:{}pt;font:{}pt/1.16 'DejaVu Sans',sans-serif;color:#000;overflow:hidden\">",handlebars::html_escape(&field.path),field.x,field.width,field.height,field.font_size).unwrap();
        match field.kind.as_str() {
            "text" => {
                if !value.is_string() && !value.is_number() {
                    return Err(format!(
                        "Text field {} requires text or a number",
                        field.path
                    ));
                }
                let text = value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string());
                overlay.push_str(&handlebars::html_escape(&text));
            }
            "mark" => {
                let selected = value
                    .as_bool()
                    .ok_or_else(|| "Ballot marks require an explicit boolean".to_string())?;
                if selected {
                    overlay.push_str("<svg viewBox=\"0 0 100 100\" preserveAspectRatio=\"none\" width=\"100%\" height=\"100%\"><ellipse cx=\"50\" cy=\"50\" rx=\"50\" ry=\"50\" fill=\"black\"/></svg>");
                }
            }
            "qr" => {
                let text = value.as_str().ok_or("QR field requires text")?;
                if text.len() > 2000 {
                    return Err("QR content exceeds 2000 bytes".into());
                }
                let qr = qrcode::QrCode::with_error_correction_level(
                    text.as_bytes(),
                    qrcode::EcLevel::M,
                )
                .map_err(|e| e.to_string())?;
                let size = qr.width();
                write!(overlay,"<svg role=\"img\" aria-label=\"QR code\" viewBox=\"0 0 {0} {0}\" width=\"100%\" height=\"100%\" shape-rendering=\"crispEdges\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/><path fill=\"black\" d=\"",size+8).unwrap();
                for row in 0..size {
                    for col in 0..size {
                        if qr[(col, row)] == qrcode::Color::Dark {
                            write!(overlay, "M{} {}h1v1h-1z", col + 4, row + 4).unwrap();
                        }
                    }
                }
                overlay.push_str("\"/></svg>");
            }
            "image" => {
                use base64::Engine;
                let encoded = value
                    .as_str()
                    .ok_or("Image field requires base64 PNG/JPEG")?;
                if encoded.len() > 4_000_000 {
                    return Err("Runtime image exceeds 3 MB".into());
                }
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|e| e.to_string())?;
                let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    "image/png"
                } else if bytes.starts_with(b"\xff\xd8\xff") {
                    "image/jpeg"
                } else {
                    return Err("Only PNG/JPEG runtime images are supported".into());
                };
                write!(overlay,"<img alt=\"Runtime image\" style=\"width:100%;height:100%\" src=\"data:{mime};base64,{encoded}\"/>").unwrap();
            }
            _ => return Err("Unknown field kind".into()),
        }
        overlay.push_str("</div>");
    }
    overlay.push_str("</div>");
    let hide_css = hidden
        .iter()
        .map(|page| format!("[data-pdf-page=\"{page}\"]{{display:none!important}}"))
        .collect::<String>();
    let style=format!("<style>{hide_css}html{{margin:0;padding:0}}body{{position:relative;margin:0;padding:0;width:{page_width}pt;min-height:{page_height}pt}}@page{{size:{page_width}pt {page_height}pt;margin:0}}</style>");
    let mut html = prepared.html.clone();
    if html.contains("</head>") {
        html = html.replacen("</head>", &format!("{style}</head>"), 1);
    } else {
        html = format!("{style}{html}");
    }
    if html.contains("</body>") {
        html = html.replacen("</body>", &format!("{overlay}</body>"), 1);
    } else {
        html.push_str(&overlay);
    }
    Ok(html)
}
