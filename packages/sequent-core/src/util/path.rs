// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_subfolders(path: &Path) -> Vec<PathBuf> {
    let mut subfolders = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    subfolders.push(path);
                }
            }
        }
    }
    subfolders
}

pub fn get_folder_name(path: &Path) -> Option<String> {
    path.components()
        .last()
        .map(|component| component.as_os_str().to_str())
        .flatten()
        .map(|component| component.to_string())
}

pub fn change_file_extension(
    filename_str: &str,
    new_extention: &str,
) -> Option<String> {
    // 1. Create a Path reference from the string slice
    let path = Path::new(filename_str);

    // 2. Use `with_extension` to create a new PathBuf with the desired
    //    extension. `with_extension` replaces the existing extension if one
    //    exists, or adds the extension if none exists. It handles the '.'
    //    correctly.
    let new_path = path.with_extension(new_extention); // Pass the new extension without the dot

    // 3. Convert the resulting PathBuf back to a String. `to_str()` converts to
    //    `Option<&str>` to handle potential non-UTF8 paths (less likely if
    //    starting from a valid Rust String, but good practice).
    //    `map(String::from)` converts the `Option<&str>` to `Option<String>`.
    new_path.to_str().map(String::from)
}
