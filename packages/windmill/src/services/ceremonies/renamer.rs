// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, instrument};
use walkdir::{DirEntry, WalkDir};

pub const FOLDER_MAX_CHARS: usize = 200;

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
            info!("Renaming {:?} to {:?}", old_path, new_path);
            fs::rename(&old_path, &new_path)?;
        }
    }

    Ok(())
}

pub fn take_last_n_chars(s: &str, n: usize) -> String {
    s.chars()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

pub fn take_first_n_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

// Function to sanitize filenames
fn sanitize_filename(filename: &str) -> String {
    let sanitized: String = filename
        .trim_end_matches(&[' ', '.'][..])
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();

    take_last_n_chars(&sanitized, FOLDER_MAX_CHARS)
}

/// Alias takes priority over name when deciding the election display name used
/// as the folder component in the tally output tree (and in report templates).
/// An empty alias is treated as absent.
pub fn resolve_display_name(name: &str, alias: &str) -> String {
    if alias.is_empty() {
        name.to_string()
    } else {
        alias.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- take_first_n_chars ---

    #[test]
    fn test_take_first_n_chars_shorter_than_n() {
        assert_eq!(take_first_n_chars("abc", 10), "abc");
    }

    #[test]
    fn test_take_first_n_chars_exactly_n() {
        assert_eq!(take_first_n_chars("abcde", 5), "abcde");
    }

    #[test]
    fn test_take_first_n_chars_longer_than_n() {
        assert_eq!(take_first_n_chars("abcdef", 3), "abc");
    }

    #[test]
    fn test_take_first_n_chars_unicode_counts_by_char() {
        // "Élection" is 8 chars; take 3 should give "Éle"
        assert_eq!(take_first_n_chars("Élection", 3), "Éle");
    }

    #[test]
    fn test_take_first_n_chars_empty() {
        assert_eq!(take_first_n_chars("", 5), "");
    }

    #[test]
    fn test_take_first_n_chars_zero() {
        assert_eq!(take_first_n_chars("abc", 0), "");
    }

    // --- take_last_n_chars ---

    #[test]
    fn test_take_last_n_chars_shorter_than_n() {
        assert_eq!(take_last_n_chars("abc", 10), "abc");
    }

    #[test]
    fn test_take_last_n_chars_exactly_n() {
        assert_eq!(take_last_n_chars("abcde", 5), "abcde");
    }

    #[test]
    fn test_take_last_n_chars_longer_than_n() {
        assert_eq!(take_last_n_chars("abcdef", 3), "def");
    }

    #[test]
    fn test_take_last_n_chars_empty() {
        assert_eq!(take_last_n_chars("", 5), "");
    }

    // --- sanitize_filename ---

    #[test]
    fn test_sanitize_filename_alphanumeric_and_underscore_kept() {
        assert_eq!(sanitize_filename("Election_2024"), "Election_2024");
    }

    #[test]
    fn test_sanitize_filename_spaces_become_underscores() {
        assert_eq!(sanitize_filename("My Election"), "My_Election");
    }

    #[test]
    fn test_sanitize_filename_punctuation_becomes_underscores() {
        // '!', ' ', '(' and ')' are all non-alphanumeric/non-underscore
        assert_eq!(sanitize_filename("Vote! (2024)"), "Vote___2024_");
    }

    #[test]
    fn test_sanitize_filename_unicode_letters_kept() {
        // Unicode letters satisfy is_alphanumeric() and pass through unchanged
        assert_eq!(sanitize_filename("Élection générale"), "Élection_générale");
    }

    #[test]
    fn test_sanitize_filename_truncates_to_folder_max_chars() {
        let long = "a".repeat(FOLDER_MAX_CHARS + 50);
        let result = sanitize_filename(&long);
        // truncation keeps the last FOLDER_MAX_CHARS chars
        assert_eq!(result.chars().count(), FOLDER_MAX_CHARS);
        assert_eq!(result, "a".repeat(FOLDER_MAX_CHARS));
    }

    #[test]
    fn test_sanitize_filename_empty() {
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn test_sanitize_filename_trims_trailing_spaces() {
        assert_eq!(sanitize_filename("Election   "), "Election");
    }

    #[test]
    fn test_sanitize_filename_trims_trailing_dots() {
        assert_eq!(sanitize_filename("Election..."), "Election");
    }

    #[test]
    fn test_sanitize_filename_trims_mixed_trailing_spaces_and_dots() {
        assert_eq!(sanitize_filename("Election. . ."), "Election");
    }

    #[test]
    fn test_sanitize_filename_does_not_trim_leading_spaces_or_dots() {
        assert_eq!(sanitize_filename("  Election"), "__Election");
        assert_eq!(sanitize_filename(".-Election"), ".-Election");
    }

    #[test]
    fn test_sanitize_filename_only_spaces_and_dots() {
        assert_eq!(sanitize_filename("  ...  "), "");
    }

    #[test]
    fn test_resolve_display_name_prefers_alias() {
        assert_eq!(
            resolve_display_name("General Election 2024", "GE 2024"),
            "GE 2024"
        );
    }

    #[test]
    fn test_resolve_display_name_falls_back_to_name_when_alias_empty() {
        assert_eq!(
            resolve_display_name("General Election 2024", ""),
            "General Election 2024"
        );
    }
}
