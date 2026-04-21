// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(non_camel_case_types)]

use serde_json::Value;

/// UUID type
pub type uuid = String;
/// JSONB type
pub type jsonb = Value;
/// Timestamptz type
pub type timestamptz = String;
/// Bytea type
pub type bytea = String;
/// Text type
pub type text = String;
/// Varchar type
pub type varchar = String;
/// Numeric type
pub type numeric = f64;
/// JSON type
pub type json = Value;
