// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::temp_path::generate_random_path;
use crate::types::error::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Read;
use std::path::Path;
use std::{fs::File, path::PathBuf};
use tar;
use tracing::{event, instrument, Level};

pub fn read_file_to_bytes(path: &PathBuf) -> Result<Vec<u8>> {
    let mut opened_file = File::open(path.clone())?;
    let mut data: Vec<u8> = Vec::new();
    opened_file.read_to_end(&mut data)?;
    event!(Level::INFO, "Vec size: {}", data.len());
    Ok(data)
}

// .tar.gz file
#[instrument(err)]
pub fn compress_folder(folder_path: &Path) -> Result<Vec<u8>> {
    let tar_file_path = generate_random_path("tally-", ".tar.gz")?;
    event!(Level::INFO, " Path: {}", tar_file_path.display());
    let tar_gz_file = File::create(tar_file_path.clone())?;
    let enc = GzEncoder::new(tar_gz_file, Compression::default());
    let mut tar_builder = tar::Builder::new(enc);
    tar_builder.append_dir_all(".", folder_path)?;

    // Finish writing the .tar.gz file and get the file (temporary file in this case)
    let finished_file = tar_builder.into_inner()?.finish()?;
    event!(
        Level::INFO,
        " Tar file size: {}",
        finished_file.metadata().unwrap().len()
    );

    let data = read_file_to_bytes(&tar_file_path)?;

    // Remove the tar file since it's no longer needed
    std::fs::remove_file(tar_file_path)?;

    Ok(data)
}
