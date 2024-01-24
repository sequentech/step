// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::temp_path::generate_random_path;
use crate::types::error::Result;
use anyhow::Context;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Read;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::{fs::File, path::PathBuf};
use tar;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, instrument, Level};

pub fn read_file_to_bytes(path: &PathBuf) -> Result<Vec<u8>> {
    let mut opened_file = File::open(path.clone())?;
    let mut data: Vec<u8> = Vec::new();
    opened_file.read_to_end(&mut data)?;
    event!(Level::INFO, "Vec size: {}", data.len());
    Ok(data)
}

pub fn get_file_size(filepath: &str) -> Result<u64> {
    let metadata = fs::metadata(filepath)?;
    Ok(metadata.len())
}

pub fn write_into_named_temp_file(data: &Vec<u8>) -> Result<(TempPath, String, u64)> {
    let file = NamedTempFile::new().with_context(|| "Error creating named temp file")?;
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
    let file_size =
        get_file_size(temp_path_string.as_str()).with_context(|| "Error obtaining file size")?;
    Ok((temp_path, temp_path_string, file_size))
}

// Generates a .tar.gz file, returning its path and file size
#[instrument(err)]
pub fn compress_folder(folder_path: &Path) -> Result<(String, u64)> {
    let tar_file_path = generate_random_path("tally-", ".tar.gz")?;
    let tar_file_str = tar_file_path.to_string_lossy().to_string();
    event!(Level::INFO, " Path: {tar_file_str}");
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

    let file_size = get_file_size(tar_file_str.as_str())?;

    Ok((tar_file_str, file_size))
}
