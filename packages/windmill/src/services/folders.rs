// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use fs_extra::dir::{self, CopyOptions};
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};
use tracing::instrument;

#[instrument(err)]
pub fn copy_to_temp_dir(base_tally_path: &PathBuf) -> Result<TempDir> {
    // Create a temporary directory
    let temp_dir = tempdir()?;

    // Get the path to the temporary directory
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Set up copy options
    let mut options = CopyOptions::new();
    //options.copy_inside = false;

    // Copy the directory contents
    dir::copy(base_tally_path, &temp_dir_path, &options)?;

    // Return the path to the temporary directory
    Ok(temp_dir)
}
