// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ordered_float::NotNan;
use serde_json::{to_string, Value};

/// Serializes an optional JSON value to a string for `SQLite` storage.
pub fn opt_json(opt: &Option<Value>) -> Option<String> {
    opt.as_ref().and_then(|v| to_string(v).ok())
}

/// Converts an optional ordered float to a plain `f64` for `SQLite` storage.
pub fn opt_f64(opt: &Option<NotNan<f64>>) -> Option<f64> {
    opt.map(|n| n.into_inner())
}
