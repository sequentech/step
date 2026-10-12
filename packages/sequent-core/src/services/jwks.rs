// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verifies Keycloak access tokens with `certs.json`, the JWKS that Windmill
//! publishes for Hasura. A token is only trusted for the realm of the key that
//! signed it.

use crate::services::jwt::JwtClaims;
use crate::services::keycloak::parse_realm;
use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::header::{HeaderValue, CACHE_CONTROL};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, MutexGuard, RwLock};
use tracing::{instrument, warn};
use x509_cert::der::Decode;
use x509_cert::Certificate;

const JWKS_CERTS_PATH_ENV: &str = "AWS_S3_JWKS_CERTS_PATH";
const DEFAULT_JWKS_CERTS_PATH: &str = "certs.json";
const PRIVATE_URI_ENV: &str = "AWS_S3_PRIVATE_URI";
const PUBLIC_BUCKET_ENV: &str = "AWS_S3_PUBLIC_BUCKET";
const JWKS_FETCH_TIMEOUT_ENV: &str = "AWS_S3_JWKS_FETCH_TIMEOUT_SECS";
const JWKS_MIN_REFRESH_ENV: &str = "AWS_S3_JWKS_MIN_REFRESH_SECS";
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
    env::var(JWKS_CERTS_PATH_ENV)
        .unwrap_or_else(|_| DEFAULT_JWKS_CERTS_PATH.to_string())
}

/// A positive number of seconds from the environment, else the default.
fn seconds_setting(name: &str, default: u64) -> Duration {
    let Ok(value) = env::var(name) else {
        return Duration::from_secs(default);
    };
    match value.trim().parse::<u64>() {
        Ok(seconds) if seconds > 0 => Duration::from_secs(seconds),
        _ => {
            warn!("{name} must be a positive integer, got {value:?}; using {default}");
            Duration::from_secs(default)
        }
    }
}

/// The `max-age` of a `Cache-Control` header, zero when there is none.
fn max_age(cache_control: Option<&HeaderValue>) -> Duration {
    cache_control
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.split(',').find_map(|directive| {
                let directive = directive.trim().to_ascii_lowercase();
                directive.strip_prefix("max-age=")?.parse().ok()
            })
        })
        .map(Duration::from_secs)
        .unwrap_or_default()
}

static HTTP_CLIENT: LazyLock<reqwest::Client> =
    LazyLock::new(reqwest::Client::new);
static FETCH_TIMEOUT: LazyLock<Duration> = LazyLock::new(|| {
    seconds_setting(JWKS_FETCH_TIMEOUT_ENV, DEFAULT_JWKS_FETCH_TIMEOUT_SECS)
});
static MIN_REFRESH: LazyLock<Duration> = LazyLock::new(|| {
    seconds_setting(JWKS_MIN_REFRESH_ENV, DEFAULT_JWKS_MIN_REFRESH_SECS)
});

/// Reads `certs.json` from `AWS_S3_PRIVATE_URI`/`AWS_S3_PUBLIC_BUCKET`, where
/// Windmill writes it. A missing file has no keys.
#[instrument]
pub async fn fetch_jwks() -> Result<Jwks> {
    let url = format!(
        "{}/{}/{}",
        env::var(PRIVATE_URI_ENV)
            .with_context(|| format!("{PRIVATE_URI_ENV} must be set"))?,
        env::var(PUBLIC_BUCKET_ENV)
            .with_context(|| format!("{PUBLIC_BUCKET_ENV} must be set"))?,
        get_jwks_secret_path()
    );
    let response = HTTP_CLIENT
        .get(url)
        .timeout(*FETCH_TIMEOUT)
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

/// `certs.json`, kept for its `max-age`. Known keys are used right away; once
/// expired, it is refreshed in the background. Only a token signed with a key
/// that is not cached waits for a refresh. Refreshes happen at most once every
/// `AWS_S3_JWKS_MIN_REFRESH_SECS`, and when one fails, or finds no keys, the
/// cached keys stay in use.
struct JwksCache {
    cached: RwLock<Arc<CachedKeys>>,
    /// When the last refresh started. Held while refreshing.
    last_refresh: Mutex<Option<Instant>>,
}

static JWKS_CACHE: LazyLock<JwksCache> = LazyLock::new(|| JwksCache {
    cached: RwLock::new(Arc::new(CachedKeys {
        keys: HashMap::new(),
        expires_at: Instant::now(),
    })),
    last_refresh: Mutex::new(None),
});

/// Whether the minimum interval has passed since the last refresh.
fn refresh_due(last_refresh: &Option<Instant>) -> bool {
    last_refresh.is_none_or(|at| at.elapsed() >= *MIN_REFRESH)
}

impl JwksCache {
    async fn key(&'static self, kid: &str) -> Result<JWKKey> {
        let cached = self.cached.read().await.clone();
        if let Some(key) = cached.keys.get(kid) {
            if Instant::now() >= cached.expires_at {
                if let Ok(last_refresh) = self.last_refresh.try_lock() {
                    if refresh_due(&last_refresh) {
                        tokio::spawn(self.refresh(last_refresh));
                    }
                }
            }
            return Ok(key.clone());
        }
        let last_refresh = self.last_refresh.lock().await;
        if refresh_due(&last_refresh) {
            self.refresh(last_refresh).await;
        }
        self.cached
            .read()
            .await
            .keys
            .get(kid)
            .cloned()
            .ok_or_else(|| anyhow!("Unknown key id {kid}"))
    }

    async fn refresh(&self, mut last_refresh: MutexGuard<'_, Option<Instant>>) {
        *last_refresh = Some(Instant::now());
        let jwks = match fetch_jwks().await {
            Ok(jwks) if !jwks.keys.is_empty() => jwks,
            Ok(_) => {
                warn!("Keeping the cached JWKS, certs.json has no keys");
                return;
            }
            Err(err) => {
                warn!("Keeping the cached JWKS, refresh failed: {err:?}");
                return;
            }
        };
        *self.cached.write().await = Arc::new(CachedKeys {
            expires_at: Instant::now()
                .checked_add(jwks.max_age)
                .unwrap_or_else(Instant::now),
            keys: jwks
                .keys
                .into_iter()
                .map(|key| {
                    let realm = key_realm(&key);
                    (key.kid.clone(), JWKKey { realm, ..key })
                })
                .collect(),
        });
    }
}

/// The key's realm: its `realm` member, or for keys written before Windmill
/// added it, the subject of its certificate, which Keycloak issues as
/// `CN=<realm>`.
pub fn key_realm(key: &JWKKey) -> Option<String> {
    if let Some(realm) = &key.realm {
        return Some(realm.clone());
    }
    let der = STANDARD.decode(key.x5c.first()?).ok()?;
    let subject = Certificate::from_der(&der).ok()?.tbs_certificate.subject;
    subject.to_string().strip_prefix("CN=").map(str::to_string)
}

/// The realm in a Keycloak issuer, `<url>/realms/<realm>`.
fn issuer_realm(iss: &str) -> Option<&str> {
    iss.rsplit_once("/realms/").map(|(_, realm)| realm)
}

/// Whether the claimed tenant, and election event when there is one, are
/// those of the realm. Voters of an event realm may have no event claim.
fn claims_match_realm(claims: &JwtClaims, realm: &str) -> bool {
    let Some((tenant_id, event_id)) = parse_realm(realm) else {
        return false;
    };
    let claims = &claims.hasura_claims;
    claims.tenant_id == tenant_id
        && claims
            .election_event_id
            .as_ref()
            .is_none_or(|claimed| Some(claimed) == event_id.as_ref())
}

fn verify_with_key(token: &str, key: &JWKKey) -> Result<JwtClaims> {
    let realm = key_realm(key)
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
    if issuer_realm(&claims.iss) != Some(realm.as_str()) {
        bail!("Token from {} signed with a key of {realm}", claims.iss);
    }
    if !claims_match_realm(&claims, &realm) {
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
#[instrument(skip_all)]
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
    use crate::services::keycloak::{get_event_realm, get_tenant_realm};
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
        // A key without a realm or certificate.
        let realm = get_tenant_realm(TENANT);
        let (key, mut jwk) = key_pair(&realm);
        jwk.realm = None;
        let token = sign(&key, &claims(&realm, None, valid_until()));
        assert!(verify_with_key(&token, &jwk).is_err());
    }

    #[test]
    fn untagged_keys_have_their_certificate_realm() {
        // A certificate that Keycloak issued for a development event realm's key.
        const CERTIFICATE: &str = "MIIDOzCCAiMCBgGhKATBxDANBgkqhkiG9w0BAQsFADBhMV8wXQYDVQQDDFZ0ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1LWV2ZW50LTc1ZjQzOWYzLTBjNmItNDFlMC05ZDBjLTRlNDNiYzdlNzA0YjAeFw0yNjEwMTAyMjUxMDJaFw0zNjEwMTAyMjUyNDJaMGExXzBdBgNVBAMMVnRlbmFudC05MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUtZXZlbnQtNzVmNDM5ZjMtMGM2Yi00MWUwLTlkMGMtNGU0M2JjN2U3MDRiMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAtFAh7jRLraKuz7YSMMpwT10w0p481UDcH5j++ya9R3rCNAbPFt/v2i404LS5BMmVn6QAm3hRvEWGf58K95cHJFjv8ANJT0JUpb6fweDz8Rrpz/NBObKdN1y3Py1BczbHJ8mv4kfvCf+2M0xMNJuOxH/eSxDVMW8CxkbQmFC3rteXx0Xv/Z5DXk3o+UHfuNYINF7nmIm7Q/Ayz6nJWhhBNDvD677OFKnqrWsAgdtA8JFIKbImvm80w1xJhYdIEQAKYnc7B1tGI2WH8fobDMTm0ay7+7Xdj7pW1YoIeH8wNVVizCjo9VxI+MvolEgT83mvvrFbdZd0kqrQo/ApEp/H3QIDAQABMA0GCSqGSIb3DQEBCwUAA4IBAQCOqX9WJqm65m40TZXdyI65LM2SI3DiWKRHZjKwEBbqaB4ccoyBdoc7NZgWKTozRPT1URPZ5UQaXFaPmMHY4Z1n6gzL6xjvRCLQOKd2l3V/tMe5rfr7gJ6rpFBXXnwuQTBKkb6LXYoGFzweEjAD0pcsKSJNQA89lnvI8LwXh3kr65IiSUGlORBhWzutBt9GrK2bkwqTJWPFn2P/D5cg9/ySZCOYQCVnn+ML6bnGcXURHF2bPtE7bnmBv5EbjSQN8dP0yCK+E9lX62IWiWYPOaF529QbW4Kn4gN06kqrdbVwqYUH8jNXuvS1DfFciPfs7Ez39/6MDL9oosanLOuEm6Z1";
        let (_, mut jwk) = key_pair("ignored");
        jwk.realm = None;
        jwk.x5c = vec![CERTIFICATE.into()];
        assert_eq!(
            key_realm(&jwk).as_deref(),
            Some("tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-75f439f3-0c6b-41e0-9d0c-4e43bc7e704b")
        );
        jwk.realm = Some("tenant-t1".into());
        assert_eq!(key_realm(&jwk).as_deref(), Some("tenant-t1"));
    }

    #[test]
    fn reads_max_age() {
        let header = |value| Some(HeaderValue::from_static(value));
        assert_eq!(
            max_age(header("public, Max-Age=30").as_ref()),
            Duration::from_secs(30)
        );
        assert_eq!(max_age(header("no-cache").as_ref()), Duration::ZERO);
        assert_eq!(max_age(None), Duration::ZERO);
    }
}
