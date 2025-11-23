// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::plugins::{get_plugin_shared_dir, Plugins};

/// represent a temporary file and ensure it's removed on drop.
pub struct TempFileGuard {
    pub path: PathBuf,
}

impl TempFileGuard {
    pub fn new(path: PathBuf) -> Self {
        TempFileGuard { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.path) {
            // Log the error but don't panic, as this happens on cleanup.
            eprintln!(
                "Failed to remove temporary file: {:?}, error: {}",
                self.path, e
            );
        }
    }
}

/// Generates a unique filename using a prefix and a suffix.
fn generate_unique_filename(prefix: &str, suffix: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{}{}", prefix, timestamp, suffix)
}

/// Creates a new temporary file in the current working directory.
pub fn create_temp_file(
    prefix: &str,
    suffix: &str,
    base_path: &str,
) -> Result<(TempFileGuard, String), String> {
    let filename = generate_unique_filename(prefix, suffix);
    let path = PathBuf::from(base_path).join(filename.clone());

    File::create(&path).map_err(|e| {
        format!("Error creating temp file at {:?}: {}", path, e)
    })?;

    Ok((TempFileGuard::new(path), filename))
}

/// Writes data into a named temporary file.
pub fn write_into_named_temp_file(
    data: &[u8],
    prefix: &str,
    suffix: &str,
    base_path: &str,
) -> Result<(TempFileGuard, String, String, u64), String> {
    let (temp_file_guard, file_name) =
        create_temp_file(prefix, suffix, base_path)?;
    let temp_path = temp_file_guard.path();

    {
        let mut file = OpenOptions::new()
            .write(true)
            .open(&temp_path)
            .map_err(|e| {
                format!(
                    "Couldn't open file for writing at {:?}: {}",
                    temp_path, e
                )
            })?;

        file.write_all(data).map_err(|e| {
            format!("Error writing into named temp file: {}", e)
        })?;
    }

    let file_size = get_file_size(&temp_path)?;
    let temp_path_string = temp_path.to_string_lossy().to_string();

    Ok((temp_file_guard, file_name, temp_path_string, file_size))
}

/// Obtains the size of a file.
pub fn get_file_size(path: &Path) -> Result<u64, String> {
    let metadata = fs::metadata(path).map_err(|e| {
        format!("Error obtaining file metadata for {:?}: {}", path, e)
    })?;
    Ok(metadata.len())
}

pub fn read_temp_file(
    temp_file_guard: &TempFileGuard,
) -> Result<Vec<u8>, String> {
    let path = temp_file_guard.path();

    let mut file = File::open(path).map_err(|e| {
        format!("Error opening temp file for reading at {:?}: {}", path, e)
    })?;

    let mut file_bytes = Vec::new();
    file.read_to_end(&mut file_bytes).map_err(|e| {
        format!("Error reading contents of temp file at {:?}: {}", path, e)
    })?;

    Ok(file_bytes)
}
