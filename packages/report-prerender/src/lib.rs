// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
pub use sequent_template_renderer::prerender::{prepare, Field, Prepared, VERSION};
mod fill;
mod stream;
pub use fill::{fill_many, validate_background};
/// Bundled font makes cached layouts portable across worker hosts.
pub const DEFAULT_FONT: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");

/// Stable object-key ordering prevents a reordered JSON request invalidating the cache.
pub fn fingerprint(template: &serde_json::Value, known: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    fn canonical(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let ordered: std::collections::BTreeMap<_, _> =
                    map.iter().map(|(k, v)| (k.clone(), canonical(v))).collect();
                serde_json::to_value(ordered).unwrap()
            }
            serde_json::Value::Array(items) => items.iter().map(canonical).collect(),
            _ => value.clone(),
        }
    }
    let mut hash = Sha256::new();
    hash.update(b"sequent-prerender-v1.1-lopdf-0.38-rustybuzz-0.20");
    hash.update(DEFAULT_FONT);
    hash.update(canonical(template).to_string());
    hash.update([0]);
    hash.update(canonical(known).to_string());
    format!("{:x}", hash.finalize())
}
mod cached;
pub use cached::CachedPdf;
