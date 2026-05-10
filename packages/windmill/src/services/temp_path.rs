// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Temporary directories and scratch paths for file processing.

use crate::types::error::Result;
use anyhow::Context;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, Write};
use tempfile::Builder;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, instrument, Level};

/// QR code template.
pub const QR_CODE_TEMPLATE: &str = "<div id=\"qrcode\"></div>";
/// Logo template.
pub const LOGO_TEMPLATE: &str = "<div class=\"logo\"></div>";
/// Public assets logo image.
pub const PUBLIC_ASSETS_LOGO_IMG: &str = "sequent-logo.svg";
/// Public assets QR code library.
pub const PUBLIC_ASSETS_QRCODE_LIB: &str = "qrcode.min.js";
/// Public assets velvet ballot images template.
pub const PUBLIC_ASSETS_VELVET_BALLOT_IMAGES_TEMPLATE: &str = "ballot_images_user.hbs";
/// Public assets velvet ballot images template system.
pub const PUBLIC_ASSETS_VELVET_BALLOT_IMAGES_TEMPLATE_SYSTEM: &str = "ballot_images_system.hbs";
/// Public assets velvet MC ballot images template.
pub const PUBLIC_ASSETS_VELVET_MC_BALLOT_IMAGES_TEMPLATE: &str = "mc_ballot_images_user.hbs";
/// Velvet ballot images template title.
pub const VELVET_BALLOT_IMAGES_TEMPLATE_TITLE: &str = "Ballot Images";
/// Public assets I18N defaults.
pub const PUBLIC_ASSETS_I18N_DEFAULTS: &str = "i18n_defaults.json";

/// Public assets initialization report system template.
pub const PUBLIC_ASSETS_INITIALIZATION_TEMPLATE_SYSTEM: &str = "initialization_report_system.hbs";
/// Public assets electoral results template system.
pub const PUBLIC_ASSETS_ELECTORAL_RESULTS_TEMPLATE_SYSTEM: &str = "electoral_results_system.hbs";
