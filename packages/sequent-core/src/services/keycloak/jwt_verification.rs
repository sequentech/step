// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{get_event_realm, get_tenant_realm};
use crate::services::jwt::JwtClaims;
use anyhow::{anyhow, ensure, Context, Result};
use base64::Engine;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::{redirect::Policy, Client, Url};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

const KEYCLOAK_URL: &str = "KEYCLOAK_URL";
const KEYCLOAK_PUBLIC_URL: &str = "KEYCLOAK_PUBLIC_URL";
const KIOSK_KEYCLOAK_URL: &str = "KIOSK_KEYCLOAK_URL";
const HARVEST_JWT_ISSUER_URLS: &str = "HARVEST_JWT_ISSUER_URLS";
/// Optional settings, each a comma-separated list of further issuer bases.
const OPTIONAL_ISSUER_SETTINGS: [&str; 3] = [
    KEYCLOAK_PUBLIC_URL,
    KIOSK_KEYCLOAK_URL,
    HARVEST_JWT_ISSUER_URLS,
];

const CACHE_TTL: Duration = Duration::from_secs(300);
const REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CACHED_REALMS: usize = 256;
const MAX_JWKS_BYTES: usize = 1024 * 1024;
const MAX_JWKS_KEYS: usize = 128;
const MAX_CONCURRENT_DOWNLOADS: usize = 32;
const MAX_TOKEN_BYTES: usize = 64 * 1024;
const MAX_REALM_NAME_LEN: usize = 255;

static DOWNLOADS: Semaphore = Semaphore::const_new(MAX_CONCURRENT_DOWNLOADS);

#[derive(Default)]
struct CachedKeys {
    keys: Option<Arc<JwkSet>>,
    fetched: Option<Instant>,
    attempted: Option<Instant>,
}

type RealmCache = HashMap<String, (Instant, Arc<Mutex<CachedKeys>>)>;

fn cache() -> &'static Mutex<RealmCache> {
    static CACHE: OnceLock<Mutex<RealmCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn configured_base(base: &str) -> Result<Url> {
    let url = Url::parse(base)?;
    ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        "Keycloak URL must be an http(s) URL with a host"
    );
    ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Keycloak URL must not contain credentials, a query or a fragment"
    );
    Ok(url)
}

/// Issuers whose bearer tokens are accepted, and the server their signing keys
/// are downloaded from. Every URL is validated once, when the value is built,
/// and applications register it as Rocket managed state for the guards.
#[derive(Debug)]
pub struct BearerIssuers {
    internal_base: Url,
    trusted_bases: Vec<Url>,
}

impl BearerIssuers {
    /// Trusts `trusted_bases` and downloads signing keys from
    /// `internal_base`, which is trusted as an issuer only if also listed.
    pub fn new(internal_base: &str, trusted_bases: &[&str]) -> Result<Self> {
        Ok(Self {
            internal_base: configured_base(internal_base)?,
            trusted_bases: trusted_bases
                .iter()
                .map(|base| configured_base(base))
                .collect::<Result<_>>()?,
        })
    }

    /// Reads the issuer settings through `var`: `KEYCLOAK_URL`, which is
    /// required, trusted and used for key downloads, and the optional
    /// comma-separated `KEYCLOAK_PUBLIC_URL`, `KIOSK_KEYCLOAK_URL` and
    /// `HARVEST_JWT_ISSUER_URLS`. Errors name the setting, never its value.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let internal = var(KEYCLOAK_URL)
            .with_context(|| format!("{KEYCLOAK_URL} must be set"))?;
        let internal_base = setting_base(KEYCLOAK_URL, &internal)?;
        let mut trusted_bases = vec![internal_base.clone()];
        for name in OPTIONAL_ISSUER_SETTINGS {
            for base in var(name)
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|base| !base.is_empty())
            {
                trusted_bases.push(setting_base(name, base)?);
            }
        }
        Ok(Self {
            internal_base,
            trusted_bases,
        })
    }
}

fn setting_base(name: &str, base: &str) -> Result<Url> {
    configured_base(base)
        .with_context(|| format!("Invalid {name} issuer configuration"))
}

fn realm_url(base: &Url, realm: &str) -> String {
    format!("{}/realms/{realm}", base.as_str().trim_end_matches('/'))
}

/// The payload is only a routing hint until signature verification completes.
/// It can select a realm below a configured origin, never a new key server.
fn trusted_realm(claims: &JwtClaims, trusted_bases: &[Url]) -> Result<String> {
    let realm = trusted_bases
        .iter()
        .find_map(|base| claims.iss.strip_prefix(&realm_url(base, "")))
        .context("Untrusted issuer")?;
    ensure!(
        !realm.is_empty()
            && realm.len() <= MAX_REALM_NAME_LEN
            && realm
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "Invalid realm"
    );
    let tenant = &claims.hasura_claims.tenant_id;
    let event_matches = match &claims.hasura_claims.election_event_id {
        Some(event) => realm == get_event_realm(tenant, event),
        // Existing event templates omit this optional claim. Their signature
        // still has to verify with keys from this exact tenant/event realm.
        None => realm
            .strip_prefix(&get_event_realm(tenant, ""))
            .is_some_and(|event| !event.is_empty()),
    };
    ensure!(
        realm == get_tenant_realm(tenant) || event_matches,
        "Issuer does not match tenant/event"
    );
    Ok(realm.to_owned())
}

fn verify_with_jwks<T: DeserializeOwned>(
    token: &str,
    issuer: &str,
    keys: &JwkSet,
) -> Result<T> {
    let header = decode_header(token)?;
    // Keycloak's default realm signing algorithm. Symmetric keys must never be
    // accepted as an alternative to the public realm signing key.
    ensure!(
        header.alg == Algorithm::RS256,
        "Unsupported signing algorithm"
    );
    let kid = header.kid.context("Missing signing key ID")?;
    let jwk = keys.find(&kid).context("Unknown signing key")?;
    let key = DecodingKey::from_jwk(jwk)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[issuer]);
    validation.set_required_spec_claims(&["exp", "iss", "sub"]);
    validation.validate_nbf = true;
    validation.leeway = 0;
    // Existing Keycloak voter/service tokens can omit aud or use "account";
    // these APIs have always authorized the verified realm's Hasura claims.
    validation.validate_aud = false;
    Ok(decode::<T>(token, &key, &validation)?.claims)
}

fn jwks_client() -> Result<&'static Client> {
    static CLIENT: OnceLock<std::result::Result<Client, reqwest::Error>> =
        OnceLock::new();
    match CLIENT.get_or_init(|| {
        Client::builder()
            .redirect(Policy::none())
            .timeout(DOWNLOAD_TIMEOUT)
            .build()
    }) {
        Ok(client) => Ok(client),
        Err(error) => Err(anyhow!("Cannot build signing-key client: {error}")),
    }
}

async fn download_keys(base: &Url, realm: &str) -> Result<JwkSet> {
    let endpoint =
        format!("{}/protocol/openid-connect/certs", realm_url(base, realm));
    let mut response = jwks_client()?.get(endpoint).send().await?;
    ensure!(
        response.status().is_success(),
        "Key response status {}",
        response.status()
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= MAX_JWKS_BYTES,
            "Key response is too large"
        );
        bytes.extend_from_slice(&chunk);
    }
    let keys: JwkSet = serde_json::from_slice(&bytes)?;
    ensure!(keys.keys.len() <= MAX_JWKS_KEYS, "Too many signing keys");
    Ok(keys)
}

/// Unverified hints can name arbitrary realms. Only idle entries may be
/// dropped, and those that never produced keys go before any that did.
fn make_room(entries: &mut RealmCache) -> Result<()> {
    let key = entries
        .iter()
        .filter(|(_, (_, keys))| Arc::strong_count(keys) == 1)
        .min_by_key(|(_, (used, keys))| {
            let verified =
                keys.try_lock().map_or(true, |cached| cached.keys.is_some());
            (verified, *used)
        })
        .map(|(key, _)| key.clone())
        .context("Too many signing-key realms in use")?;
    entries.remove(&key);
    Ok(())
}

async fn realm_keys(
    downloads: &Semaphore,
    base: &Url,
    realm: &str,
    kid: &str,
) -> Result<Arc<JwkSet>> {
    let cache_key = realm_url(base, realm);
    let entry = {
        let mut entries = cache().lock().await;
        if !entries.contains_key(&cache_key)
            && entries.len() >= MAX_CACHED_REALMS
        {
            make_room(&mut entries)?;
        }
        let (used, keys) = entries
            .entry(cache_key)
            .or_insert_with(|| (Instant::now(), Arc::default()));
        *used = Instant::now();
        Arc::clone(keys)
    };
    // Coalesce concurrent downloads for a realm, including invalid-token floods.
    let mut cached = entry.lock().await;
    if let Some(keys) = &cached.keys {
        let fresh = cached
            .fetched
            .is_some_and(|time| time.elapsed() < CACHE_TTL);
        if fresh && keys.find(kid).is_some() {
            return Ok(Arc::clone(keys));
        }
    }
    let retry_allowed = match cached.attempted {
        Some(time) => time.elapsed() >= REFRESH_INTERVAL,
        None => true,
    };
    ensure!(retry_allowed, "Signing-key refresh is rate limited");
    let result = {
        let _permit = downloads.try_acquire().map_err(|_| {
            anyhow!("Too many concurrent signing-key downloads")
        })?;
        download_keys(base, realm).await
    };
    let completed = Instant::now();
    cached.attempted = Some(completed);
    // Keep still-fresh keys after a transient failure, without extending their
    // successful-fetch timestamp or ever serving them after CACHE_TTL.
    let keys = Arc::new(result?);
    cached.fetched = Some(completed);
    cached.keys = Some(Arc::clone(&keys));
    Ok(keys)
}

/// Verifies the signature, expiry and issuer of `token` and returns its claims.
pub(crate) async fn verify_bearer(
    token: &str,
    issuers: &BearerIssuers,
) -> Result<JwtClaims> {
    ensure!(token.len() <= MAX_TOKEN_BYTES, "Token is too large");
    let header = decode_header(token)?;
    ensure!(
        header.alg == Algorithm::RS256,
        "Unsupported signing algorithm"
    );
    let kid = header.kid.context("Missing signing key ID")?;
    let payload = token.split('.').nth(1).unwrap_or_default();
    let payload: JwtClaims = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?,
    )?;
    let realm = trusted_realm(&payload, &issuers.trusted_bases)?;
    let keys =
        realm_keys(&DOWNLOADS, &issuers.internal_base, &realm, &kid).await?;
    let verified: JwtClaims = verify_with_jwks(token, &payload.iss, &keys)?;
    trusted_realm(&verified, &issuers.trusted_bases)?;
    Ok(verified)
}

// The tests and fixtures keep their path from before the move here: the
// coverage gate measures both compared revisions with one exclusion list.
#[cfg(test)]
#[path = "../jwt_verification/tests.rs"]
mod tests;
