// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::temp_path::generate_temp_file;
use crate::types::error::Result;
use anyhow::Context;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::path::Path;
use tar;
use tempfile::TempPath;
use tracing::{event, instrument, Level};

// Generates a .tar.gz file, returning its path and file size
#[instrument(err)]
pub fn compress_folder(folder_path: &Path) -> Result<(TempPath, String, u64)> {
    let tar_temp_file = generate_temp_file("tally-", ".tar.gz")
        .with_context(|| "Error generating temp tar.gz file")?;
    let file2 = tar_temp_file
        .reopen()
        .with_context(|| "Couldn't reopen file for writing")?;
    let tar_file_temp_path = tar_temp_file.into_temp_path();
    let tar_file_str = tar_file_temp_path.to_string_lossy().to_string();
    event!(Level::INFO, " Path: {tar_file_str}");
    let enc = GzEncoder::new(&file2, Compression::default());
    let mut tar_builder = tar::Builder::new(enc);
    tar_builder.append_dir_all(".", folder_path)?;

    // Finish writing the .tar.gz file and get the file (temporary file in this
    // case)
    let finished_file = tar_builder.into_inner()?.finish()?;
    let file_size = finished_file.metadata().unwrap().len();
    event!(Level::INFO, " Tar file size: {file_size}");

    Ok((tar_file_temp_path, tar_file_str, file_size))
}
