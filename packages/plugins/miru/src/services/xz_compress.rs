// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::io::Cursor;
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn xz_compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut compressed_data = Vec::new();
    let mut input_cursor = Cursor::new(data);

    lzma_rs::xz_compress(&mut input_cursor, &mut compressed_data)
        .map_err(|e| format!("Failed to compress data with lzma-rs: {}", e))?;

    Ok(compressed_data)
}
