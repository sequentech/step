// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The official-approval gate's check of a step-up token: the official's own
//! access token, forwarded by the caller, verified here against the realm's
//! keys. It must be fresh (`auth_time`), carry a step-up `acr`, name this
//! service as audience and, where configured, the tenant; and each token
//! (`jti`) authorizes one signature only.
//!
//! The realm's keys are fetched the first time an official signs, not at
//! start-up, so the service doesn't depend on Keycloak to start (Keycloak's
//! own keys may come from this service).

use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::Deserialize;

use crate::policy::OfficialIssuer;

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    jti: Option<String>,
    exp: i64,
    auth_time: Option<i64>,
    acr: Option<String>,
    #[serde(rename = "tenant-id")]
    tenant_id: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StepUpError {
    #[error("the step-up token is invalid: {0}")]
    Invalid(String),
    #[error("the step-up login is too old")]
    Stale,
    #[error("the login was not a step-up login")]
    NotStepUp,
    #[error("the token was issued for another tenant")]
    WrongTenant,
    #[error("the token was already used for a signature")]
    Replayed,
    #[error("the realm's keys could not be fetched")]
    Keys,
}

pub struct StepUpVerifier {
    issuer: OfficialIssuer,
    keys: Mutex<Option<(JwkSet, Instant)>>,
    // jti → expiry (unix seconds), pruned as tokens expire.
    used: Mutex<HashMap<String, i64>>,
    /// Supplied directly in tests, instead of fetched.
    fixed_keys: Option<JwkSet>,
}

const KEYS_TTL: Duration = Duration::from_secs(300);

/// The verified official.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Official {
    pub subject: String,
    pub auth_time: i64,
}

impl StepUpVerifier {
    pub fn new(issuer: OfficialIssuer) -> Self {
        StepUpVerifier {
            issuer,
            keys: Mutex::new(None),
            used: Mutex::new(HashMap::new()),
            fixed_keys: None,
        }
    }

    pub fn with_keys(issuer: OfficialIssuer, keys: JwkSet) -> Self {
        StepUpVerifier {
            issuer,
            keys: Mutex::new(None),
            used: Mutex::new(HashMap::new()),
            fixed_keys: Some(keys),
        }
    }

    async fn key_set(&self, refresh: bool) -> Result<JwkSet, StepUpError> {
        if let Some(keys) = &self.fixed_keys {
            return Ok(keys.clone());
        }
        if !refresh {
            if let Ok(guard) = self.keys.lock() {
                if let Some((keys, at)) = guard.as_ref() {
                    if at.elapsed() < KEYS_TTL {
                        return Ok(keys.clone());
                    }
                }
            }
        }
        let keys: JwkSet = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| StepUpError::Keys)?
            .get(&self.issuer.jwks_url)
            .send()
            .await
            .map_err(|_| StepUpError::Keys)?
            .json()
            .await
            .map_err(|_| StepUpError::Keys)?;
        if let Ok(mut guard) = self.keys.lock() {
            *guard = Some((keys.clone(), Instant::now()));
        }
        Ok(keys)
    }

    /// Verifies the token and consumes its `jti`. `now` is unix seconds.
    pub async fn verify(&self, token: &str, now: i64) -> Result<Official, StepUpError> {
        let header = decode_header(token).map_err(|e| StepUpError::Invalid(e.to_string()))?;
        let kid = header
            .kid
            .clone()
            .ok_or_else(|| StepUpError::Invalid("no kid".to_string()))?;
        let mut keys = self.key_set(false).await?;
        if keys.find(&kid).is_none() {
            keys = self.key_set(true).await?;
        }
        let jwk = keys
            .find(&kid)
            .ok_or_else(|| StepUpError::Invalid("unknown signing key".to_string()))?;
        let key = DecodingKey::from_jwk(jwk).map_err(|e| StepUpError::Invalid(e.to_string()))?;
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&self.issuer.issuer]);
        validation.set_audience(&[&self.issuer.audience]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let claims = decode::<Claims>(token, &key, &validation)
            .map_err(|e| StepUpError::Invalid(e.to_string()))?
            .claims;
        let auth_time = claims.auth_time.ok_or(StepUpError::Stale)?;
        if now - auth_time > self.issuer.max_auth_age_seconds || auth_time > now + 30 {
            return Err(StepUpError::Stale);
        }
        let acr: BTreeSet<String> = claims.acr.into_iter().collect();
        if acr.is_disjoint(&self.issuer.acr_values) {
            return Err(StepUpError::NotStepUp);
        }
        if let Some(tenant) = &self.issuer.tenant_id {
            if claims.tenant_id.as_ref() != Some(tenant) {
                return Err(StepUpError::WrongTenant);
            }
        }
        let jti = claims
            .jti
            .ok_or_else(|| StepUpError::Invalid("no jti".to_string()))?;
        let mut used = self.used.lock().map_err(|_| StepUpError::Replayed)?;
        used.retain(|_, expiry| *expiry > now);
        if used.insert(jti, claims.exp).is_some() {
            return Err(StepUpError::Replayed);
        }
        Ok(Official {
            subject: claims.sub,
            auth_time,
        })
    }
}
