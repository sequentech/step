// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::instrument;
use walkdir::{DirEntry, WalkDir};

#[instrument(skip_all, err)]
pub fn rename_folders(replacements: &HashMap<String, String>, folder_path: &PathBuf) -> Result<()> {
    // Collect directories and sort by depth in descending order
    let mut directories: Vec<DirEntry> = WalkDir::new(folder_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .collect();

    directories.sort_by(|a, b| b.depth().cmp(&a.depth()));

    // Rename directories
    for entry in directories {
        let old_path = entry.path().to_path_buf();
        let dir_name = entry.file_name().to_string_lossy().into_owned();
        let mut new_dir_name = dir_name.clone();
        for (from, to) in replacements {
            new_dir_name = new_dir_name.replace(from, to);
        }
        new_dir_name = sanitize_filename(&new_dir_name);
        if new_dir_name != dir_name {
            let new_path = old_path.with_file_name(new_dir_name);
            fs::rename(&old_path, &new_path)?;
            println!("Renamed {:?} to {:?}", old_path, new_path);
        }
    }

    Ok(())
}

// Function to sanitize filenames
fn sanitize_filename(filename: &str) -> String {
    filename
        .replace("/", "_") // Linux and macOS directory separator
        .replace("\\", "_") // Windows directory separator
        .replace(":", "_") // Windows and classic macOS
        .replace("*", "_")
        .replace("?", "_")
        .replace("\"", "_")
        .replace("<", "_")
        .replace(">", "_")
        .replace("|", "_")
        .trim_end_matches(&[' ', '.'][..]) // Trim trailing spaces and dots (Windows)
        .to_string()
}
