// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Private, bounded file I/O shared by the coordinator and isolated workers.
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

/// Claim a fresh private directory atomically; an existing census or run must not be reused.
pub fn claim_directory(path: &Path) -> Result<()> {
    fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(path)?;
    Ok(())
}

/// Create a private directory hierarchy. Existing directories retain their permissions.
pub fn directory(path: &Path) -> Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(path)?;
    Ok(())
}

/// Claim a path atomically. An existing claim is never reclaimed after a failure.
pub fn create(path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| {
            format!(
                "Cannot claim {}; use a fresh run for previously attempted voters",
                path.display()
            )
        })
}

/// Parse a structured artifact without exposing its contents in diagnostics.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_reader(BufReader::new(File::open(path)?))
        .with_context(|| format!("Invalid artifact: {}", path.display()))
}

/// Publish structured metadata only after a complete, durable write.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let temporary = path.with_extension("partial");
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    let mut output = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut output, value)?;
    writeln!(output)?;
    output.flush()?;
    output.get_ref().sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

/// Hash arbitrarily large ciphertext shards with a fixed I/O buffer.
pub fn digest(path: &Path) -> Result<String> {
    let mut file = BufReader::new(File::open(path)?);
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024]; // I/O chunk size, independent of voter allocation.
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex::encode(hash.finalize()))
}
