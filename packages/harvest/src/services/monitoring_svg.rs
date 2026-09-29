// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What Harvest lets through of a chart the renderer drew.
//!
//! The renderer checks its own output, and the Admin Portal draws a chart in
//! a sandboxed frame; this is the check in between, so no one of the three
//! is trusted alone. It rebuilds the document from an allowlist of SVG
//! elements and drops, with its content, anything else: scripts, foreign
//! objects, images, links. It drops event handlers, any link that is not to
//! a fragment of the document itself, any `url()` that is not one, and any
//! style sheet that imports or fetches. Declarations, doctypes (and so their
//! entities), comments and processing instructions go too.

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::{Reader, Writer};
use std::borrow::Cow;

/// Why a drawn chart is not shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsafeSvg {
    /// The document is not one `<svg>` element.
    NotSvg,
    /// The document is not well-formed XML.
    Malformed,
}

/// The SVG elements a chart may use.
const ELEMENTS: &[&str] = &[
    "svg",
    "g",
    "defs",
    "symbol",
    "use",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "text",
    "tspan",
    "textPath",
    "title",
    "desc",
    "metadata",
    "clipPath",
    "mask",
    "pattern",
    "marker",
    "linearGradient",
    "radialGradient",
    "stop",
    "style",
    "filter",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
];

/// `svg` with everything unsafe taken out, or why none of it can be shown.
pub fn sanitize_svg(svg: &str) -> Result<String, UnsafeSvg> {
    let mut reader = Reader::from_str(svg);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Vec::with_capacity(svg.len()));
    // Open elements kept, and the depth of an element being dropped whole.
    let mut open: Vec<String> = Vec::new();
    let mut dropping: usize = 0;
    let mut in_style = false;
    let mut style_text = String::new();
    let mut seen_root = false;
    let mut root_closed = false;

    loop {
        let event = reader.read_event().map_err(|_| UnsafeSvg::Malformed)?;
        match event {
            Event::Eof => break,
            Event::Start(_) | Event::Empty(_) if in_style => {
                return Err(UnsafeSvg::Malformed);
            }
            Event::Start(start) => {
                if dropping > 0 {
                    dropping += 1;
                    continue;
                }
                let name = element_name(&start)?;
                if !seen_root {
                    if name != "svg" {
                        return Err(UnsafeSvg::NotSvg);
                    }
                    seen_root = true;
                } else if root_closed {
                    return Err(UnsafeSvg::NotSvg);
                }
                if !ELEMENTS.contains(&name.as_str()) {
                    dropping = 1;
                    continue;
                }
                if name == "style" {
                    in_style = true;
                    style_text.clear();
                    open.push(name);
                    // Written once its text is known to be safe.
                    continue;
                }
                writer
                    .write_event(Event::Start(kept(&start)?))
                    .map_err(|_| UnsafeSvg::Malformed)?;
                open.push(name);
            }
            Event::Empty(start) => {
                if dropping > 0 {
                    continue;
                }
                let name = element_name(&start)?;
                if !seen_root {
                    if name != "svg" {
                        return Err(UnsafeSvg::NotSvg);
                    }
                    seen_root = true;
                    root_closed = true;
                } else if root_closed {
                    return Err(UnsafeSvg::NotSvg);
                }
                if !ELEMENTS.contains(&name.as_str()) {
                    continue;
                }
                writer
                    .write_event(Event::Empty(kept(&start)?))
                    .map_err(|_| UnsafeSvg::Malformed)?;
            }
            Event::End(end) => {
                if dropping > 0 {
                    dropping -= 1;
                    continue;
                }
                let name = open.pop().ok_or(UnsafeSvg::Malformed)?;
                if name == "style" {
                    in_style = false;
                    if style_is_safe(&style_text) {
                        let raw = std::mem::take(&mut style_text);
                        for event in [
                            Event::Start(BytesStart::new("style")),
                            Event::Text(BytesText::from_escaped(raw)),
                            Event::End(BytesEnd::new("style")),
                        ] {
                            writer
                                .write_event(event)
                                .map_err(|_| UnsafeSvg::Malformed)?;
                        }
                    }
                } else {
                    writer
                        .write_event(Event::End(end))
                        .map_err(|_| UnsafeSvg::Malformed)?;
                }
                if open.is_empty() {
                    root_closed = true;
                }
            }
            Event::Text(text) => {
                if dropping > 0 || !seen_root || root_closed {
                    if !seen_root
                        && !String::from_utf8_lossy(text.as_ref())
                            .trim()
                            .is_empty()
                    {
                        return Err(UnsafeSvg::NotSvg);
                    }
                    continue;
                }
                if in_style {
                    style_text
                        .push_str(&String::from_utf8_lossy(text.as_ref()));
                    continue;
                }
                writer
                    .write_event(Event::Text(text))
                    .map_err(|_| UnsafeSvg::Malformed)?;
            }
            Event::CData(data) => {
                if dropping > 0 || !seen_root || root_closed {
                    continue;
                }
                if in_style {
                    style_text.push_str("<![CDATA[");
                    style_text
                        .push_str(&String::from_utf8_lossy(data.as_ref()));
                    style_text.push_str("]]>");
                    continue;
                }
                writer
                    .write_event(Event::CData(data))
                    .map_err(|_| UnsafeSvg::Malformed)?;
            }
            // Declarations, doctypes, comments and instructions carry nothing
            // a chart needs.
            Event::Decl(_)
            | Event::DocType(_)
            | Event::Comment(_)
            | Event::PI(_) => {}
        }
    }
    if !seen_root {
        return Err(UnsafeSvg::NotSvg);
    }
    if !open.is_empty() || dropping > 0 {
        return Err(UnsafeSvg::Malformed);
    }
    String::from_utf8(writer.into_inner()).map_err(|_| UnsafeSvg::Malformed)
}

/// The element's name; one with a namespace prefix is none an SVG chart
/// needs, so it is dropped like an unknown element.
fn element_name(start: &BytesStart<'_>) -> Result<String, UnsafeSvg> {
    let name = std::str::from_utf8(start.name().as_ref())
        .map_err(|_| UnsafeSvg::Malformed)?
        .to_string();
    Ok(if name.contains(':') {
        format!("{name}:")
    } else {
        name
    })
}

/// The element with only its safe attributes.
fn kept(start: &BytesStart<'_>) -> Result<BytesStart<'static>, UnsafeSvg> {
    let name = std::str::from_utf8(start.name().as_ref())
        .map_err(|_| UnsafeSvg::Malformed)?
        .to_string();
    let mut element = BytesStart::new(name);
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|_| UnsafeSvg::Malformed)?;
        let key = std::str::from_utf8(attribute.key.as_ref())
            .map_err(|_| UnsafeSvg::Malformed)?
            .to_string();
        let value = attribute
            .unescape_value()
            .map_err(|_| UnsafeSvg::Malformed)?
            .into_owned();
        if !attribute_is_safe(&key, &value) {
            continue;
        }
        element.push_attribute(Attribute {
            key: QName(key.as_bytes()),
            value: Cow::Owned(escape_attribute(&value).into_bytes()),
        });
    }
    Ok(element)
}

fn attribute_is_safe(key: &str, value: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    let local = lower.rsplit(':').next().unwrap_or(&lower);
    if local.starts_with("on") {
        return false;
    }
    if local == "href" || local == "src" {
        return value.trim_start().starts_with('#');
    }
    let squeezed: String = value
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    if squeezed.contains("javascript:") || squeezed.contains("data:text/html") {
        return false;
    }
    if local == "style" {
        return style_is_safe(value);
    }
    urls_are_fragments(value)
}

/// A style sheet that neither imports another nor fetches anything.
fn style_is_safe(css: &str) -> bool {
    let lower = css.to_ascii_lowercase();
    !lower.contains("@import")
        && !lower.contains("expression(")
        && !lower.contains("javascript:")
        && urls_are_fragments(css)
}

/// Whether every `url(...)` in `text` points into the document.
fn urls_are_fragments(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(at) = rest.find("url(") {
        let after = rest[at + 4..]
            .trim_start()
            .trim_start_matches(['"', '\''])
            .trim_start();
        if !after.starts_with('#') {
            return false;
        }
        rest = &rest[at + 4..];
    }
    true
}

/// An attribute value inside double quotes.
fn escape_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_svg.rs"]
mod monitoring_svg_tests;
