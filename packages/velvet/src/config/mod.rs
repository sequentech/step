//! Velvet configuration module: re-exports config types and submodules for pipeline configuration.
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod ballot_images_config;
pub mod generate_reports;

#[allow(clippy::module_inception)]
mod config;
pub use config::*;
