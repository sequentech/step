// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared utilities used across sequent-core services and WASM bindings.

/// JSON value to string-vector conversions for Keycloak attributes.
pub mod convert_vec;
/// Simple date and timestamp helpers.
pub mod date;
/// Timezone-aware timestamp formatting.
pub mod date_time;
/// Election bootstrap configuration loaded from JSON files.
pub mod external_config;
/// SHA-256 integrity verification for uploaded files.
pub mod integrity_check;
/// Locale parsing and validation helpers.
pub mod locale;
/// MIME type detection utilities.
pub mod mime;
/// Canonicalization of decoded votes for tally comparison.
pub mod normalize_vote;
/// Application version compatibility checks.
pub mod version;

/// Temporary file creation and reading for report generation.
#[cfg(feature = "reports")]
pub mod temp_path;

/// AWS SDK configuration helpers for S3 integration.
#[cfg(feature = "reports")]
pub mod aws;

/// Tracing subscriber initialization.
#[cfg(feature = "log")]
pub mod init_log;

#[macro_use]
/// Browser and native console logging macros.
pub mod console_log;
/// Safe conversion from `f64` to ordered floats.
pub mod float;

/// Filesystem path helpers for report assets.
#[cfg(feature = "reports")]
pub mod path;
/// Voting-portal screen validation helpers.
pub mod voting_screen;

/// Exponential backoff retry for HTTP and S3 operations.
#[cfg(any(feature = "probe", feature = "reports"))]
pub mod retry;
