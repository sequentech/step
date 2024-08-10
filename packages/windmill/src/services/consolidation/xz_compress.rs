// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use std::io::prelude::*;
use std::io::Cursor;
use tracing::instrument;
use xz2::read::{XzDecoder, XzEncoder};

const XZ_COMPRESSION_LEVEL: u32 = 9;

#[instrument(skip_all, err)]
pub fn xz_compress(data: &[u8]) -> Result<Vec<u8>> {
    // Create a cursor for the input data
    let mut input_cursor = Cursor::new(data);

    // Create an XzEncoder with compression level 9
    let mut compressor = XzEncoder::new(&mut input_cursor, XZ_COMPRESSION_LEVEL);

    // Create a buffer to hold the compressed data
    let mut compressed_data = Vec::new();

    // Read the compressed data into the buffer
    compressor
        .read_to_end(&mut compressed_data)
        .with_context(|| "Failed to compress data")?;

    // Return the compressed data
    Ok(compressed_data)
}
