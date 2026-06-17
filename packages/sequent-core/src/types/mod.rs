// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Trustee ceremony status, tally operations, and counting algorithm types.
pub mod ceremonies;
/// Date/time formatting helpers for reports and exports.
pub mod date_time;
/// Shared error type for sequent-core domain operations.
pub mod error;
/// GraphQL/Hasura row types mirroring the election database schema.
#[cfg(feature = "default_features")]
pub mod hasura;
/// Keycloak realm, role, and permission integration types.
pub mod keycloak;
/// Permission constants and helpers for authorization checks.
pub mod permissions;
/// Election result structures produced by tallying.
pub mod results;
/// Scheduled automation event types and date tracking.
#[cfg(feature = "default_features")]
pub mod scheduled_event;
/// Tally sheet document structures for audit and publication.
pub mod tally_sheets;
/// Handlebars template types for PDF report generation.
#[cfg(feature = "reports")]
pub mod templates;

/// Utilities for converting vectors of keyed records into lookup maps.
pub mod to_map;
