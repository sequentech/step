// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Serialization helpers for ballot and strand types.

/// Base64 encoding and decoding for strand-serialized values.
pub mod base64;
/// JSON deserialization with path-aware error reporting.
pub mod deserialize_with_path;
