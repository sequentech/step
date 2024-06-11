// SPDX-FileCopyrightText: 204 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
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
