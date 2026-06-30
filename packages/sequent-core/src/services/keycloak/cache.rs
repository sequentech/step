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
use std::time::{SystemTime, UNIX_EPOCH};
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

/// Extended token response with URL and pre-computed absolute expiry
/// timestamps for cache management.
///
/// The `expires_at` / `refresh_expires_at` fields hold Unix timestamps
/// computed at write time (`now + expires_in`).  The original
/// `token_resp` is stored **unmodified** so that downstream consumers
/// (e.g. `KeycloakAdminToken`) still see the relative durations that
/// Keycloak returns.
#[derive(Debug, Clone)]
pub struct TokenResponseExt {
    pub token_resp: TokenResponse,
    pub url: String,
    pub expires_at: usize,
    pub refresh_expires_at: Option<usize>,
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
            let time_now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let pre_expiration_time: i64 =
                data.expires_at as i64 - PRE_EXPIRATION_SECS;
            if time_now < pre_expiration_time {
                return Some((data.token_resp, data.url));
            }
        }
        None
    }

    /// Reads the cached token for refresh purposes.
    ///
    /// Unlike `read_token`, this returns the cached `TokenResponse` even when
    /// the access token is near/past expiration, as long as the refresh token
    /// is still valid. Returns `None` if there is no cached token, no refresh
    /// token, or the refresh token has also expired.
    #[instrument(level = "trace", skip_all)]
    pub fn read_token_for_refresh(&self) -> Option<TokenResponse> {
        let token_resp_ext_opt = match self.token.read() {
            Ok(read) => read.clone(),
            Err(err) => {
                warn!("Error acquiring read lock {err:?}");
                return None;
            }
        };

        if let Some(data) = token_resp_ext_opt {
            // Must have a refresh token
            if data.token_resp.refresh_token.is_none() {
                return None;
            }

            // Check that the refresh token itself hasn't expired
            if let Some(refresh_expires_at) = data.refresh_expires_at {
                let time_now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let pre_expiration_time: i64 =
                    refresh_expires_at as i64 - PRE_EXPIRATION_SECS;
                if time_now < pre_expiration_time {
                    return Some(data.token_resp);
                }
            }
        }
        None
    }

    /// Writes the token to the cache.
    ///
    /// Computes absolute Unix timestamps (`expires_at`,
    /// `refresh_expires_at`) from the relative durations in `token_resp`
    /// and stores them alongside the **unmodified** token response.
    #[instrument(level = "trace", skip_all)]
    pub fn write_token(
        &self,
        token_resp: TokenResponse,
        url: String,
    ) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as usize)
            .unwrap_or(0);
        let expires_at = now + token_resp.expires_in;
        let refresh_expires_at = token_resp.refresh_expires_in.map(|r| now + r);

        let mut write = self
            .token
            .write()
            .map_err(|err| format!("Error acquiring write lock: {err:?}"))?;

        *write = Some(TokenResponseExt {
            token_resp,
            url,
            expires_at,
            refresh_expires_at,
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

    /// Creates a test token with the given relative expiration (seconds from
    /// now), matching the format Keycloak returns. `write_token` will convert
    /// this to an absolute timestamp internally.
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
        // Token valid for 1 hour (relative seconds, like Keycloak returns)
        let token = create_test_token(3600);
        let url = "http://test-keycloak/token".to_string();

        cache
            .write_token(token.clone(), url.clone())
            .expect("Write should succeed");

        let result = cache.read_token();
        assert!(result.is_some(), "Cache should return token after write");

        let (read_token, read_url) = result.unwrap();
        assert_eq!(read_token.access_token, token.access_token);
        assert_eq!(read_url, url);
    }

    #[test]
    fn test_token_cache_pre_expiration() {
        let cache = TokenCache::new();
        // Token expires in 3 seconds (less than PRE_EXPIRATION_SECS=5)
        // So it should be considered expired due to pre-expiration buffer
        let token = create_test_token(3);
        let url = "http://test-keycloak/token".to_string();

        cache.write_token(token, url).expect("Write should succeed");

        let result = cache.read_token();
        assert!(
            result.is_none(),
            "Token should be invalid when within pre-expiration buffer (5s)"
        );
    }

    #[test]
    fn test_token_cache_pre_expiration_still_valid() {
        let cache = TokenCache::new();
        // Token expires in 20 seconds (well beyond PRE_EXPIRATION_SECS=5)
        let token = create_test_token(20);
        let url = "http://test-keycloak/token".to_string();

        cache.write_token(token, url).expect("Write should succeed");

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
            .write_token(token, url.clone())
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
            .write_token(token1, url1)
            .expect("Write should succeed");

        // Write second token (overwrite)
        let mut token2 = create_test_token(7200);
        token2.access_token = "new-access-token".to_string();
        let url2 = "http://keycloak2/token".to_string();
        cache
            .write_token(token2.clone(), url2.clone())
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
        // Token with 0 expires_in means it expires immediately
        let token = create_test_token(0);
        let url = "http://test-keycloak/token".to_string();

        cache.write_token(token, url).expect("Write should succeed");

        let result = cache.read_token();
        assert!(
            result.is_none(),
            "Token with 0 expires_in should be invalid"
        );
    }

    #[test]
    fn test_token_cache_read_for_refresh_access_expired() {
        let cache = TokenCache::new();
        // Access token expires in 3s (within PRE_EXPIRATION_SECS=5 buffer),
        // but refresh token is valid for 1800s
        let token = create_test_token(3);
        let url = "http://test-keycloak/token".to_string();

        cache.write_token(token, url).expect("Write should succeed");

        // read_token should fail (access expired)
        assert!(cache.read_token().is_none());

        // read_token_for_refresh should succeed (refresh still valid)
        let result = cache.read_token_for_refresh();
        assert!(
            result.is_some(),
            "Should return token for refresh when access expired but refresh valid"
        );
        assert_eq!(
            result.unwrap().refresh_token.unwrap(),
            "test-refresh-token"
        );
    }

    #[test]
    fn test_token_cache_read_for_refresh_both_expired() {
        let cache = TokenCache::new();
        // Both access and refresh expire immediately
        let mut token = create_test_token(0);
        token.refresh_expires_in = Some(0);
        let url = "http://test-keycloak/token".to_string();

        cache.write_token(token, url).expect("Write should succeed");

        let result = cache.read_token_for_refresh();
        assert!(
            result.is_none(),
            "Should return None when both access and refresh tokens expired"
        );
    }

    #[test]
    fn test_token_cache_read_for_refresh_no_refresh_token() {
        let cache = TokenCache::new();
        let mut token = create_test_token(3);
        token.refresh_token = None;
        let url = "http://test-keycloak/token".to_string();

        cache.write_token(token, url).expect("Write should succeed");

        let result = cache.read_token_for_refresh();
        assert!(
            result.is_none(),
            "Should return None when no refresh token available"
        );
    }

    #[test]
    fn test_token_cache_refresh_expires_in_not_mutated() {
        let cache = TokenCache::new();
        let token = create_test_token(3600);
        let url = "http://test-keycloak/token".to_string();

        cache.write_token(token, url).expect("Write should succeed");

        // The TokenResponse returned by the cache should still carry the
        // original relative duration, not an absolute timestamp.
        let result = cache.read_token_for_refresh();
        assert!(result.is_some());
        let stored = result.unwrap();
        assert_eq!(
            stored.refresh_expires_in.unwrap(),
            1800,
            "refresh_expires_in should remain the original relative value"
        );
        assert_eq!(
            stored.expires_in, 3600,
            "expires_in should remain the original relative value"
        );
    }
}
