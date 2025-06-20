// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use anyhow::Context;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sequent_core::util::temp_path::generate_temp_file;
use std::fs::File;
use std::path::Path;
use tempfile::{tempdir, TempDir, TempPath};
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
    if !folder_path.is_dir() {
        return Err(format!(
            "Path doesn't exist or it's not a folder: {}",
            folder_path.display()
        )
        .into());
    }
    let enc = GzEncoder::new(&file2, Compression::default());
    let mut tar_builder = tar::Builder::new(enc);
    tar_builder.append_dir_all("", folder_path)?;

    // Finish writing the .tar.gz file and get the file (temporary file in this
    // case)
    let finished_file = tar_builder.into_inner()?.finish()?;
    let file_size = finished_file.metadata()?.len();
    event!(Level::INFO, " Tar file size: {file_size}");

    Ok((tar_file_temp_path, tar_file_str, file_size))
}

// Decompresses a .tar.gz file into a temporary directory, returning the directory path
#[instrument(err)]
pub fn decompress_file(file_path: &Path) -> Result<TempDir> {
    // Create a temporary directory
    let temp_dir = tempdir().with_context(|| "Error generating temp directory")?;
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Open the .tar.gz file
    let file = File::open(file_path)
        .with_context(|| format!("Couldn't open file: {}", file_path.display()))?;
    let dec = GzDecoder::new(file);

    // Create a tar archive reader
    let mut archive = tar::Archive::new(dec);

    // Unpack the archive into the temporary directory
    archive
        .unpack(&temp_dir_path)
        .with_context(|| "Error unpacking the tar archive")?;

    event!(
        Level::INFO,
        "Decompressed into directory: {}",
        temp_dir_path.display()
    );

    Ok(temp_dir)
}
