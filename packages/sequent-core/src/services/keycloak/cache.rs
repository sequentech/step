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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_token(expires_in: usize) -> TokenResponse {
        TokenResponse {
            access_token: "test-access-token".to_string(),
            expires_in,
            not_before_policy: Some(0),
            refresh_expires_in: Some(1800),
            refresh_token: Some("test-refresh-token".to_string()),
            scope: Some("openid profile email".to_string()),
            session_state: Some("test-session".to_string()),
            token_type: Some("Bearer".to_string()),
        }
    }

    #[test]
    fn test_token_cache_read_empty() {
        let cache = TokenCache::new();
        let result = cache.read_token();
        assert!(result.is_none(), "Empty cache should return None");
    }

    #[test]
    fn test_token_cache_write_then_read() {
        let cache = TokenCache::new();
        let token = create_test_token(3600); // Expires in 1 hour
        let url = "http://test-keycloak/token".to_string();

        cache
            .write_token(token.clone(), url.clone(), Instant::now())
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(result.is_some(), "Cache should return token after write");

        let (read_token, read_url) = result.unwrap();
        assert_eq!(read_token.access_token, token.access_token);
        assert_eq!(read_url, url);
    }

    #[test]
    fn test_token_cache_expiration() {
        let cache = TokenCache::new();
        // Token that expires in 1 second (effectively already expired with
        // pre-expiration buffer)
        let token = create_test_token(1);
        let url = "http://test-keycloak/token".to_string();

        // Write with a timestamp in the past
        let past_timestamp = Instant::now() - Duration::from_secs(10);
        cache
            .write_token(token, url, past_timestamp)
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(result.is_none(), "Expired token should return None");
    }

    #[test]
    fn test_token_cache_pre_expiration() {
        let cache = TokenCache::new();
        // Token expires in 10 seconds
        let token = create_test_token(10);
        let url = "http://test-keycloak/token".to_string();

        // Write with a timestamp 6 seconds in the past
        // With PRE_EXPIRATION_SECS=5, effective TTL is 10-5=5 seconds
        // After 6 seconds elapsed, the token should be considered expired
        let past_timestamp = Instant::now() - Duration::from_secs(6);
        cache
            .write_token(token, url, past_timestamp)
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(
            result.is_none(),
            "Token should be invalid 5s before expiry (pre-expiration buffer)"
        );
    }

    #[test]
    fn test_token_cache_pre_expiration_still_valid() {
        let cache = TokenCache::new();
        // Token expires in 20 seconds
        let token = create_test_token(20);
        let url = "http://test-keycloak/token".to_string();

        // Write with a timestamp 10 seconds in the past
        // With PRE_EXPIRATION_SECS=5, effective TTL is 20-5=15 seconds
        // After 10 seconds elapsed, the token should still be valid
        let past_timestamp = Instant::now() - Duration::from_secs(10);
        cache
            .write_token(token, url, past_timestamp)
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(
            result.is_some(),
            "Token should still be valid before pre-expiration window"
        );
    }

    #[test]
    fn test_token_cache_url_preserved() {
        let cache = TokenCache::new();
        let token = create_test_token(3600);
        let url = "http://custom-keycloak-url:8080/realms/test/protocol/openid-connect/token".to_string();

        cache
            .write_token(token, url.clone(), Instant::now())
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(result.is_some(), "Cache should return token");

        let (_, read_url) = result.unwrap();
        assert_eq!(read_url, url, "URL should be preserved exactly");
    }

    #[test]
    fn test_token_cache_overwrite() {
        let cache = TokenCache::new();

        // Write first token
        let token1 = create_test_token(3600);
        let url1 = "http://keycloak1/token".to_string();
        cache
            .write_token(token1, url1, Instant::now())
            .expect("Write should succeed");

        // Write second token (overwrite)
        let mut token2 = create_test_token(7200);
        token2.access_token = "new-access-token".to_string();
        let url2 = "http://keycloak2/token".to_string();
        cache
            .write_token(token2.clone(), url2.clone(), Instant::now())
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(result.is_some(), "Cache should return token");

        let (read_token, read_url) = result.unwrap();
        assert_eq!(
            read_token.access_token, token2.access_token,
            "Should return the newer token"
        );
        assert_eq!(read_url, url2, "Should return the newer URL");
    }

    #[test]
    fn test_token_cache_zero_expires_in() {
        let cache = TokenCache::new();
        // Token with 0 expires_in should be immediately invalid
        let token = create_test_token(0);
        let url = "http://test-keycloak/token".to_string();

        cache
            .write_token(token, url, Instant::now())
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(
            result.is_none(),
            "Token with 0 expires_in should be invalid"
        );
    }
}
