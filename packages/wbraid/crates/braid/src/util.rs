// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};

use std::fmt::Debug;
use thiserror::Error;

use b4::messages::statement::StatementType;
use b4::CryptographicHash as Hash;
use cryptography::utils::error::Error as CryptographyError;

/// An error that occurs during protocol execution.
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
    CryptographyError(#[from] cryptography::utils::error::Error),
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
    #[error("WASM implementation not yet available")]
    WasmNotImplemented,
}
/// Allows attaching a context string to a ProtocolError result.
///
/// Analogous to anyhow::Context
pub trait ProtocolContext<T> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError>;
}
impl<T> ProtocolContext<T> for Result<T, ProtocolError> {
    /// Attaches a contextual string to a ProtocolError.
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
/// Allows attaching a context string to a CryptographyError result.
impl<T> ProtocolContext<T> for Result<T, CryptographyError> {
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

/// Returns a truncated hex encoding of the given hash bytes.
///
/// Used when displaying hashes in debug messages.
pub(crate) fn dbg_hash(h: &Hash) -> String {
    hex::encode(h)[0..10].to_string()
}

/// Returns a fixed-size array Hash from the given vector.
pub fn hash_from_vec(bytes: &[u8]) -> Result<Hash, CryptographyError> {
    if bytes.len() == 64 {
        Ok(Hash::try_from(bytes)?)
    } else {
        Err(CryptographyError::DeserializationError(
            format!("Expected 64 bytes, got {}", bytes.len()),
        ))
    }
}

/// Returns base64 no pad decode.
pub fn decode_base64(s: &String) -> Result<Vec<u8>> {
    general_purpose::STANDARD_NO_PAD
        .decode(&s)
        .map_err(|error| anyhow!(error))
}
