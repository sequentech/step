// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "areas")]
pub mod area_tree;
#[cfg(feature = "keycloak")]
pub mod axum_auth;
#[cfg(feature = "keycloak")]
pub mod connection;
#[cfg(feature = "keycloak")]
pub mod date;
#[cfg(all(feature = "keycloak", feature = "s3"))]
pub mod jwks;
#[cfg(feature = "keycloak")]
pub mod jwt;
#[cfg(feature = "keycloak")]
pub mod keycloak;
#[cfg(feature = "reports")]
pub mod pdf;
#[cfg(feature = "probe")]
pub mod probe;
#[cfg(feature = "reports")]
pub mod reports;
#[cfg(feature = "s3")]
pub mod s3;

pub mod error_checker;
pub mod generate_urls;

pub mod translations;

#[cfg(feature = "keycloak")]
pub mod replace_uuids;

#[cfg(feature = "keycloak")]
pub mod authorization;

#[cfg(feature = "keycloak")]
pub mod uuid_validation;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
