// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::PathBuf;
use strand::hash::Hash;

pub(crate) fn dbg_hash(h: &[u8; 64]) -> String {
    hex::encode(h)[0..10].to_string()
}

/*pub(crate) fn dbg_hashes<const N: usize>(hs: &[[u8; 64]; N]) -> String {
    hs.map(|h| hex::encode(h)[0..10].to_string()).join(" ")
}*/

pub fn hash_from_vec(bytes: &[u8]) -> anyhow::Result<Hash> {
    strand::util::to_hash_array(bytes).map_err(|e| e.into())
}

pub fn decode_base64(s: &String) -> Result<Vec<u8>> {
    general_purpose::STANDARD_NO_PAD
        .decode(&s)
        .map_err(|error| anyhow!(error))
}

pub fn assert_folder(folder: PathBuf) -> Result<()> {
    let path = folder.as_path();
    if path.exists() {
        if path.is_dir() {
            Ok(())
        } else {
            Err(anyhow!("Path is not a folder: {}", path.display()))
        }
    } else {
        fs::create_dir(path).map_err(|err| anyhow!(err))
    }
}
