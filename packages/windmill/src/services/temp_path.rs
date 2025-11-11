// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use anyhow::Context;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, Write};
use tempfile::Builder;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, instrument, Level};

pub const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";
pub const LOGO_TEMPLATE: &'static str = "<div class=\"logo\"></div>";
pub const PUBLIC_ASSETS_LOGO_IMG: &'static str = "sequent-logo.svg";
pub const PUBLIC_ASSETS_QRCODE_LIB: &'static str = "qrcode.min.js";
pub const PUBLIC_ASSETS_VELVET_MC_VOTE_RECEIPTS_TEMPLATE: &'static str = "mc_vote_receipt_user.hbs";
pub const PUBLIC_ASSETS_VELVET_BALLOT_IMAGES_TEMPLATE: &'static str = "ballot_images_user.hbs";
pub const PUBLIC_ASSETS_VELVET_BALLOT_IMAGES_TEMPLATE_SYSTEM: &'static str =
    "ballot_images_system.hbs";
pub const PUBLIC_ASSETS_VELVET_MC_BALLOT_IMAGES_TEMPLATE: &'static str =
    "mc_ballot_images_user.hbs";
pub const VELVET_BALLOT_IMAGES_TEMPLATE_TITLE: &'static str = "Ballot Images";
pub const PUBLIC_ASSETS_I18N_DEFAULTS: &'static str = "i18n_defaults.json";

pub const PUBLIC_ASSETS_INITIALIZATION_TEMPLATE_SYSTEM: &'static str =
    "initialization_report_system.hbs";
pub const PUBLIC_ASSETS_ELECTORAL_RESULTS_TEMPLATE_SYSTEM: &'static str =
    "electoral_results_system.hbs";
