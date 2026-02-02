// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{OnceLock, RwLock};
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{info, instrument, trace, warn};

use crate::services::s3::{get_public_bucket, get_s3_client};
use crate::util::aws::get_s3_aws_config;

/// Global JWKS cache instance.
static JWKS_CACHE: OnceLock<JwksCache> = OnceLock::new();

/// Returns a reference to the global JWKS cache.
#[instrument]
pub fn get_global_jwks_cache() -> &'static JwksCache {
    JWKS_CACHE.get_or_init(JwksCache::init)
}

/// Represents a single JWK key from a JWKS response.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JWKKey {
    pub alg: String,
    pub kty: String,
    pub r#use: String,
    pub n: String,
    pub e: String,
    pub kid: String,
    pub x5t: String,
    pub x5c: Vec<String>,
}

/// Represents the JWKS response containing a list of keys.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JwksOutput {
    pub keys: Vec<JWKKey>,
}

/// Returns the path to the JWKS certificates file in S3.
#[instrument]
pub fn get_jwks_secret_path() -> String {
    env::var("AWS_S3_JWKS_CERTS_PATH")
        .unwrap_or_else(|_| "certs.json".to_string())
}

/// Parses a Cache-Control header value and extracts the max-age in seconds.
/// Returns None if the header is "no-cache" or if max-age cannot be parsed.
#[instrument]
fn parse_cache_control(cache_control: Option<&str>) -> Option<u64> {
    let header = cache_control?;

    if header.contains("no-cache") || header.contains("no-store") {
        return None;
    }

    // Look for max-age=<seconds>
    for directive in header.split(',') {
        let directive = directive.trim();
        if let Some(value) = directive.strip_prefix("max-age=") {
            if let Ok(seconds) = value.trim().parse::<u64>() {
                return Some(seconds);
            }
        }
    }

    None
}

/// Fetches JWKS certificates from S3.
///
/// Returns a tuple of (keys, cache_control_seconds) where cache_control_seconds
/// is Some(seconds) if the Cache-Control header specifies max-age, or None if
/// it's set to no-cache or not present.
#[instrument(err)]
pub async fn get_jwks() -> Result<(Vec<JWKKey>, Option<u64>)> {
    let s3_bucket = get_public_bucket()
        .with_context(|| "Failed to get public S3 bucket")?;

    let path = get_jwks_secret_path();

    trace!(
        bucket = %s3_bucket,
        path = %path,
        "Fetching JWKS from S3"
    );

    let config = get_s3_aws_config(true)
        .await
        .with_context(|| "Error getting S3 AWS config")?;
    let client = get_s3_client(config)
        .await
        .with_context(|| "Error getting S3 client")?;

    let response = client
        .get_object()
        .bucket(&s3_bucket)
        .key(&path)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(err) => {
            // Check if it's a 404 Not Found
            if let Some(service_err) = err.as_service_error() {
                if service_err.is_no_such_key() {
                    info!("JWKS file not found in S3, returning empty keys");
                    return Ok((vec![], None));
                }
            }
            return Err(anyhow!(
                "Failed to download JWKS from s3://{s3_bucket}/{path}: {err:?}"
            ));
        }
    };

    // Extract cache-control header
    let cache_control_secs = parse_cache_control(response.cache_control());

    trace!(
        cache_control = ?response.cache_control(),
        cache_control_secs = ?cache_control_secs,
        "S3 response cache-control"
    );

    // Read the body
    let mut body_bytes: Vec<u8> = Vec::new();
    let mut body = response.body;
    while let Some(bytes) = body.try_next().await.map_err(|err| {
        anyhow!("Failed to read from S3 download stream: {err:?}")
    })? {
        body_bytes.extend(&bytes);
    }

    // Parse the JSON
    let jwks_output: JwksOutput = serde_json::from_slice(&body_bytes)
        .with_context(|| "Failed to parse JWKS JSON")?;

    Ok((jwks_output.keys, cache_control_secs))
}

/// Pre-expiration buffer in seconds to refresh the cache before it expires.
const CACHE_PRE_EXPIRATION_SECS: u64 = 5;

/// Default cache TTL in seconds when no Cache-Control header is present.
const DEFAULT_CACHE_TTL_SECS: u64 = 300; // 5 minutes

/// Cached JWKS entry containing keys, TTL, and fetch timestamp.
#[derive(Debug, Clone)]
struct JwksCacheEntry {
    keys: Vec<JWKKey>,
    cache_control_secs: Option<u64>,
    fetched_at: Instant,
}

/// Thread-safe cache for JWKS keys.
#[derive(Debug)]
pub struct JwksCache {
    cache: RwLock<Option<JwksCacheEntry>>,
    /// Prevents thundering herd: only one task fetches new keys at a time.
    fetch_lock: Mutex<()>,
}

impl JwksCache {
    #[instrument]
    pub fn init() -> Self {
        JwksCache {
            cache: RwLock::new(None),
            fetch_lock: Mutex::const_new(()),
        }
    }

    /// Checks if the cache entry is still valid (not expired).
    #[instrument(skip(entry))]
    fn is_cache_valid(entry: &JwksCacheEntry) -> bool {
        let ttl = entry.cache_control_secs.unwrap_or(DEFAULT_CACHE_TTL_SECS);

        // Apply pre-expiration buffer
        let effective_ttl = ttl.saturating_sub(CACHE_PRE_EXPIRATION_SECS);

        entry.fetched_at.elapsed().as_secs() < effective_ttl
    }

    /// Reads JWKS from cache if available and not expired.
    #[instrument(skip(self))]
    fn read_from_cache(&self) -> Option<Vec<JWKKey>> {
        let cache_guard = match self.cache.read() {
            Ok(guard) => guard,
            Err(err) => {
                warn!("Error acquiring read lock on JWKS cache: {err:?}");
                return None;
            }
        };

        if let Some(entry) = cache_guard.as_ref() {
            if Self::is_cache_valid(entry) {
                trace!(
                    keys_count = entry.keys.len(),
                    cache_age_secs = entry.fetched_at.elapsed().as_secs(),
                    "Returning JWKS from cache"
                );
                return Some(entry.keys.clone());
            } else {
                trace!(
                    cache_age_secs = entry.fetched_at.elapsed().as_secs(),
                    "JWKS cache expired"
                );
            }
        }

        None
    }

    /// Writes JWKS to cache.
    #[instrument(skip(self, keys))]
    fn write_to_cache(
        &self,
        keys: Vec<JWKKey>,
        cache_control_secs: Option<u64>,
    ) -> Result<()> {
        let mut cache_guard = self.cache.write().map_err(|err| {
            anyhow!("Error acquiring write lock on JWKS cache: {err:?}")
        })?;

        *cache_guard = Some(JwksCacheEntry {
            keys,
            cache_control_secs,
            fetched_at: Instant::now(),
        });

        trace!(
            cache_control_secs = ?cache_control_secs,
            "Updated JWKS cache"
        );

        Ok(())
    }

    /// Gets JWKS from cache if valid, otherwise fetches from S3 and updates cache.
    #[instrument(skip(self))]
    pub async fn get_jwks_cached(&self) -> Result<Vec<JWKKey>> {
        // Fast path: check cache without fetch lock
        if let Some(keys) = self.read_from_cache() {
            return Ok(keys);
        }

        // Acquire fetch lock to prevent thundering herd
        let _fetch_guard = self.fetch_lock.lock().await;

        // Double-check: someone else may have fetched while we waited
        if let Some(keys) = self.read_from_cache() {
            return Ok(keys);
        }

        // Still a cache miss, fetch from S3
        trace!("JWKS cache miss, fetching from S3");
        let (keys, cache_control_secs) = get_jwks().await?;

        // Update cache
        self.write_to_cache(keys.clone(), cache_control_secs)?;

        Ok(keys)
    }
}

/// Verifies a JWT token signature against the JWKS keys.
///
/// This function decodes the JWT header to find the key ID (kid),
/// looks up the corresponding key in the provided JWKS, and verifies
/// the signature.
#[instrument(skip(token, keys), fields(kid))]
pub fn verify_token_signature(token: &str, keys: &[JWKKey]) -> Result<()> {
    // Decode header to get the key ID
    let header = decode_header(token)
        .map_err(|err| anyhow!("Failed to decode JWT header: {err}"))?;

    let kid = header
        .kid
        .ok_or_else(|| anyhow!("JWT header missing 'kid' field"))?;
    tracing::Span::current().record("kid", &kid);

    // Find the matching key
    let jwk = keys
        .iter()
        .find(|k| k.kid == kid)
        .ok_or_else(|| anyhow!("No matching key found for kid: {kid}"))?;

    // Determine the algorithm.
    // NOTE: To support other algorithms in the future, you'd need to either:
    //      - Make JWKKey an enum with variants for different key types, or
    //      - Use optional fields and handle different key formats
    let algorithm = match jwk.alg.as_str() {
        "RS256" => Algorithm::RS256,
        "RS384" => Algorithm::RS384,
        "RS512" => Algorithm::RS512,
        alg => return Err(anyhow!("Unsupported algorithm: {alg}")),
    };

    // Create decoding key from RSA components (n and e)
    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|err| anyhow!("Failed to create decoding key: {err}"))?;

    // Set up validation - we only verify signature here, claims are validated elsewhere
    let mut validation = Validation::new(algorithm);
    validation.validate_exp = false; // Claims validation is done separately
    validation.validate_aud = false;

    // Decode and verify - we use a generic claims type since we only care about signature
    let _: jsonwebtoken::TokenData<serde_json::Value> =
        decode(token, &decoding_key, &validation).map_err(|err| {
            anyhow!("JWT signature verification failed: {err}")
        })?;

    trace!("JWT signature verified successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cache_control_max_age() {
        assert_eq!(parse_cache_control(Some("max-age=300")), Some(300));
        assert_eq!(parse_cache_control(Some("max-age=3600")), Some(3600));
        assert_eq!(parse_cache_control(Some("public, max-age=600")), Some(600));
        assert_eq!(parse_cache_control(Some("max-age=300, public")), Some(300));
    }

    #[test]
    fn test_parse_cache_control_no_cache() {
        assert_eq!(parse_cache_control(Some("no-cache")), None);
        assert_eq!(parse_cache_control(Some("no-store")), None);
        assert_eq!(parse_cache_control(Some("no-cache, no-store")), None);
    }

    #[test]
    fn test_parse_cache_control_none() {
        assert_eq!(parse_cache_control(None), None);
    }

    #[test]
    fn test_parse_cache_control_invalid() {
        assert_eq!(parse_cache_control(Some("max-age=invalid")), None);
        assert_eq!(parse_cache_control(Some("private")), None);
    }
}
