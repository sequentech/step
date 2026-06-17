// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared services (auth, storage, reports, translations).

/// Hierarchical area tree utilities for contest assignment.
#[cfg(feature = "areas")]
pub mod area_tree;
/// Rocket request guards and token caching for authenticated API access.
#[cfg(feature = "keycloak")]
pub mod connection;
/// Date and time parsing helpers.
#[cfg(feature = "keycloak")]
pub mod date;
/// Keycloak JWT claim types and decoding.
#[cfg(feature = "keycloak")]
pub mod jwt;
/// Keycloak Admin REST API client.
#[cfg(feature = "keycloak")]
pub mod keycloak;
/// PDF generation service integration.
#[cfg(feature = "reports")]
pub mod pdf;
/// Health and readiness probe handlers.
#[cfg(feature = "probe")]
pub mod probe;
/// Election report generation.
#[cfg(feature = "reports")]
pub mod reports;
/// S3-compatible object storage client.
#[cfg(feature = "s3")]
pub mod s3;

/// Ballot and configuration consistency checks.
pub mod error_checker;
/// URL generation for voter and admin portals.
pub mod generate_urls;

/// Translation lookup and locale handling.
pub mod translations;

/// UUID replacement when importing election events.
#[cfg(feature = "keycloak")]
pub mod replace_uuids;

/// Permission checks against JWT claims.
#[cfg(feature = "keycloak")]
pub mod authorization;

/// UUID format validation helpers.
#[cfg(feature = "keycloak")]
pub mod uuid_validation;
