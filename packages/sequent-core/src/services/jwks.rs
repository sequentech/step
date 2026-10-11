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
use tokio::sync::{Mutex, RwLock};
use tracing::{instrument, warn};
use x509_cert::der::Decode;
use x509_cert::Certificate;

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
                directive.trim().strip_prefix("max-age=")?.parse().ok()
            })
        })
        .map(Duration::from_secs)
        .unwrap_or_default()
}

static HTTP_CLIENT: LazyLock<reqwest::Client> =
    LazyLock::new(reqwest::Client::new);

/// Reads `certs.json` from `AWS_S3_PRIVATE_URI`/`AWS_S3_PUBLIC_BUCKET`, where
/// Windmill writes it. A missing file has no keys.
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
    let response = HTTP_CLIENT
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

/// `certs.json`, kept for its `max-age`. Known keys are used right away; once
/// expired, it is refreshed in the background. Only a token signed with a key
/// that is not cached waits for a refresh. Refreshes happen at most once every
/// `AWS_S3_JWKS_MIN_REFRESH_SECS`, and when one fails the cached keys stay in
/// use.
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

impl JwksCache {
    async fn key(&'static self, kid: &str) -> Result<JWKKey> {
        let cached = self.cached.read().await.clone();
        if let Some(key) = cached.keys.get(kid) {
            if Instant::now() >= cached.expires_at {
                tokio::spawn(self.refresh(false));
            }
            return Ok(key.clone());
        }
        self.refresh(true).await;
        self.cached
            .read()
            .await
            .keys
            .get(kid)
            .cloned()
            .ok_or_else(|| anyhow!("Unknown key id {kid}"))
    }

    /// Reads `certs.json` again, unless it was read within the minimum
    /// interval. With `wait`, waits for a refresh in progress instead of
    /// leaving it to finish.
    async fn refresh(&self, wait: bool) {
        let mut last_refresh = if wait {
            self.last_refresh.lock().await
        } else {
            match self.last_refresh.try_lock() {
                Ok(last_refresh) => last_refresh,
                Err(_) => return,
            }
        };
        let min_refresh = seconds_setting(
            JWKS_MIN_REFRESH_ENV,
            DEFAULT_JWKS_MIN_REFRESH_SECS,
        );
        if last_refresh.is_some_and(|at| at.elapsed() < min_refresh) {
            return;
        }
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
}

/// The key's realm: its `realm` member, or for keys written before Windmill
/// added it, the subject of its certificate, which Keycloak issues as
/// `CN=<realm>`.
fn key_realm(key: &JWKKey) -> Option<String> {
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
        // A certificate that Keycloak issued for a development realm's key.
        const CERTIFICATE: &str = "MIIC5TCCAc0CBgGhJ+Y2ezANBgkqhkiG9w0BAQsFADA2MTQwMgYDVQQDDCt0ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1MB4XDTI2MTAxMDIyMTc0MVoXDTM2MTAxMDIyMTkyMVowNjE0MDIGA1UEAwwrdGVuYW50LTkwNTA1YzhhLTIzYTktNGNkZi1hMjZiLTRlMTlmNmEwOTdkNTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAKeVqyvQBwiF7/hUGAHMtdq4Pjd/00ReUaHUOOvP9PM6MmVHVaUDdJ4gjedDgvD+Aa9Gm/KsHwVHvQ/Vl3ZWfG6zmVdyZwSon5ogamKJxBQNc+YViadjZtnh3+TZdgAmny/KUanlaLbkqxRYo3r2KwEqH8QQDyKMXhrKrSuX8F2R2uU43uhUY8iUiffwHgkNKSk0u+YxRngniTQieSaE2aTYu645g2ufUcXzN2o1qwjxCYSGRYGN29ofjaAMC+4jJFWeGGefmaeNbo6kagQj4YA1dLG1YE+qCVSkvG4KCQNSpCHxoMcQASTWwg9Q1ul25Jiq96Z/RR73KD3NTBzeMpMCAwEAATANBgkqhkiG9w0BAQsFAAOCAQEAdmy5ISrIf6hPTOJOd6Hi/c7+0Jd3URlVR/wHRro4ArSEtqyz/LEhfsMXtxVBofWU1i3VZCZv4T8Ie7pwEXugMRLMapqkHl+DT1b2sKW8SzFwf7mnNX53Kidy2oYXP1kzm9a7+y7imidu1NnyZjJ2r26RoQ++/xhSISeAAd4+wVxNKrrcXPQH45yz4yfsVrXg+2vlY8qfsNc8yX6yZC6WrML8YJ3vNFNmh6gJGR0Ya/GtrLCVgIRO1z7e+V/YGI76C6YTZhrAezvjsK5wvCzj6zlLmN7stjiX8MVK2EB7Cmwyj8klafe1kB4unMJ874ako2BT9Ee9a6671J7aZavdOQ==";
        let (_, mut jwk) = key_pair("ignored");
        jwk.realm = None;
        jwk.x5c = vec![CERTIFICATE.into()];
        assert_eq!(
            key_realm(&jwk).as_deref(),
            Some("tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5")
        );
        jwk.realm = Some("tenant-t1".into());
        assert_eq!(key_realm(&jwk).as_deref(), Some("tenant-t1"));
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
