// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! String helpers and depth-first directory renames used when exporting Velvet result folders with
//! human-readable prefixes.

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, instrument};
use walkdir::{DirEntry, WalkDir};

/// Maximum length retained from the right side of a sanitized folder name (UUID suffix excluded).
pub const FOLDER_MAX_CHARS: usize = 200;

/// Walks `folder_path` deepest-first and renames each directory whose name contains keys in
/// `replacements`, applying [`sanitize_filename`] so exports stay filesystem-safe.
///
/// # Errors
///
/// Propagates `std::io::Error` from `rename` when a target path cannot be created.
#[instrument(skip_all, err)]
pub fn rename_folders(replacements: &HashMap<String, String>, folder_path: &PathBuf) -> Result<()> {
    // Collect directories and sort by depth in descending order
    let mut directories: Vec<DirEntry> = WalkDir::new(folder_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .collect();

    directories.sort_by_key(|a| std::cmp::Reverse(a.depth()));

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
            info!("Renaming {:?} to {:?}", old_path, new_path);
            fs::rename(&old_path, &new_path)?;
        }
    }

    Ok(())
}

/// Returns up to the last `n` Unicode characters of `s`.
pub fn take_last_n_chars(s: &str, n: usize) -> String {
    s.chars()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// Returns up to the first `n` Unicode characters of `s`.
pub fn take_first_n_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Sanitizes a filename by replacing cross-platform reserved characters
/// and trimming trailing dots/spaces.
fn sanitize_filename(filename: &str) -> String {
    let sanitized = filename
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
        .to_string();

    take_last_n_chars(&sanitized, FOLDER_MAX_CHARS)
}
