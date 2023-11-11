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

pub fn generate_random_path(prefix: &str, suffix: &str) -> Result<PathBuf> {
    // Get the system's temporary directory.
    let temp_dir = env::temp_dir();

    // Use the Builder to create a temporary file within the temporary directory.
    // The file will be deleted when the `NamedTempFile` object goes out of scope.
    let temp_file = Builder::new()
        .prefix(prefix) // Optional: specify a prefix for the file name.
        .suffix(suffix) // Optional: specify a suffix for the file name.
        .rand_bytes(12) // Optional: specify the number of random bytes to use for the name.
        .tempfile_in(&temp_dir)?
        .into_temp_path(); // Convert the NamedTempFile into a TempPath without deleting the file when dropped.

    // At this point, temp_file is a TempPath, and the file will not be automatically
    // deleted unless you explicitly call `temp_file.close()`.

    // Get the file path.
    Ok(temp_file.to_owned())
}