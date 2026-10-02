// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Portable, bounded files carried with a template. Paths are URL paths, never
//! filesystem paths. Renderers serve the decoded map from a synthetic origin.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const MAX_FILES: usize = 100;
pub const MAX_FILE_BYTES: usize = 2_000_000;
pub const MAX_TOTAL_BYTES: usize = 8_000_000;
pub const DOCUMENT_PATH: &str = "__sequent_document__.html";
pub const ORIGIN: &str = "https://template.invalid";
pub const DOCUMENT_URL: &str = "https://template.invalid/__sequent_document__.html";
// Response headers enforce sandboxing even for a top-level document. The
// renderer additionally intercepts all requests and remains offline.
pub const RENDER_CSP: &str = "sandbox allow-scripts allow-same-origin; default-src 'none'; script-src 'unsafe-inline' https://template.invalid; style-src 'unsafe-inline' https://template.invalid; img-src data: blob: https://template.invalid; font-src data: https://template.invalid; media-src data: https://template.invalid; connect-src https://template.invalid; worker-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'";
const START: &str = "<!--sequent-template-assets:v1:";
const END: &str = "-->";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplateAsset {
    pub mime: String,
    pub base64: String,
}
pub type TemplateAssets = BTreeMap<String, TemplateAsset>;
#[derive(Clone, Debug)]
pub struct DecodedAsset {
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub fn validate_path(path: &str) -> Result<(), String> {
    validate_bounded_path(path, 240)
}
pub(crate) fn validate_archive_path(path: &str) -> Result<(), String> {
    validate_bounded_path(path, 1024)
}
fn validate_bounded_path(path: &str, max_len: usize) -> Result<(), String> {
    if path.is_empty()
        || path.len() > max_len
        || path.starts_with('/')
        || path == DOCUMENT_PATH
        || path.contains(['\\', ':', '%', '?', '#'])
        || path.chars().any(|c| c.is_control())
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part == "__proto__"
                || part.trim() != part
                || part.ends_with('.')
        })
    {
        return Err(format!("Invalid relative file path: {path:?}"));
    }
    Ok(())
}
pub fn validate_mime(mime: &str) -> Result<(), String> {
    let mut parts = mime.split('/');
    let valid = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&c))
    };
    if mime.len() > 100
        || !parts.next().is_some_and(valid)
        || !parts.next().is_some_and(valid)
        || parts.next().is_some()
    {
        return Err("File media type must be a plain type/subtype".into());
    }
    Ok(())
}
pub fn decode(assets: &TemplateAssets) -> Result<BTreeMap<String, DecodedAsset>, String> {
    if assets.len() > MAX_FILES {
        return Err(format!("A template supports at most {MAX_FILES} files"));
    }
    let mut seen = HashSet::new();
    let mut total = 0;
    let mut decoded = BTreeMap::new();
    for (path, asset) in assets {
        validate_path(path)?;
        validate_mime(&asset.mime)?;
        if !seen.insert(path.to_lowercase()) {
            return Err(format!("File paths differ only by letter case: {path}"));
        }
        if asset.base64.len() > MAX_FILE_BYTES.div_ceil(3) * 4 {
            return Err(format!("File exceeds 2 MB: {path}"));
        }
        let bytes = STANDARD
            .decode(&asset.base64)
            .map_err(|_| format!("Invalid base64 file: {path}"))?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(format!("File exceeds 2 MB: {path}"));
        }
        total += bytes.len();
        if total > MAX_TOTAL_BYTES {
            return Err("Template files exceed 8 MB in total".into());
        }
        decoded.insert(
            path.clone(),
            DecodedAsset {
                mime: asset.mime.clone(),
                bytes,
            },
        );
    }
    Ok(decoded)
}
pub fn from_value(value: &serde_json::Value) -> Result<TemplateAssets, String> {
    if value.is_null() {
        return Ok(TemplateAssets::new());
    }
    let assets = serde_json::from_value(value.clone())
        .map_err(|e| format!("Invalid template files: {e}"))?;
    decode(&assets)?;
    Ok(assets)
}

/// Carries files through the existing HTML-only PDF transports (including
/// Lambda/OpenWhisk) without staging any user-provided filesystem paths.
/// It is inert HTML until an asset-aware renderer validates and consumes it.
pub fn attach(html: &str, assets: &TemplateAssets) -> Result<String, String> {
    decode(assets)?;
    if html.contains(START) {
        return Err("Document already contains a template file envelope".into());
    }
    if assets.is_empty() {
        return Ok(html.to_string());
    }
    let data = serde_json::to_vec(assets).map_err(|e| e.to_string())?;
    Ok(format!("{START}{}{END}{html}", STANDARD.encode(data)))
}
pub fn has_envelope(html: &str) -> bool {
    html.contains(START)
}
pub fn detach(html: &str) -> Result<(String, TemplateAssets), String> {
    let Some(start) = html.find(START) else {
        return Ok((html.to_string(), TemplateAssets::new()));
    };
    let data_start = start + START.len();
    let end = html[data_start..]
        .find(END)
        .ok_or("Invalid template file envelope")?
        + data_start;
    // Base64(JSON(base64(files))) remains bounded before either allocation.
    if end - data_start > 15_000_000 {
        return Err("Template file envelope exceeds limit".into());
    }
    let bytes = STANDARD
        .decode(&html[data_start..end])
        .map_err(|_| "Invalid template file envelope")?;
    let assets: TemplateAssets = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    decode(&assets)?;
    let clean = format!("{}{}", &html[..start], &html[end + END.len()..]);
    if clean.contains(START) {
        return Err("Duplicate template file envelopes".into());
    }
    Ok((clean, assets))
}

/// Resolves exact canonical paths only. Encoded separators, dot segments and
/// alternate hosts/schemes never become filesystem or network accesses.
pub fn request_path(url: &str) -> Option<String> {
    let path = url.strip_prefix(&format!("{ORIGIN}/"))?;
    let path = path.split(['?', '#']).next()?;
    if path.to_lowercase().contains("%2f") || path.to_lowercase().contains("%5c") {
        return None;
    }
    let decoded = urlencoding::decode(path).ok()?.into_owned();
    validate_path(&decoded).ok()?;
    Some(decoded)
}
