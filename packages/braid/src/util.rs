// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};

use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use b4::messages::statement::StatementType;
use strand::hash::Hash;
use strand::util::StrandError;

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

#[cfg(feature = "native")]
use sequent_core::services::keycloak::{KeycloakUserClient, KeycloakUserLoginConfig};
#[cfg(feature = "native")]
use tracing_subscriber::filter::LevelFilter;
#[cfg(feature = "native")]
use tracing_subscriber::reload::Handle;
#[cfg(feature = "native")]
use tracing_subscriber::{filter, reload};
#[cfg(feature = "native")]
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
#[cfg(feature = "native")]
use tracing_tree::HierarchicalLayer;

/// Initialize the tracing log, returning a handle that
/// allows changing log levels at run time.
///
/// The log can display messages within a tree representation of the
/// call stack. To do this you must mark function definitions
/// you wish to track with the #[instrument] annotation.
/// See https://docs.rs/tracing-attributes/latest/tracing_attributes/attr.instrument.html
#[cfg(feature = "native")]
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

/// Gets the Keycloak user login configuration from environment variables.
///
/// Uses:
/// - `KEYCLOAK_URL` - Keycloak server URL
/// - `KC_TRUSTEE_CLIENT_SECRET` - OAuth client secret
/// - `SUPER_ADMIN_TENANT_ID` - Tenant ID for realm
///
/// # Errors
/// Returns an error if any required environment variable is not set.
#[cfg(feature = "native")]
fn get_keycloak_trustee_user_login_config(
    username: &str,
    password: &str,
) -> Result<KeycloakUserLoginConfig> {
    use std::env;
    // TODO: Create new client in Keycloak and adapt tenant json file.
    let client_id = "cli-account-admin".to_string();
    let client_secret = env::var("KC_TRUSTEE_CLIENT_SECRET")
        .map_err(|_| anyhow!("KC_TRUSTEE_CLIENT_SECRET must be set"))?;
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .map_err(|_| anyhow!("SUPER_ADMIN_TENANT_ID must be set"))?;
    Ok(KeycloakUserLoginConfig::new(
        username.to_string(),
        password.to_string(),
        client_id,
        client_secret,
        tenant_id,
    ))
}

/// Fetches the access token from Keycloak for B4 authentication using trustee
/// credentials.
///
/// Uses KeycloakUserClient::get_cached_token() which authenticates as a specific
/// trustee user using the Resource Owner Password Credentials flow. Tokens are
/// cached to avoid repeated authentication requests.
///
/// # Arguments
/// * `username` - The Keycloak username (e.g., "trustee1")
/// * `password` - The trustee's password from the config file
#[cfg(feature = "native")]
pub async fn get_access_token(username: &str, password: &str) -> Result<String> {
    let login_config = get_keycloak_trustee_user_login_config(username, password)?;
    KeycloakUserClient::get_cached_token(&login_config).await
}
