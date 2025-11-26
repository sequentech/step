// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use fs_extra::dir::{self, CopyOptions};
use std::path::PathBuf;
use std::{fs, path::Path};
use tempfile::{tempdir, TempDir};
use tracing::{info, instrument};
use walkdir::WalkDir;

#[instrument(err)]
pub fn copy_to_temp_dir(base_tally_path: &PathBuf) -> Result<TempDir> {
    // Create a temporary directory
    let temp_dir = tempdir()?;

    // Get the path to the temporary directory
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Copy the directory contents manually
    for entry in WalkDir::new(base_tally_path)
        .into_iter()
        .filter_map(Result::ok)
    {
        let src_path = entry.path();
        let relative_path = src_path.strip_prefix(base_tally_path)?;
        let dest_path = temp_dir_path.join(relative_path);

        if src_path.is_dir() {
            fs::create_dir_all(&dest_path).context("Failed to create directory")?;
        } else {
            fs::copy(&src_path, &dest_path).context("Failed to copy file")?;
        }
    }

    // Return the path to the temporary directory
    Ok(temp_dir)
}

#[instrument]
pub fn list_files(dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            info!("Directory: {}", path.display());
            list_files(&path)?;
        } else {
            info!("File: {}", path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    #[test]
    fn test_copy_to_temp_dir() -> Result<()> {
        // Create a temporary directory to serve as the source
        let src_dir = tempdir()?;
        let src_dir_path = src_dir.path();

        // Create some files and subdirectories in the source directory
        fs::create_dir(src_dir_path.join("subdir"))?;
        fs::write(src_dir_path.join("file1.txt"), "Content of file1")?;
        fs::write(
            src_dir_path.join("subdir").join("file2.txt"),
            "Content of file2",
        )?;

        // Print the input directory structure
        println!("Source directory structure:");
        list_files(src_dir_path)?;

        // Call the function
        let temp_dir = copy_to_temp_dir(&src_dir_path.to_path_buf())?;
        let temp_dir_path = temp_dir.path();

        // Print the output directory structure
        println!("Temporary directory structure:");
        list_files(temp_dir_path)?;

        // Check that the contents of the source directory are copied correctly
        assert!(temp_dir_path.join("file1.txt").exists());
        assert!(temp_dir_path.join("subdir").join("file2.txt").exists());

        Ok(())
    }
}
