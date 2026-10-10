// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, ensure, Context, Result};
use base64::Engine;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::{redirect::Policy, Client, Url};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

const CACHE_TTL: Duration = Duration::from_secs(300);
const REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const MAX_CACHED_REALMS: usize = 256;
const MAX_JWKS_BYTES: usize = 1024 * 1024;
const MAX_CONCURRENT_DOWNLOADS: usize = 32;

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

/// The payload is only a routing hint until signature verification completes.
/// It can select a realm below a configured origin, never a new key server.
fn trusted_realm(payload: &Value, trusted_bases: &[&str]) -> Result<String> {
    let issuer = payload["iss"].as_str().context("Missing issuer")?;
    let realm = trusted_bases
        .iter()
        .find_map(|base| {
            let base = configured_base(base).ok()?;
            issuer.strip_prefix(&format!(
                "{}/realms/",
                base.as_str().trim_end_matches('/')
            ))
        })
        .context("Untrusted issuer")?;
    ensure!(
        !realm.is_empty()
            && realm.len() <= 255
            && realm
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "Invalid realm"
    );
    let claims = &payload["https://hasura.io/jwt/claims"];
    let tenant = claims["x-hasura-tenant-id"]
        .as_str()
        .context("Missing tenant")?;
    let tenant_realm = format!("tenant-{tenant}");
    let event_prefix = format!("{tenant_realm}-event-");
    let event_matches = match claims["x-hasura-election-event-id"].as_str() {
        Some(event) => realm == format!("{event_prefix}{event}"),
        // Existing event templates omit this optional claim. Their signature
        // still has to verify with keys from this exact tenant/event realm.
        None => realm
            .strip_prefix(&event_prefix)
            .is_some_and(|event| !event.is_empty()),
    };
    ensure!(
        realm == tenant_realm || event_matches,
        "Issuer does not match tenant/event"
    );
    Ok(realm.to_owned())
}

fn verify_with_jwks(token: &str, issuer: &str, keys: &JwkSet) -> Result<Value> {
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
    Ok(decode::<Value>(token, &key, &validation)?.claims)
}

fn jwks_client() -> Result<&'static Client> {
    static CLIENT: OnceLock<std::result::Result<Client, reqwest::Error>> =
        OnceLock::new();
    match CLIENT.get_or_init(|| {
        Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
    }) {
        Ok(client) => Ok(client),
        Err(error) => {
            Err(anyhow!("Cannot initialize signing-key client: {error}"))
        }
    }
}

async fn download_keys(base: &str, realm: &str) -> Result<JwkSet> {
    let base = configured_base(base)?;
    let endpoint = format!(
        "{}/realms/{realm}/protocol/openid-connect/certs",
        base.as_str().trim_end_matches('/')
    );
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
    ensure!(keys.keys.len() <= 128, "Too many signing keys");
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
    base: &str,
    realm: &str,
    kid: &str,
) -> Result<Arc<JwkSet>> {
    let cache_key = format!("{base}/realms/{realm}");
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

pub(super) async fn verify_bearer(token: &str) -> Result<Value> {
    let internal_base =
        std::env::var("KEYCLOAK_URL").context("KEYCLOAK_URL must be set")?;
    validate_configured_base("KEYCLOAK_URL", &internal_base)?;
    let mut bases = vec![internal_base.clone()];
    for variable in [
        "KEYCLOAK_PUBLIC_URL",
        "KIOSK_KEYCLOAK_URL",
        "HARVEST_JWT_ISSUER_URLS",
    ] {
        if let Ok(value) = std::env::var(variable) {
            for base in value
                .split(',')
                .map(str::trim)
                .filter(|base| !base.is_empty())
            {
                validate_configured_base(variable, base)?;
                bases.push(base.to_owned());
            }
        }
    }
    let trusted_bases: Vec<_> = bases.iter().map(String::as_str).collect();
    verify_bearer_with_config(token, &trusted_bases, &internal_base).await
}

fn validate_configured_base(variable: &str, base: &str) -> Result<()> {
    configured_base(base).map(|_| ()).map_err(|error| {
        // Report the setting name without logging a potentially sensitive value.
        tracing::warn!(
            setting = variable,
            "Invalid Keycloak issuer configuration"
        );
        error.context(format!("Invalid {variable} issuer configuration"))
    })
}

pub(super) async fn verify_bearer_with_config(
    token: &str,
    trusted_bases: &[&str],
    internal_base: &str,
) -> Result<Value> {
    ensure!(token.len() <= 64 * 1024, "Token is too large");
    let header = decode_header(token)?;
    ensure!(
        header.alg == Algorithm::RS256,
        "Unsupported signing algorithm"
    );
    let kid = header.kid.context("Missing signing key ID")?;
    let payload = token.split('.').nth(1).unwrap_or_default();
    let payload: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?,
    )?;
    let realm = trusted_realm(&payload, trusted_bases)?;
    let keys = realm_keys(&DOWNLOADS, internal_base, &realm, &kid).await?;
    let issuer = payload["iss"].as_str().unwrap_or_default();
    let verified = verify_with_jwks(token, issuer, &keys)?;
    trusted_realm(&verified, trusted_bases)?;
    Ok(verified)
}

#[cfg(test)]
#[path = "jwt_verification/tests.rs"]
mod tests;
