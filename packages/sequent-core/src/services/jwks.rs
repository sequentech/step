// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verifies Keycloak access tokens with `certs.json`, the JWKS that Windmill
//! publishes for Hasura. Each key there names the realm it belongs to, so a
//! token is only trusted for the realm whose key signed it.

use crate::services::jwt::JwtClaims;
use crate::services::keycloak::{get_event_realm, get_tenant_realm};
use anyhow::{anyhow, bail, Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::header::{HeaderValue, CACHE_CONTROL};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{instrument, warn};

pub const JWKS_FETCH_TIMEOUT_ENV: &str = "AWS_S3_JWKS_FETCH_TIMEOUT_SECS";
pub const JWKS_MIN_REFRESH_ENV: &str = "AWS_S3_JWKS_MIN_REFRESH_SECS";
const DEFAULT_JWKS_FETCH_TIMEOUT_SECS: u64 = 5;
const DEFAULT_JWKS_MIN_REFRESH_SECS: u64 = 10;

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
    /// Keycloak realm whose key this is. Hasura ignores this member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JwksOutput {
    pub keys: Vec<JWKKey>,
}

/// `certs.json` as read, with how long it may be cached.
pub struct Jwks {
    pub keys: Vec<JWKKey>,
    pub max_age: Duration,
}

pub fn get_jwks_secret_path() -> String {
    env::var("AWS_S3_JWKS_CERTS_PATH").unwrap_or("certs.json".to_string())
}

fn seconds_setting(name: &str, default: u64) -> Duration {
    Duration::from_secs(
        env::var(name)
            .ok()
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(default),
    )
}

/// The `max-age` of a `Cache-Control` header, zero when there is none.
fn max_age(cache_control: Option<&HeaderValue>) -> Duration {
    cache_control
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.split(',').find_map(|directive| {
                directive.trim().strip_prefix("max-age=")?.parse().ok()
            })
        })
        .map(Duration::from_secs)
        .unwrap_or_default()
}

/// Reads `certs.json` from the public bucket, as Hasura does. A missing file
/// has no keys.
#[instrument(err)]
pub async fn fetch_jwks() -> Result<Jwks> {
    let url = format!(
        "{}/{}/{}",
        env::var("AWS_S3_PRIVATE_URI")
            .context("AWS_S3_PRIVATE_URI must be set")?,
        env::var("AWS_S3_PUBLIC_BUCKET")
            .context("AWS_S3_PUBLIC_BUCKET must be set")?,
        get_jwks_secret_path()
    );
    let response = reqwest::Client::new()
        .get(url)
        .timeout(seconds_setting(
            JWKS_FETCH_TIMEOUT_ENV,
            DEFAULT_JWKS_FETCH_TIMEOUT_SECS,
        ))
        .send()
        .await
        .context("Error requesting the JWKS")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Jwks {
            keys: vec![],
            max_age: Duration::ZERO,
        });
    }
    let response = response
        .error_for_status()
        .context("Error requesting the JWKS")?;
    let max_age = max_age(response.headers().get(CACHE_CONTROL));
    let jwks: JwksOutput =
        response.json().await.context("Error parsing the JWKS")?;
    Ok(Jwks {
        keys: jwks.keys,
        max_age,
    })
}

struct CachedKeys {
    keys: HashMap<String, JWKKey>,
    expires_at: Instant,
}

/// `certs.json`, kept for its `max-age`. A token signed with a key that is
/// not cached, or that has no realm yet, refreshes it, at most once every
/// `AWS_S3_JWKS_MIN_REFRESH_SECS`. When a refresh fails the cached keys stay
/// in use.
struct JwksCache {
    cached: RwLock<Arc<CachedKeys>>,
    /// Held while refreshing, so that one request refreshes for all.
    last_refresh: Mutex<Option<Instant>>,
}

static JWKS_CACHE: LazyLock<JwksCache> = LazyLock::new(|| JwksCache {
    cached: RwLock::new(Arc::new(CachedKeys {
        keys: HashMap::new(),
        expires_at: Instant::now(),
    })),
    last_refresh: Mutex::new(None),
});

impl JwksCache {
    async fn key(&self, kid: &str) -> Result<JWKKey> {
        if let Some(key) = self.current_key(kid).await {
            return Ok(key);
        }
        let mut last_refresh = self.last_refresh.lock().await;
        if let Some(key) = self.current_key(kid).await {
            return Ok(key);
        }
        let min_refresh = seconds_setting(
            JWKS_MIN_REFRESH_ENV,
            DEFAULT_JWKS_MIN_REFRESH_SECS,
        );
        if last_refresh.is_none_or(|at| at.elapsed() >= min_refresh) {
            *last_refresh = Some(Instant::now());
            match fetch_jwks().await {
                Ok(jwks) => {
                    *self.cached.write().await = Arc::new(CachedKeys {
                        expires_at: Instant::now()
                            .checked_add(jwks.max_age)
                            .unwrap_or_else(Instant::now),
                        keys: jwks
                            .keys
                            .into_iter()
                            .map(|key| (key.kid.clone(), key))
                            .collect(),
                    });
                }
                Err(err) => {
                    warn!("Keeping the cached JWKS, refresh failed: {err:?}")
                }
            }
        }
        let cached = self.cached.read().await.clone();
        cached
            .keys
            .get(kid)
            .cloned()
            .ok_or_else(|| anyhow!("Unknown key id {kid}"))
    }

    /// The key, if it is cached, unexpired and has its realm.
    async fn current_key(&self, kid: &str) -> Option<JWKKey> {
        let cached = self.cached.read().await;
        if Instant::now() >= cached.expires_at {
            return None;
        }
        cached
            .keys
            .get(kid)
            .filter(|key| key.realm.is_some())
            .cloned()
    }
}

/// The realm in a Keycloak issuer, `<url>/realms/<realm>`.
fn issuer_realm(iss: &str) -> Option<&str> {
    iss.rsplit_once("/realms/").map(|(_, realm)| realm)
}

/// Whether the claimed tenant, and election event when there is one, are
/// those of the realm. Voters of an event realm may have no event claim.
fn claims_match_realm(claims: &JwtClaims, realm: &str) -> bool {
    let tenant_id = &claims.hasura_claims.tenant_id;
    let tenant_realm = get_tenant_realm(tenant_id);
    match &claims.hasura_claims.election_event_id {
        Some(event_id) => realm == get_event_realm(tenant_id, event_id),
        None => {
            realm == tenant_realm
                || realm.starts_with(&format!("{tenant_realm}-event-"))
        }
    }
}

fn verify_with_key(token: &str, key: &JWKKey) -> Result<JwtClaims> {
    let realm = key
        .realm
        .as_deref()
        .ok_or_else(|| anyhow!("Key {} has no realm", key.kid))?;
    let algorithm = Algorithm::from_str(&key.alg)
        .with_context(|| format!("Key {} is not for signing", key.kid))?;
    let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
        .with_context(|| format!("Key {} is not an RSA key", key.kid))?;
    let mut validation = Validation::new(algorithm);
    validation.validate_aud = false;
    let claims = decode::<JwtClaims>(token, &decoding_key, &validation)
        .context("Invalid token")?
        .claims;
    if issuer_realm(&claims.iss) != Some(realm) {
        bail!("Token from {} signed with a key of {realm}", claims.iss);
    }
    if !claims_match_realm(&claims, realm) {
        bail!(
            "Token of realm {realm} claims tenant {} and election event {:?}",
            claims.hasura_claims.tenant_id,
            claims.hasura_claims.election_event_id
        );
    }
    Ok(claims)
}

/// Verifies a token's signature and expiry with the key of `certs.json` its
/// header names, and that the token and its tenant and election event claims
/// are of that key's realm.
#[instrument(err, skip_all)]
pub async fn verify_jwt(token: &str) -> Result<JwtClaims> {
    let kid = decode_header(token)
        .context("Invalid token header")?
        .kid
        .ok_or_else(|| anyhow!("Token without a key id"))?;
    let key = JWKS_CACHE.key(&kid).await?;
    verify_with_key(token, &key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lc_rs::encoding::AsDer;
    use aws_lc_rs::rsa::{KeyPair, KeySize, PublicKeyComponents};
    use aws_lc_rs::signature::KeyPair as _;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use base64::Engine;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::{json, Value};

    const TENANT: &str = "t1";
    const EVENT: &str = "e1";
    const KID: &str = "k1";

    fn key_pair(realm: &str) -> (EncodingKey, JWKKey) {
        let pair = KeyPair::generate(KeySize::Rsa2048).unwrap();
        let pkcs8 = pair.as_der().unwrap();
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
            STANDARD.encode(pkcs8.as_ref())
        );
        let public: PublicKeyComponents<Vec<u8>> = pair.public_key().into();
        let jwk = JWKKey {
            alg: "RS256".into(),
            kty: "RSA".into(),
            r#use: "sig".into(),
            n: URL_SAFE_NO_PAD.encode(public.n),
            e: URL_SAFE_NO_PAD.encode(public.e),
            kid: KID.into(),
            x5t: String::new(),
            x5c: vec![],
            realm: Some(realm.into()),
        };
        (EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap(), jwk)
    }

    fn claims(realm: &str, event_id: Option<&str>, exp: i64) -> Value {
        json!({
            "exp": exp, "iat": 0, "jti": "jti", "sub": "user",
            "iss": format!("http://keycloak:8090/realms/{realm}"),
            "aud": "account", "typ": "Bearer", "azp": "voting-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "user",
                "x-hasura-tenant-id": TENANT,
                "x-hasura-user-id": "user",
                "x-hasura-election-event-id": event_id,
                "x-hasura-allowed-roles": ["user"]
            }
        })
    }

    fn sign(key: &EncodingKey, claims: &Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(KID.into());
        encode(&header, claims, key).unwrap()
    }

    fn valid_until() -> i64 {
        chrono::Utc::now().timestamp() + 300
    }

    #[test]
    fn accepts_tokens_of_the_key_realm() {
        let tenant_realm = get_tenant_realm(TENANT);
        let (key, jwk) = key_pair(&tenant_realm);
        let token = sign(&key, &claims(&tenant_realm, None, valid_until()));
        assert!(verify_with_key(&token, &jwk).is_ok());

        let event_realm = get_event_realm(TENANT, EVENT);
        let (key, jwk) = key_pair(&event_realm);
        for event_id in [Some(EVENT), None] {
            let token =
                sign(&key, &claims(&event_realm, event_id, valid_until()));
            assert!(verify_with_key(&token, &jwk).is_ok());
        }
    }

    #[test]
    fn rejects_forged_and_expired_tokens() {
        let realm = get_tenant_realm(TENANT);
        let (key, jwk) = key_pair(&realm);
        let (forger, _) = key_pair(&realm);
        let forged = sign(&forger, &claims(&realm, None, valid_until()));
        assert!(verify_with_key(&forged, &jwk).is_err());

        let expired = sign(&key, &claims(&realm, None, 1));
        assert!(verify_with_key(&expired, &jwk).is_err());

        let unsigned = format!(
            "{}.{}.",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"none","kid":"k1"}"#),
            URL_SAFE_NO_PAD
                .encode(claims(&realm, None, valid_until()).to_string())
        );
        assert!(verify_with_key(&unsigned, &jwk).is_err());
    }

    #[test]
    fn rejects_tokens_outside_the_key_realm() {
        let event_realm = get_event_realm(TENANT, EVENT);
        let (key, jwk) = key_pair(&event_realm);
        // A token naming another realm than the key's.
        let other_issuer = sign(
            &key,
            &claims(&get_tenant_realm(TENANT), None, valid_until()),
        );
        assert!(verify_with_key(&other_issuer, &jwk).is_err());
        // A token claiming another election event than its realm's.
        let other_event =
            sign(&key, &claims(&event_realm, Some("e2"), valid_until()));
        assert!(verify_with_key(&other_event, &jwk).is_err());
        // A token claiming another tenant than its realm's.
        let other_realm = get_tenant_realm("t2");
        let (key, jwk) = key_pair(&other_realm);
        let other_tenant =
            sign(&key, &claims(&other_realm, None, valid_until()));
        assert!(verify_with_key(&other_tenant, &jwk).is_err());
        // A key without a realm.
        let realm = get_tenant_realm(TENANT);
        let (key, mut jwk) = key_pair(&realm);
        jwk.realm = None;
        let token = sign(&key, &claims(&realm, None, valid_until()));
        assert!(verify_with_key(&token, &jwk).is_err());
    }

    #[test]
    fn reads_max_age() {
        let header = |value| Some(HeaderValue::from_static(value));
        assert_eq!(
            max_age(header("public, max-age=30").as_ref()),
            Duration::from_secs(30)
        );
        assert_eq!(max_age(header("no-cache").as_ref()), Duration::ZERO);
        assert_eq!(max_age(None), Duration::ZERO);
    }
}
