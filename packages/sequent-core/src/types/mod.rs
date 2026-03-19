// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Ceremony-related types and enums.
pub mod ceremonies;
/// Date/time formatting and timezone types.
pub mod date_time;
/// Unified error type and result alias for sequent-core.
pub mod error;

/// Types for integrating with Hasura.
#[cfg(feature = "default_features")]
pub mod hasura;
/// Keycloak integration types.
pub mod keycloak;
/// Permission and access control types.
pub mod permissions;
/// Election tally results types.
pub mod results;

// scheduled events types.
#[cfg(feature = "default_features")]
pub mod scheduled_event;
/// Tally sheet types.
pub mod tally_sheets;

/// Types related to templating and report generation.
#[cfg(feature = "reports")]
pub mod templates;

/// Utility types for mapping and conversion.
pub mod to_map;
