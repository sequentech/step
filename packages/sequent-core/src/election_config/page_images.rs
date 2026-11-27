// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Images the platform's pages name by an Internet address, and embedding one.
//!
//! A sign-in page's CSS (a realm's `loginCustomCss`), the admin portal's
//! look-and-feel CSS and an event's `logo_url` can name an image by an
//! Internet address, as an online tenant they were read from does. A site with
//! no Internet then shows an empty space where the image goes, and nothing
//! says why. A `data:` address carries the file itself, so it shows anywhere.
//!
//! [`check`] finds such addresses in a workbook, for the build to warn about:
//! a warning, not a refusal, because an online site loads them fine and a
//! package does not say where it will be applied. [`embed_image`] replaces one
//! address with the file's `data:` address everywhere in a workbook. A text
//! longer than a cell holds is continued in `<column>+` cells when the
//! workbook is written, so the size of an image is not limited by a cell.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

use super::problem::{Code, Problem};
use super::sheet::{Row, Sheet, Workbook};

/// The largest file [`embed_image`] takes. Every page that names it carries
/// it, a third larger, in its text.
pub const MAX_IMAGE_BYTES: usize = 1024 * 1024;

/// What a browser fetches for a text: the address in `url(...)`, the string
/// of an `@import`, or a `src=` attribute. A JSON key ending in `logo_url`
/// is fetched too: the portals put it in an `<img>`.
const FETCHED: [&str; 3] = ["url(", "@import", "src="];
const LOGO_URL: &str = "logo_url";

fn is_internet(address: &str) -> bool {
    let lower = address.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("//")
}

/// The Internet addresses a browser fetches for this text, in order, each
/// once.
pub fn internet_addresses(text: &str) -> Vec<String> {
    // ASCII lowercasing keeps every byte where it was, so an offset in
    // `lower` is the same offset in `text`.
    let lower = text.to_ascii_lowercase();
    let mut found: Vec<String> = Vec::new();
    for prefix in FETCHED {
        let mut from = 0;
        while let Some(at) = lower[from..].find(prefix) {
            let start = from + at;
            from = start + prefix.len();
            // `data-src=` and the like are not fetched.
            if prefix == "src="
                && lower[..start].chars().next_back().is_some_and(|c| {
                    c.is_ascii_alphanumeric() || c == '-' || c == '_'
                })
            {
                continue;
            }
            let mut rest = &text[from..];
            if prefix == "@import" {
                rest = rest.trim_start();
                if rest.to_ascii_lowercase().starts_with("url(") {
                    rest = &rest[4..];
                }
            }
            // Quotes are optional in url(), and in a realm text, which is
            // MessageFormat, a quote is doubled ('').
            let rest = rest.trim_start().trim_start_matches(['"', '\'']);
            let rest = rest.trim_start();
            if !is_internet(rest) {
                continue;
            }
            let end = rest
                .find(|c: char| {
                    c.is_whitespace()
                        || matches!(c, '"' | '\'' | ')' | '\\' | '<' | '>')
                })
                .unwrap_or(rest.len());
            let address = rest[..end].to_string();
            if !found.contains(&address) {
                found.push(address);
            }
        }
    }
    found
}

/// Calls `visit(path, text)` for every text in `value`, reading a text that
/// holds JSON as that JSON. `path` is the dotted key the text is under.
fn walk(value: &Value, path: &str, visit: &mut dyn FnMut(&str, &str)) {
    match value {
        Value::String(text) => match json_in(text) {
            Some(inner) => walk(&inner, path, visit),
            None => visit(path, text),
        },
        Value::Array(items) => {
            for item in items {
                walk(item, path, visit);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                let inner = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                walk(item, &inner, visit);
            }
        }
        _ => {}
    }
}

/// The JSON a text holds, when it holds an object or an array.
fn json_in(text: &str) -> Option<Value> {
    let trimmed = text.trim_start();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(|v| v.is_object() || v.is_array())
}

/// A warning for each cell whose pages would ask the Internet for an image.
pub fn check(workbook: &Workbook) -> Vec<Problem> {
    let mut problems = Vec::new();
    for sheet in workbook.sheets() {
        for row in &sheet.rows {
            for (column, value) in &row.cells {
                let mut named: Vec<(String, String)> = Vec::new();
                walk(value, "", &mut |path, text| {
                    let key = if path.is_empty() {
                        column.as_str()
                    } else {
                        path
                    };
                    let is_logo = key.ends_with(LOGO_URL);
                    let mut addresses = internet_addresses(text);
                    if is_logo && is_internet(text.trim()) {
                        addresses.insert(0, text.trim().to_string());
                    }
                    for address in addresses {
                        if !named.iter().any(|(_, seen)| *seen == address) {
                            named.push((path.to_string(), address));
                        }
                    }
                });
                for (path, address) in named {
                    let origin = row.origin(Some(column));
                    let inside = if path.is_empty() {
                        String::new()
                    } else {
                        format!(" ({path})")
                    };
                    problems.push(
                        Problem::warning(
                            Code::InvalidValue,
                            origin.to_string(),
                            format!(
                                "a page shows {address}{inside}, an image on \
                                 the Internet: a site with no Internet shows an \
                                 empty space there. Embed the file in the \
                                 workbook as a data: address"
                            ),
                        )
                        .at(&origin)
                        .id("page.internet-image")
                        .detail("address", &address),
                    );
                }
            }
        }
    }
    problems
}

/// The media type of an image file, from its first bytes.
fn media_type(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok("image/png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Ok("image/gif");
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok("image/webp");
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        let lower = text.to_ascii_lowercase();
        if lower.contains("<svg") {
            // An image never runs a script, wherever it is shown; one that
            // carries a script is not what anybody meant to embed.
            if lower.contains("<script") {
                return Err("the SVG carries a script".into());
            }
            return Ok("image/svg+xml");
        }
    }
    Err("it is not a PNG, JPEG, GIF, WebP or SVG image".into())
}

/// The `data:` address of an image file.
pub fn data_address(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "it is {} bytes; at most {MAX_IMAGE_BYTES} can be embedded",
            bytes.len()
        ));
    }
    let media = media_type(bytes)?;
    Ok(format!("data:{media};base64,{}", STANDARD.encode(bytes)))
}

/// A short, stable custom property name for an address.
fn property_name(address: &str) -> String {
    // FNV-1a: no dependency, and the name only has to be stable and distinct.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in address.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("--embedded-image-{hash:08x}")
}

/// A text with every mention of `address` replaced by `data`. When a
/// stylesheet uses it more than once, the file is written once, as a custom
/// property its rules use, so the page carries one copy.
fn embed_in_text(text: &str, address: &str, data: &str) -> (String, usize) {
    let count = text.matches(address).count();
    if count == 0 {
        return (text.to_string(), 0);
    }
    // Longest quoting first, so `url(''x'')` is not counted as `url('`...
    let mut forms: Vec<String> = ["''", "\"", "'", ""]
        .iter()
        .map(|quote| format!("url({quote}{address}{quote})"))
        .collect();
    forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
    let mut rest = text.to_string();
    let mut in_url = 0;
    for form in &forms {
        in_url += rest.matches(form.as_str()).count();
        rest = rest.replace(form.as_str(), "");
    }
    if in_url >= 2 && text.contains('{') {
        let name = property_name(address);
        let mut out = text.to_string();
        for form in &forms {
            out = out.replace(form.as_str(), &format!("var({name})"));
        }
        out = out.replace(address, data);
        // A realm text is MessageFormat, where a brace is written '{'.
        let (open, close) = if text.contains("'{'") {
            ("'{'", "'}'")
        } else {
            ("{", "}")
        };
        let rule = format!(":root {open} {name}: url(\"{data}\"); {close}\n");
        // After any @charset and @import: a rule before them voids them.
        let mut at = 0;
        loop {
            let rest = &out[at..];
            let lower = rest.trim_start().to_ascii_lowercase();
            if !(lower.starts_with("@import") || lower.starts_with("@charset"))
            {
                break;
            }
            match rest.find(';') {
                Some(end) => at += end + 1,
                None => break,
            }
        }
        let separator = if at == 0 { "" } else { "\n" };
        out.insert_str(at, &format!("{separator}{rule}"));
        return (out, count);
    }
    (text.replace(address, data), count)
}

fn embed_in_value(value: &Value, address: &str, data: &str) -> (Value, usize) {
    match value {
        Value::String(text) => {
            if !text.contains(address) {
                return (value.clone(), 0);
            }
            if let Some(inner) = json_in(text) {
                let (inner, n) = embed_in_value(&inner, address, data);
                return (Value::String(inner.to_string()), n);
            }
            let (text, n) = embed_in_text(text, address, data);
            (Value::String(text), n)
        }
        Value::Array(items) => {
            let mut total = 0;
            let items = items
                .iter()
                .map(|item| {
                    let (item, n) = embed_in_value(item, address, data);
                    total += n;
                    item
                })
                .collect();
            (Value::Array(items), total)
        }
        Value::Object(map) => {
            let mut total = 0;
            let map = map
                .iter()
                .map(|(key, item)| {
                    let (item, n) = embed_in_value(item, address, data);
                    total += n;
                    (key.clone(), item)
                })
                .collect();
            (Value::Object(map), total)
        }
        other => (other.clone(), 0),
    }
}

/// The workbook with the image at `address` embedded: every text that names
/// the address names `bytes` as a `data:` address instead. Also returns how
/// many mentions were replaced.
pub fn embed_image(
    workbook: &Workbook,
    address: &str,
    bytes: &[u8],
) -> Result<(Workbook, usize), Problem> {
    if !is_internet(address) {
        return Err(Problem::error(
            Code::InvalidValue,
            "address",
            format!("'{address}' is not an http(s) address"),
        ));
    }
    let data = data_address(bytes).map_err(|reason| {
        Problem::error(
            Code::InvalidValue,
            "image",
            format!("the file cannot be embedded: {reason}"),
        )
    })?;
    let mut replaced = 0;
    let sheets: Vec<Sheet> = workbook
        .sheets()
        .iter()
        .map(|sheet| Sheet {
            rows: sheet
                .rows
                .iter()
                .map(|row| Row {
                    cells: row
                        .cells
                        .iter()
                        .map(|(column, value)| {
                            let (value, n) =
                                embed_in_value(value, address, &data);
                            replaced += n;
                            (column.clone(), value)
                        })
                        .collect(),
                    ..row.clone()
                })
                .collect(),
            ..sheet.clone()
        })
        .collect();
    if replaced == 0 {
        return Err(Problem::error(
            Code::InvalidValue,
            "address",
            format!("nothing in the workbook names {address}"),
        ));
    }
    Ok((Workbook::new(sheets)?, replaced))
}

#[cfg(test)]
#[path = "page_images_tests.rs"]
mod tests;
