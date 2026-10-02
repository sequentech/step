// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "areas")]
pub mod area_tree;
#[cfg(feature = "keycloak")]
pub mod connection;
#[cfg(feature = "time")]
pub mod date;
#[cfg(feature = "jwt")]
pub mod jwt;
#[cfg(feature = "keycloak")]
pub mod keycloak;
#[cfg(feature = "reports")]
pub mod pdf;
#[cfg(feature = "probe")]
pub mod probe;
#[cfg(feature = "template_rendering")]
pub mod reports;
#[cfg(feature = "s3")]
pub mod s3;
#[cfg(feature = "template_pdf")]
pub mod template_pdf;

#[cfg(feature = "default_features")]
pub mod error_checker;
#[cfg(feature = "default_features")]
pub mod generate_urls;

#[cfg(feature = "default_features")]
pub mod tally_sheet_validation;
#[cfg(feature = "default_features")]
pub mod translations;

#[cfg(feature = "keycloak")]
pub mod replace_uuids;

#[cfg(feature = "keycloak")]
pub mod authorization;

#[cfg(feature = "keycloak")]
pub mod uuid_validation;
