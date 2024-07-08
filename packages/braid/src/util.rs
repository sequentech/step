// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};

use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use board_messages::braid::statement::StatementType;
use strand::hash::Hash;
use strand::util::StrandError;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("{0}")]
    DatalogError(String),
    #[error("{0}")]
    MissingArtifact(StatementType),
    #[error("{0}")]
    MismatchedArtifactHash(StatementType),
    #[error("{0}")]
    MessageConfigurationMismatch(String),
    #[error("{0}")]
    StrandError(#[from] strand::util::StrandError),
    #[error("{0}: {1}")]
    WrappedError(String, Box<ProtocolError>),
    #[error("{0}")]
    VerificationError(String),
    #[error("{0}")]
    SignatureVerificationError(String),
    #[error("{0}")]
    InvalidTrusteeSelection(String),
    #[error("{0}")]
    InvalidConfiguration(String),
    #[error("{0}")]
    BootstrapError(String),
    #[error("{0}")]
    BoardError(String),
    #[error("{0}")]
    BoardOverwriteAttempt(String),
    #[error("{0}")]
    InternalError(String),
}
pub trait ProtocolContext<T> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError>;
}
impl<T> ProtocolContext<T> for Result<T, ProtocolError> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError> {
        if let Err(e) = self {
            Err(ProtocolError::WrappedError(
                context.to_string(),
                Box::new(e),
            ))
        } else {
            self
        }
    }
}
impl<T> ProtocolContext<T> for Result<T, StrandError> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError> {
        if let Err(e) = self {
            Err(ProtocolError::WrappedError(
                context.to_string(),
                Box::new(e.into()),
            ))
        } else {
            Ok(self?)
        }
    }
}

pub(crate) fn dbg_hash(h: &[u8; 64]) -> String {
    hex::encode(h)[0..10].to_string()
}

/*pub(crate) fn dbg_hashes<const N: usize>(hs: &[[u8; 64]; N]) -> String {
    hs.map(|h| hex::encode(h)[0..10].to_string()).join(" ")
}*/

pub fn hash_from_vec(bytes: &[u8]) -> Result<Hash, StrandError> {
    strand::util::to_hash_array(bytes)
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
