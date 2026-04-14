// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ordered_float::NotNan;
use serde_json::{to_string, Value};

#[must_use]
/// Converts an `Option<Value>` to an `Option<String>`.
pub fn opt_json(opt: &Option<Value>) -> Option<String> {
    opt.as_ref().and_then(|v| to_string(v).ok())
}

#[must_use]
/// Converts an `Option<NotNan<f64>>` to an `Option<f64>`.
pub fn opt_f64(opt: &Option<NotNan<f64>>) -> Option<f64> {
    opt.map(ordered_float::NotNan::into_inner)
}
