// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use anyhow::Context;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sequent_core::util::temp_path::generate_temp_file;
use std::fs::File;
use std::path::Path;
use tempfile::{tempdir, TempDir, TempPath};
use tracing::{event, instrument, Level};

/// Generates a .tar or .tar.gz file from a folder, returning its path, string representation, and file size.
///
/// # Arguments
/// * `folder_path` - The path to the folder to be archived.
/// * `compress` - If true, creates a compressed .tar.gz file; otherwise, creates an uncompressed .tar file.
#[instrument(err)]
pub fn create_archive_from_folder(
    folder_path: &Path,
    compress: bool,
) -> Result<(TempPath, String, u64)> {
    let extension = if compress { ".tar.gz" } else { ".tar" };
    let tar_temp_file = generate_temp_file("tally-", extension)
        .with_context(|| format!("Error generating temporary {} file", extension))?;

    // Reopen the temp file for writing. This handle will be used by the archiver/compressor.
    let file_write_handle = tar_temp_file
        .reopen()
        .with_context(|| "Couldn't reopen temporary file for writing")?;

    // Get the TempPath, which ensures the file is deleted when this path goes out of scope.
    // The NamedTempFile is consumed here.
    let tar_file_temp_path = tar_temp_file.into_temp_path();
    let tar_file_str = tar_file_temp_path.to_string_lossy().to_string();

    if !folder_path.is_dir() {
        return Err(format!(
            // Using anyhow::anyhow for a simpler error creation
            "Path doesn't exist or it's not a folder: {}",
            folder_path.display()
        )
        .into());
    }

    if compress {
        let gz_encoder = GzEncoder::new(&file_write_handle, Compression::default());
        let mut tar_builder = tar::Builder::new(gz_encoder);
        tar_builder.append_dir_all("", folder_path)?;
        // Must finish the GzEncoder to finalize the GZip stream and get the underlying writer.
        let finished_gz_encoder = tar_builder.into_inner()?;
        finished_gz_encoder.finish()?;
    } else {
        let mut tar_builder = tar::Builder::new(&file_write_handle);
        tar_builder.append_dir_all("", folder_path)?;
        // Must finish the TarBuilder to finalize the TAR archive (write trailing null blocks).
        tar_builder.finish()?;
    }

    // Ensure all data is flushed from OS buffers to the disk.
    file_write_handle.sync_all()?;

    let file_size = file_write_handle
        .metadata()
        .with_context(|| "Failed to get metadata from temporary archive file")?
        .len();
    event!(
        Level::INFO,
        "Archive file ({}) size: {} bytes",
        extension,
        file_size
    );

    Ok((tar_file_temp_path, tar_file_str, file_size))
}

/// Decompresses/extracts a .tar.gz or .tar file into a temporary directory, returning the directory path.
///
/// # Arguments
/// * `file_path` - The path to the .tar.gz or .tar file to be decompressed/extracted.
/// * `is_compressed` - If true, assumes the file is a .tar.gz and decompresses it;
///                    otherwise, assumes it's a .tar file and extracts it directly.
#[instrument(err)]
pub fn extract_archive_to_temp_dir(file_path: &Path, is_compressed: bool) -> Result<TempDir> {
    let temp_dir =
        tempdir().with_context(|| "Error generating temporary directory for extraction")?;
    let temp_dir_path_buf = temp_dir.path().to_path_buf();

    let file_to_read = File::open(file_path)
        .with_context(|| format!("Couldn't open archive file: {}", file_path.display()))?;

    if is_compressed {
        let gz_decoder = GzDecoder::new(file_to_read); // GzDecoder takes ownership of file_to_read
        let mut archive = tar::Archive::new(gz_decoder); // Archive takes ownership of gz_decoder
        archive
            .unpack(&temp_dir_path_buf)
            .with_context(|| "Error unpacking the compressed tar.gz archive")?;
        event!(
            Level::INFO,
            "Decompressed .tar.gz into directory: {}",
            temp_dir_path_buf.display()
        );
    } else {
        let mut archive = tar::Archive::new(file_to_read); // Archive takes ownership of file_to_read
        archive
            .unpack(&temp_dir_path_buf)
            .with_context(|| "Error unpacking the .tar archive")?;
        event!(
            Level::INFO,
            "Extracted .tar into directory: {}",
            temp_dir_path_buf.display()
        );
    }

    Ok(temp_dir)
}
