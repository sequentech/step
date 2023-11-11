// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fs::File;

use crate::types::error::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::env;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar;
use tempfile::Builder;
use tracing::{event, instrument, Level};

pub fn generate_random_path() -> Result<PathBuf> {
    // Get the system's temporary directory.
    let temp_dir = env::temp_dir();

    // Use the Builder to create a temporary file within the temporary directory.
    // The file will be deleted when the `NamedTempFile` object goes out of scope.
    let temp_file = Builder::new()
        .prefix("my-temp-") // Optional: specify a prefix for the file name.
        .suffix(".tar.gz") // Optional: specify a suffix for the file name.
        .rand_bytes(12) // Optional: specify the number of random bytes to use for the name.
        .tempfile_in(&temp_dir)?
        .into_temp_path(); // Convert the NamedTempFile into a TempPath without deleting the file when dropped.

    // At this point, temp_file is a TempPath, and the file will not be automatically
    // deleted unless you explicitly call `temp_file.close()`.

    // Get the file path.
    Ok(temp_file.to_owned())
}

// .tar.gz file
#[instrument]
pub fn compress_folder(folder_path: &Path) -> Result<Vec<u8>> {
    let tar_file_path = generate_random_path()?;
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

    //let tar_file_path = "my.tar";
    let mut opened_file = File::open(tar_file_path.clone())?;
    let mut data: Vec<u8> = Vec::new();
    opened_file.read_to_end(&mut data)?;
    event!(Level::INFO, "Vec size: {}", data.len());

    // Remove the tar file since it's no longer needed
    std::fs::remove_file(tar_file_path)?;

    Ok(data)
}
