// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tar;
use tempfile::tempfile;
use flate2::Compression;
use flate2::write::GzEncoder;
use crate::types::error::{Error, Result};

// .tar.gz file
pub fn compress_folder(folder_path: &Path) -> Result<File> {
    let mut tar_gz_file = tempfile()?;
    let enc = GzEncoder::new(tar_gz_file, Compression::default());
    let mut tar_builder = tar::Builder::new(enc);
    tar_builder.append_dir_all(".", folder_path)?;
    Ok(tar_gz_file)
}