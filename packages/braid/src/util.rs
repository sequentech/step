// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use board_messages::braid::statement::StatementType;
use thiserror::Error;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::PathBuf;
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

    /*ParseBigIntError(#[from] num_bigint::ParseBigIntError),
    #[error("io error: {0}")]
    SerializationError(#[from] std::io::Error),
    #[error("decode error: {0}")]
    DecodingError(#[from] base64::DecodeError),
    #[error("ecdsa error: {0}")]
    EcdsaError(#[from] ecdsa::Error),
    #[error("chacha20poly1305 error: {0}")]
    Chacha20Error(chacha20poly1305::Error),
    #[error("rcgen error: {0}")]
    RCGenError(#[from] rcgen::RcgenError),
    #[error("x509_parser error: {0}")]
    X509ParserError(
        #[from] x509_parser::nom::Err<x509_parser::error::X509Error>,
    ),
    #[cfg(any(feature = "openssl_core", feature = "openssl_full"))]
    #[error("openssl error: {0}")]
    OpenSSLError(#[from] openssl::error::ErrorStack),*/
}
pub trait ProtocolContext<T> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError>;
}
impl<T> ProtocolContext<T> for Result<T, ProtocolError> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError> {
        if let Err(e) = self {
            Err(ProtocolError::WrappedError(context.to_string(), Box::new(e)))
        }
        else {
            self
        }
    }
}
impl<T> ProtocolContext<T> for Result<T, StrandError> {
    fn add_context(self, context: &str) -> Result<T, ProtocolError> {
        if let Err(e) = self {
            Err(ProtocolError::WrappedError(context.to_string(), Box::new(e.into())))
        }
        else {
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
