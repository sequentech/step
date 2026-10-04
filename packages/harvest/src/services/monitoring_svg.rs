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
//!
//! The engine writes every figure as `1,234.5`; [`localize_figures`] then
//! writes a kept chart's figures in its election event's number format.

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::{Reader, Writer};
use sequent_core::types::number_format::NumberFormatPolicy;
use std::borrow::Cow;
use tracing::warn;

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

/// `svg`, a chart [`sanitize_svg`] kept, with its figures in `policy`; as
/// drawn, with a warning, if they cannot be rewritten, so that the chart is
/// still shown. `affixes` is the text the board writes beside its figures
/// ([`figure_affixes`](sequent_core::monitoring::render_request::figure_affixes)).
pub fn localize_figures_or_keep(
    svg: String,
    policy: NumberFormatPolicy,
    affixes: &[String],
) -> String {
    match localize_figures(&svg, policy, affixes) {
        Ok(localized) => localized,
        Err(error) => {
            warn!("A chart's figures stay as drawn: {error:?}");
            svg
        }
    }
}

/// The elements a chart writes its text in.
const TEXT_ELEMENTS: &[&str] = &["text", "tspan", "textPath"];

/// What the engine writes after a figure's digits: a percent sign, the
/// native formats' points, and d3's SI prefixes in its own register (`1.5M`)
/// and in the analytic (`1.5 M`) and narrative (`1.5mn`) ones.
const ENGINE_UNITS: &[&str] = &[
    "%", " pts", "y", "z", "a", "f", "p", "n", "\u{b5}", "m", "k", "M", "G",
    "T", "P", "E", "Z", "Y", " K", " M", " B", " T", " P", " E", " Z", " Y",
    "mn", "bn", "trn",
];

/// The signs the engine writes before a figure: `−` for a negative one.
const SIGNS: [char; 3] = ['+', '-', '\u{2212}'];

/// `svg`, a chart [`sanitize_svg`] kept, with its figures written in
/// `policy`. dbt Charts writes each figure in the format of `1,234.5`, in a
/// run of text of its own: axis ticks, bar labels, KPI values, table cells,
/// and the figure of a KPI's supporting line, whose words are the next run.
/// A run is rewritten when it is one figure as the engine writes it, with
/// the board's own `affixes` around it, and nothing else; then only its
/// digits' separators change. Labels such as `18-24`, `10:00` or `May'26`
/// stay as drawn.
pub fn localize_figures(
    svg: &str,
    policy: NumberFormatPolicy,
    affixes: &[String],
) -> Result<String, UnsafeSvg> {
    if policy == NumberFormatPolicy::CommaPeriod {
        return Ok(svg.to_string());
    }
    let mut reader = Reader::from_str(svg);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Vec::with_capacity(svg.len()));
    // Whether each open element is one a chart writes its text in.
    let mut open: Vec<bool> = Vec::new();
    loop {
        let event = reader.read_event().map_err(|_| UnsafeSvg::Malformed)?;
        let event = match event {
            Event::Eof => break,
            Event::Start(start) => {
                open.push(
                    TEXT_ELEMENTS.contains(&element_name(&start)?.as_str()),
                );
                Event::Start(start)
            }
            Event::End(end) => {
                open.pop();
                Event::End(end)
            }
            Event::Text(text) if open.last() == Some(&true) => {
                let raw = std::str::from_utf8(text.as_ref())
                    .map_err(|_| UnsafeSvg::Malformed)?;
                match localized_figure(&unescape(raw)?, policy, affixes) {
                    Some(figure) => {
                        Event::Text(BytesText::new(&figure).into_owned())
                    }
                    None => Event::Text(text),
                }
            }
            other => other,
        };
        writer
            .write_event(event)
            .map_err(|_| UnsafeSvg::Malformed)?;
    }
    String::from_utf8(writer.into_inner()).map_err(|_| UnsafeSvg::Malformed)
}

/// `text` without the longest of `options` it starts with, if any.
fn strip_longest_prefix<'a, 'b>(
    text: &'a str,
    options: impl IntoIterator<Item = &'b str>,
) -> &'a str {
    options
        .into_iter()
        .filter(|option| !option.is_empty())
        .filter_map(|option| text.strip_prefix(option))
        .min_by_key(|rest| rest.len())
        .unwrap_or(text)
}

/// `text` without the longest of `options` it ends with, if any.
fn strip_longest_suffix<'a, 'b>(
    text: &'a str,
    options: impl IntoIterator<Item = &'b str>,
) -> &'a str {
    options
        .into_iter()
        .filter(|option| !option.is_empty())
        .filter_map(|option| text.strip_suffix(option))
        .min_by_key(|rest| rest.len())
        .unwrap_or(text)
}

/// `text` in `policy` when it is a figure as the engine writes one, with
/// the space around it: a sign; a currency sign or one of the board's
/// `affixes`, before or after the sign; digits grouped in thousands with
/// commas, with an optional fraction after a period; then an engine unit,
/// one of the `affixes`, both or neither, as in `−1,234`, `53.2%`, `20.6 M`,
/// `$1,234.57` or `≈1,234 votes`. Only the digits' separators change, never
/// an affix's.
fn localized_figure(
    text: &str,
    policy: NumberFormatPolicy,
    affixes: &[String],
) -> Option<String> {
    let figure = text.trim();
    let affixes = || affixes.iter().map(String::as_str);
    let unsigned = figure.strip_prefix(SIGNS);
    let after_prefix = strip_longest_prefix(
        unsigned.unwrap_or(figure),
        std::iter::once("$").chain(affixes()),
    );
    let digits_start = match unsigned {
        Some(_) => after_prefix,
        None => after_prefix.strip_prefix(SIGNS).unwrap_or(after_prefix),
    };
    let number = strip_longest_suffix(
        strip_longest_suffix(digits_start, affixes()),
        ENGINE_UNITS.iter().copied(),
    );
    let (integer, fraction) = match number.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (number, None),
    };
    let digits = |part: &str| {
        !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())
    };
    let mut groups = integer.split(',');
    let first = groups.next().unwrap_or_default();
    let grouped = integer.contains(',');
    let is_figure = digits(first)
        && (!grouped || first.len() <= 3)
        && groups.all(|group| group.len() == 3 && digits(group))
        && fraction.map_or(true, digits);
    if !is_figure || !(grouped || fraction.is_some()) {
        return None;
    }
    let mut localized = figure[..figure.len() - digits_start.len()].to_string();
    for c in number.chars() {
        match c {
            ',' => localized.push_str(policy.group_separator()),
            '.' => localized.push_str(policy.decimal_separator()),
            digit => localized.push(digit),
        }
    }
    localized.push_str(&digits_start[number.len()..]);
    Some(text.replacen(figure, &localized, 1))
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
        let value = unescape(
            std::str::from_utf8(&attribute.value)
                .map_err(|_| UnsafeSvg::Malformed)?,
        )?;
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

/// An attribute value as written, with XML's five entities and character
/// references replaced; any other entity is not XML.
fn unescape(raw: &str) -> Result<String, UnsafeSvg> {
    let mut value = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        value.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let end = after.find(';').ok_or(UnsafeSvg::Malformed)?;
        let entity = &after[..end];
        let c = match entity {
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    u32::from_str_radix(hex, 16)
                } else if let Some(decimal) = entity.strip_prefix('#') {
                    decimal.parse::<u32>()
                } else {
                    return Err(UnsafeSvg::Malformed);
                };
                code.ok()
                    .and_then(char::from_u32)
                    .ok_or(UnsafeSvg::Malformed)?
            }
        };
        value.push(c);
        rest = &after[end + 1..];
    }
    value.push_str(rest);
    Ok(value)
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
