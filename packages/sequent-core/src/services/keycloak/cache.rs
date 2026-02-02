// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Token caching utilities for Keycloak clients.
//!
//! This module provides shared caching infrastructure for both admin client
//! (client credentials flow) and user client (password grant flow) tokens.

use crate::services::connection::PRE_EXPIRATION_SECS;
use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{instrument, warn};

/// Token response with common fields for both admin and user tokens.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: usize,
    #[serde(rename = "not-before-policy")]
    pub not_before_policy: Option<usize>,
    pub refresh_expires_in: Option<usize>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub session_state: Option<String>,
    pub token_type: Option<String>,
}

/// Extended token response with timestamp and URL for cache management.
#[derive(Debug, Clone)]
pub struct TokenResponseExt {
    pub token_resp: TokenResponse,
    pub timestamp: Instant,
    pub url: String,
}

/// Generic token cache with thundering herd prevention.
///
/// Uses a dual-locking mechanism:
/// - `RwLock` for fast path reads (check if valid)
/// - `Mutex` (`fetch_lock`) for coordinating writes (prevent multiple
///   simultaneous token requests)
#[derive(Debug)]
pub struct TokenCache {
    token: RwLock<Option<TokenResponseExt>>,
    /// Prevents thundering herd: only one task fetches a new token at a time.
    pub fetch_lock: Mutex<()>,
}

impl TokenCache {
    /// Creates a new empty token cache.
    pub fn new() -> Self {
        TokenCache {
            token: RwLock::new(None),
            fetch_lock: Mutex::const_new(()),
        }
    }

    /// Reads the access token if it has been requested successfully before and
    /// it is not expired.
    ///
    /// Returns the token response and URL if valid, None otherwise.
    #[instrument(level = "trace", skip_all)]
    pub fn read_token(&self) -> Option<(TokenResponse, String)> {
        let token_resp_ext_opt = match self.token.read() {
            Ok(read) => read.clone(),
            Err(err) => {
                warn!("Error acquiring read lock {err:?}");
                return None;
            }
        };

        if let Some(data) = token_resp_ext_opt {
            let pre_expiration_time: i64 =
                data.token_resp.expires_in as i64 - PRE_EXPIRATION_SECS;
            if pre_expiration_time.is_positive()
                && data.timestamp.elapsed()
                    < Duration::from_secs(pre_expiration_time as u64)
            {
                return Some((data.token_resp, data.url));
            }
        }
        None
    }

    /// Writes the token to the cache.
    #[instrument(level = "trace", skip_all)]
    pub fn write_token(
        &self,
        token_resp: TokenResponse,
        url: String,
        timestamp: Instant,
    ) -> Result<(), String> {
        let mut write = self
            .token
            .write()
            .map_err(|err| format!("Error acquiring write lock: {err:?}"))?;

        *write = Some(TokenResponseExt {
            token_resp,
            timestamp,
            url,
        });

        Ok(())
    }
}

impl Default for TokenCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Global admin token cache instance.
static ADMIN_TOKEN_CACHE: OnceLock<TokenCache> = OnceLock::new();

/// Returns a reference to the global admin token cache.
#[instrument(level = "trace", skip_all)]
pub fn get_admin_token_cache() -> &'static TokenCache {
    ADMIN_TOKEN_CACHE.get_or_init(TokenCache::new)
}

/// Global user token cache instance (i.e. for trustee authentication).
static USER_TOKEN_CACHE: OnceLock<TokenCache> = OnceLock::new();

/// Returns a reference to the global user token cache.
#[instrument(level = "trace", skip_all)]
pub fn get_user_token_cache() -> &'static TokenCache {
    USER_TOKEN_CACHE.get_or_init(TokenCache::new)
}
