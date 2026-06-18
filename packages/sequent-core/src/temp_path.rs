// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Public asset paths and temporary file helpers for report generation.

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, Write};
use tempfile::Builder;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, instrument, Level};

/// HTML snippet placeholder for embedding a QR code in reports.
pub const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";
/// HTML snippet placeholder for embedding a logo in reports.
pub const LOGO_TEMPLATE: &'static str = "<div class=\"logo\"></div>";
/// Default logo image filename under public assets.
pub const PUBLIC_ASSETS_LOGO_IMG: &'static str = "sequent-logo.svg";
/// QR code JavaScript library filename under public assets.
pub const PUBLIC_ASSETS_QRCODE_LIB: &'static str = "qrcode.min.js";
/// Handlebars template for user-facing ballot image reports.
pub const PUBLIC_ASSETS_VELVET_BALLOT_IMAGES_TEMPLATE: &'static str =
    "ballot_images_user.hbs";
/// Handlebars template for system ballot image reports.
pub const PUBLIC_ASSETS_VELVET_BALLOT_IMAGES_TEMPLATE_SYSTEM: &'static str =
    "ballot_images_system.hbs";
/// Handlebars template for multi-contest ballot image reports.
pub const PUBLIC_ASSETS_VELVET_MC_BALLOT_IMAGES_TEMPLATE: &'static str =
    "mc_ballot_images_user.hbs";
/// Base Handlebars template for outbound eml files.
pub const PUBLIC_ASSETS_EML_BASE_TEMPLATE: &'static str = "eml_base.hbs";
/// Default title for ballot image PDF reports.
pub const VELVET_BALLOT_IMAGES_TEMPLATE_TITLE: &'static str =
    "Ballot images - Sequentech";
/// Default i18n strings bundled with report templates.
pub const PUBLIC_ASSETS_I18N_DEFAULTS: &'static str = "i18n_defaults.json";

/// Handlebars template for election initialization reports.
pub const PUBLIC_ASSETS_INITIALIZATION_TEMPLATE_SYSTEM: &'static str =
    "initialization_report_system.hbs";
/// Handlebars template for electoral results reports.
pub const PUBLIC_ASSETS_ELECTORAL_RESULTS_TEMPLATE_SYSTEM: &'static str =
    "electoral_results_system.hbs";

/// Reads the `PUBLIC_ASSETS_PATH` environment variable.
///
/// # Errors
///
/// Returns an error when the environment variable is not set.
pub fn get_public_assets_path_env_var() -> Result<String> {
    match env::var("PUBLIC_ASSETS_PATH") {
        Ok(path) => Ok(path),
        Err(e) => Err(e)
            .with_context(|| "Error fetching PUBLIC_ASSETS_PATH env var")?,
    }
}

/// Returns the byte size of a file at `filepath`.
///
/// # Errors
///
/// Returns an error when the file metadata cannot be read.
pub fn get_file_size(filepath: &str) -> Result<u64> {
    let metadata = fs::metadata(filepath)?;
    Ok(metadata.len())
}

/// Writes data into a named temp file. The temp file will have the
/// specificed prefix and suffix.
///
/// Returns the `TempPath` of the file, the stringified version of the path to
/// the file and the bytes size of the file.
///
/// NOTE: The file will be dropped when the `TempPath` goes out of the scope.
/// Returning the `TempPath`, even if the variable goes unused, allows the
/// caller to control the lifetime of the created temp file.
///
/// # Errors
///
/// Returns an error when temp file creation, writing, or size lookup fails.
#[instrument(skip(data), err)]
pub fn write_into_named_temp_file(
    data: &Vec<u8>,
    prefix: &str,
    suffix: &str,
) -> Result<(TempPath, String, u64)> {
    let file: NamedTempFile = generate_temp_file(prefix, suffix)
        .with_context(|| "Error creating named temp file")?;
    {
        let file2 = file
            .reopen()
            .with_context(|| "Couldn't reopen file for writing")?;
        let mut buf_writer = BufWriter::new(file2);
        buf_writer
            .write(&data)
            .with_context(|| "Error writing into named temp file")?;
        buf_writer
            .flush()
            .with_context(|| "Error calling flush into named temp file")?;
    }
    let temp_path = file.into_temp_path();
    let temp_path_string = temp_path.to_string_lossy().to_string();
    let file_size = get_file_size(temp_path_string.as_str())
        .with_context(|| "Error obtaining file size")?;
    Ok((temp_path, temp_path_string, file_size))
}

/// Creates a named temporary file with the given prefix and suffix.
///
/// # Errors
///
/// Returns an error when the temp file cannot be created.
pub fn generate_temp_file(prefix: &str, suffix: &str) -> Result<NamedTempFile> {
    // Get the system's temporary directory.
    let temp_dir = env::temp_dir();

    // Use the Builder to create a temporary file within the temporary
    // directory. The file will be deleted when the `NamedTempFile` object
    // goes out of scope.
    let temp_file = Builder::new()
        .prefix(prefix) // Optional: specify a prefix for the file name.
        .suffix(suffix) // Optional: specify a suffix for the file name.
        .rand_bytes(12) // Optional: specify the number of random bytes to use for the name.
        .tempfile_in(&temp_dir)
        .with_context(|| "Error generating temp file")?;

    Ok(temp_file)
}

/// Reads all bytes from a rewound named temporary file.
///
/// # Errors
///
/// Returns an error when rewinding or reading the temp file fails.
#[instrument(err)]
pub fn read_temp_file(temp_file: &mut NamedTempFile) -> Result<Vec<u8>> {
    // Rewind the file to the beginning to read its contents
    temp_file.rewind()?;

    // Read the file's contents into a Vec<u8>
    let mut file_bytes = Vec::new();
    temp_file.read_to_end(&mut file_bytes)?;
    Ok(file_bytes)
}

/// Reads all bytes from a persisted temporary file path.
///
/// # Errors
///
/// Returns an error when opening or reading the temp file fails.
#[instrument(err)]
pub fn read_temp_path(temp_path: &TempPath) -> Result<Vec<u8>> {
    let mut file = File::open(temp_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
