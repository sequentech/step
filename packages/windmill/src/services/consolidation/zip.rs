// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Zip create/extract helpers for folder trees and archives.

use anyhow::{Context, Result};
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use tracing::{info, instrument};
use walkdir::WalkDir;
use zip::read::ZipArchive;
use zip::write::{FileOptions, SimpleFileOptions};

/// Recursively zips `src_dir` into `dst_file` with deflated entries.
///
/// # Errors
///
/// Walkdir, zip writer, or filesystem errors.
#[instrument(skip_all, err)]
pub fn compress_folder_to_zip(src_dir: &Path, dst_file: &Path) -> Result<()> {
    let path = src_dir;
    let file = File::create(dst_file)
        .with_context(|| format!("Failed to create destination file: {}", dst_file.display()))?;
    let mut zip = zip::ZipWriter::new(file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for entry in WalkDir::new(path) {
        let entry =
            entry.with_context(|| format!("Failed to access entry in path: {}", path.display()))?;
        let entry_path = entry.path();
        let name = entry_path.strip_prefix(Path::new(path)).with_context(|| {
            format!("Failed to strip prefix from path: {}", entry_path.display())
        })?;

        info!("Adding entry to zip :{}", name.display());

        if entry_path.is_file() {
            zip.start_file_from_path(name, options)
                .with_context(|| format!("Failed to add file to zip: {}", entry_path.display()))?;
            let mut f = File::open(entry_path)
                .with_context(|| format!("Failed to open file: {}", entry_path.display()))?;
            io::copy(&mut f, &mut zip).with_context(|| {
                format!("Failed to write file to zip: {}", entry_path.display())
            })?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory_from_path(name, options)
                .with_context(|| {
                    format!("Failed to add directory to zip: {}", entry_path.display())
                })?;
        }
    }
    zip.finish()
        .with_context(|| "Failed to finish writing zip file")?;
    Ok(())
}

/// Extracts `src_file` into `dst_dir`, preserving basic directory structure.
///
/// # Errors
///
/// Archive read errors or filesystem failures while writing members.
#[instrument(skip_all, err)]
pub fn unzip_file(src_file: &Path, dst_dir: &Path) -> Result<()> {
    let file = File::open(src_file)
        .with_context(|| format!("Failed to open source zip file: {}", src_file.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("Failed to read zip archive: {}", src_file.display()))?;

    for i in 0..archive.len() {
        let mut zip_member = archive
            .by_index(i)
            .with_context(|| format!("Failed to access file in archive at index: {i}"))?;
        let out_path = dst_dir.join(zip_member.sanitized_name());

        if zip_member.name().ends_with('/') {
            fs::create_dir_all(&out_path)
                .with_context(|| format!("Failed to create directory: {}", out_path.display()))?;
        } else {
            if let Some(p) = out_path.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)
                        .with_context(|| format!("Failed to create directory: {}", p.display()))?;
                }
            }
            let mut outfile = File::create(&out_path)
                .with_context(|| format!("Failed to create output file: {}", out_path.display()))?;
            io::copy(&mut zip_member, &mut outfile)
                .with_context(|| format!("Failed to write file: {}", out_path.display()))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = zip_member.unix_mode() {
                fs::set_permissions(&out_path, fs::Permissions::from_mode(mode)).with_context(
                    || format!("Failed to set permissions for: {}", out_path.display()),
                )?;
            }
        }
    }
    Ok(())
}
