// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};

use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use b3::messages::statement::StatementType;
use strand::hash::Hash;
use strand::util::StrandError;

pub const VERIFIABLE_BULLETIN_BOARD_FILE: &str = "verifiable_bulletin_board.db";

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
/// Allows attaching a context string to a StrandError result.
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

/// Returns a truncated hex encoding of the given hash bytes.
///
/// Used when displaying hashes in debug messages.
pub(crate) fn dbg_hash(h: &Hash) -> String {
    hex::encode(h)[0..10].to_string()
}

/// Returns a fixed-size array Hash from the given vector.
pub fn hash_from_vec(bytes: &[u8]) -> Result<Hash, StrandError> {
    strand::util::to_hash_array(bytes)
}

/// Returns base64 no pad decode.
pub fn decode_base64(s: &String) -> Result<Vec<u8>> {
    general_purpose::STANDARD_NO_PAD
        .decode(&s)
        .map_err(|error| anyhow!(error))
}

/// Checks for and creates a directory if needed.
pub fn ensure_directory(folder: PathBuf) -> Result<()> {
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

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{filter, reload};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

/// Initialize the tracing log, returning a handle that
/// allows changing log levels at run time.
///
/// The log can display messages within a tree representation of the
/// call stack. To do this you must mark function definitions
/// you wish to track with the #[instrument] annotation.
/// See https://docs.rs/tracing-attributes/latest/tracing_attributes/attr.instrument.html
pub fn init_log(set_global: bool) -> Handle<LevelFilter, Registry> {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false);

    let filter = filter::LevelFilter::INFO;
    let (filter, reload_handle) = reload::Layer::new(filter);
    let subscriber = Registry::default().with(filter).with(layer);

    if set_global {
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
    tracing_log::LogTracer::init().unwrap();
    reload_handle
}
