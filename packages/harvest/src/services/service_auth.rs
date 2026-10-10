// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keycloak's service account on the internal sending API (`/messages/*`).
//!
//! Harvest is internal, but anything on that network could otherwise call
//! these routes with a token it made up: the general `JwtClaims` guard
//! decodes the payload without checking its signature, because the other
//! routes are reached through Hasura, which checks it. Keycloak calls
//! `/messages/*` directly, so here the token is verified against the keys
//! of the realm that issued it: signature, expiry, issuer and the
//! `service-account` role. The route then binds the request's tenant (and
//! its election event) to the token's.

use crate::ports::service_keys::ServiceKeySource;
use crate::services::dependencies::HarvestServices;
use deadpool_postgres::GenericClient;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::types::permissions::Permissions;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{error, warn};
use uuid::Uuid;

/// How long a realm's key set is used before it is fetched again.
pub const KEYS_TTL: Duration = Duration::from_secs(10 * 60);
/// A token signed with a key the cached set doesn't have makes the set be
/// fetched again once (Keycloak rotated its keys), but not more often than
/// this per realm, so made-up key ids can't hammer Keycloak.
pub const REFETCH_COOLDOWN: Duration = Duration::from_secs(10);
/// Clock difference allowed with Keycloak on `exp` and `nbf`.
pub const EXPIRY_LEEWAY_SECONDS: u64 = 30;
/// The Keycloak client whose service account Keycloak's extensions send
/// with (`KEYCLOAK_CLIENT_ID`), when the variable is unset. Every realm
/// template names it so.
pub const DEFAULT_SERVICE_CLIENT: &str = "service-account";
/// Keycloak's `typ` of an access token.
const ACCESS_TOKEN_TYPE: &str = "Bearer";
/// Realms whose keys are remembered at once. Tokens name their tenant
/// before they are verified, so the map is bounded.
const MAX_CACHED_REALMS: usize = 1024;

const HASURA_CLAIMS: &str = "https://hasura.io/jwt/claims";
const TENANT_CLAIM: &str = "x-hasura-tenant-id";

/// Why a service token was refused. Each has one reason, used in the log.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceTokenRefusal {
    MISSING_TOKEN,
    MALFORMED_TOKEN,
    UNSUPPORTED_ALGORITHM,
    NO_TENANT,
    KEYS_UNAVAILABLE,
    UNKNOWN_KEY,
    BAD_SIGNATURE,
    EXPIRED,
    NOT_YET_VALID,
    NOT_AN_ACCESS_TOKEN,
    WRONG_CLIENT,
    WRONG_ISSUER,
    NOT_A_SERVICE_ACCOUNT,
}

impl ServiceTokenRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::MISSING_TOKEN => "No bearer token",
            Self::MALFORMED_TOKEN => "The token can't be read",
            Self::UNSUPPORTED_ALGORITHM => {
                "The token isn't signed with a realm key algorithm"
            }
            Self::NO_TENANT => "The token names no tenant",
            Self::KEYS_UNAVAILABLE => "The realm's keys can't be fetched",
            Self::UNKNOWN_KEY => "The token's key isn't one of the realm's",
            Self::BAD_SIGNATURE => "The token's signature doesn't verify",
            Self::EXPIRED => "The token has expired",
            Self::NOT_YET_VALID => "The token isn't valid yet",
            Self::NOT_AN_ACCESS_TOKEN => "The token isn't an access token",
            Self::WRONG_CLIENT => {
                "The token wasn't issued to the platform's service client"
            }
            Self::WRONG_ISSUER => "The token wasn't issued by its tenant realm",
            Self::NOT_A_SERVICE_ACCOUNT => {
                "The token isn't a service account's"
            }
        }
    }

    /// What the caller is answered: 503 when the realm's keys can't be
    /// fetched (try again later), else 401.
    pub fn status(self) -> Status {
        match self {
            Self::KEYS_UNAVAILABLE => Status::ServiceUnavailable,
            _ => Status::Unauthorized,
        }
    }
}

/// Why a verified service token can't act on a request.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceScopeRefusal {
    OTHER_TENANT,
    OTHER_TENANT_EVENT,
}

impl ServiceScopeRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::OTHER_TENANT => "The request is for another tenant",
            Self::OTHER_TENANT_EVENT => {
                "The election event isn't one of the tenant's"
            }
        }
    }
}

/// Keeps the key set of a body of JWKS JSON whose keys this crate can read.
/// One key of an unknown type doesn't hide the others.
pub fn parse_key_set(body: &Value) -> JwkSet {
    let keys = body
        .get("keys")
        .and_then(Value::as_array)
        .map(|keys| {
            keys.iter()
                .filter_map(|key| {
                    serde_json::from_value::<Jwk>(key.clone()).ok()
                })
                .collect()
        })
        .unwrap_or_default();
    JwkSet { keys }
}

#[derive(Default)]
struct RealmKeys {
    keys: Option<JwkSet>,
    fetched_at: Option<Instant>,
}

/// Realm key sets, fetched from the source on first use, after [`KEYS_TTL`],
/// and once more when a token names a key the set doesn't have. A failed
/// fetch keeps the set it had. Also names the client service tokens must be
/// issued to.
pub struct ServiceKeys {
    source: Arc<dyn ServiceKeySource>,
    ttl: Duration,
    cooldown: Duration,
    realms: Mutex<HashMap<String, Arc<tokio::sync::Mutex<RealmKeys>>>>,
    client_id: String,
}

impl ServiceKeys {
    pub fn new(source: Arc<dyn ServiceKeySource>) -> Self {
        Self::with_timing(source, KEYS_TTL, REFETCH_COOLDOWN)
    }

    pub fn with_timing(
        source: Arc<dyn ServiceKeySource>,
        ttl: Duration,
        cooldown: Duration,
    ) -> Self {
        Self {
            source,
            ttl,
            cooldown,
            realms: Mutex::new(HashMap::new()),
            client_id: DEFAULT_SERVICE_CLIENT.to_string(),
        }
    }

    /// Service tokens must be issued to `client_id` (their `azp`).
    pub fn for_client(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    /// The client service tokens must be issued to.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    fn realm(&self, realm: &str) -> Arc<tokio::sync::Mutex<RealmKeys>> {
        let mut realms = self.realms.lock().unwrap_or_else(|e| e.into_inner());
        if !realms.contains_key(realm) && realms.len() >= MAX_CACHED_REALMS {
            realms.clear();
        }
        realms.entry(realm.to_string()).or_default().clone()
    }

    /// The realm's key with this id.
    pub async fn key(
        &self,
        realm: &str,
        kid: &str,
    ) -> Result<Jwk, ServiceTokenRefusal> {
        let entry = self.realm(realm);
        // One fetch per realm at a time; other realms aren't held up.
        let mut cached = entry.lock().await;
        let age = cached.fetched_at.map(|at| at.elapsed());
        let fresh = age.is_some_and(|age| age < self.ttl);
        if fresh {
            if let Some(key) = cached.keys.as_ref().and_then(|k| k.find(kid)) {
                return Ok(key.clone());
            }
            if age.is_some_and(|age| age < self.cooldown) {
                return Err(match cached.keys {
                    Some(_) => ServiceTokenRefusal::UNKNOWN_KEY,
                    None => ServiceTokenRefusal::KEYS_UNAVAILABLE,
                });
            }
        }
        cached.fetched_at = Some(Instant::now());
        match self.source.realm_keys(realm).await {
            Ok(keys) => {
                let key = keys.find(kid).cloned();
                cached.keys = Some(keys);
                key.ok_or(ServiceTokenRefusal::UNKNOWN_KEY)
            }
            Err(error) => {
                // Stale if error: a blip at the TTL boundary must not
                // refuse the tokens the keys it had still verify.
                warn!("keys of realm {realm} unavailable: {error:#}");
                cached
                    .keys
                    .as_ref()
                    .and_then(|keys| keys.find(kid))
                    .cloned()
                    .ok_or(ServiceTokenRefusal::KEYS_UNAVAILABLE)
            }
        }
    }
}

/// A `/messages/*` caller whose token Keycloak signed for a tenant realm,
/// holding the `service-account` role.
#[derive(Debug, Clone)]
pub struct VerifiedServiceClaims {
    pub claims: JwtClaims,
    tenant_id: Uuid,
}

fn tenant_of(token: &str) -> Option<Uuid> {
    let payload =
        jsonwebtoken::dangerous::insecure_decode::<Value>(token).ok()?;
    let tenant = payload.claims.get(HASURA_CLAIMS)?.get(TENANT_CLAIM)?;
    Uuid::parse_str(tenant.as_str()?).ok()
}

/// Algorithms a Keycloak realm signs access tokens with. A symmetric or
/// unsigned token is never accepted, whatever its header says.
fn realm_algorithm(algorithm: Algorithm) -> bool {
    !matches!(
        algorithm,
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
    )
}

/// Verifies the `Authorization` header's token. Everything that can be
/// refused without the realm's keys is refused before they are fetched.
pub async fn verify_service_token(
    keys: &ServiceKeys,
    authorization: Option<&str>,
) -> Result<VerifiedServiceClaims, ServiceTokenRefusal> {
    use ServiceTokenRefusal::*;
    let token = authorization
        .ok_or(MISSING_TOKEN)?
        .strip_prefix("Bearer ")
        .ok_or(MALFORMED_TOKEN)?
        .trim();
    let header = decode_header(token).map_err(|_| MALFORMED_TOKEN)?;
    if !realm_algorithm(header.alg) {
        return Err(UNSUPPORTED_ALGORITHM);
    }
    let kid = header.kid.as_deref().ok_or(MALFORMED_TOKEN)?;
    let tenant_id = tenant_of(token).ok_or(NO_TENANT)?;
    let realm = get_tenant_realm(&tenant_id.to_string());

    let jwk = keys.key(&realm, kid).await?;
    let key = DecodingKey::from_jwk(&jwk).map_err(|_| UNKNOWN_KEY)?;
    let mut validation = Validation::new(header.alg);
    validation.leeway = EXPIRY_LEEWAY_SECONDS;
    validation.validate_nbf = true;
    // Keycloak's client-credentials tokens name no audience of ours; the
    // issuing client is checked as `azp` below.
    validation.validate_aud = false;
    let claims = decode::<JwtClaims>(token, &key, &validation)
        .map_err(|error| match error.kind() {
            ErrorKind::ExpiredSignature => EXPIRED,
            ErrorKind::ImmatureSignature => NOT_YET_VALID,
            ErrorKind::InvalidSignature => BAD_SIGNATURE,
            ErrorKind::InvalidAlgorithm => UNSUPPORTED_ALGORITHM,
            _ => MALFORMED_TOKEN,
        })?
        .claims;

    // An ID or refresh token the realm signed isn't a credential to call
    // with, and only the platform's client sends.
    if claims.typ != ACCESS_TOKEN_TYPE {
        return Err(NOT_AN_ACCESS_TOKEN);
    }
    if claims.azp != keys.client_id() {
        return Err(WRONG_CLIENT);
    }

    // The internal and public Keycloak addresses differ, so only the realm
    // part of the issuer is compared.
    if !claims.iss.ends_with(&format!("/realms/{realm}")) {
        return Err(WRONG_ISSUER);
    }
    if claims.hasura_claims.tenant_id != tenant_id.to_string() {
        return Err(NO_TENANT);
    }
    let service = Permissions::SERVICE_ACCOUNT.to_string();
    if !claims.hasura_claims.allowed_roles.contains(&service) {
        return Err(NOT_A_SERVICE_ACCOUNT);
    }
    Ok(VerifiedServiceClaims { claims, tenant_id })
}

impl VerifiedServiceClaims {
    /// The tenant whose realm signed the token.
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// The request's tenant must be the token's. No super-admin tenant may
    /// act for another one here.
    pub fn require_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Uuid, (Status, String)> {
        match Uuid::parse_str(tenant_id) {
            Ok(id) if id == self.tenant_id => Ok(id),
            Ok(_) | Err(_) => {
                let refusal = ServiceScopeRefusal::OTHER_TENANT;
                warn!("service request refused: {}", refusal.reason());
                Err((Status::Forbidden, refusal.reason().to_string()))
            }
        }
    }

    /// The request's election event, if it names one, must be the token
    /// tenant's.
    pub async fn require_event<C: GenericClient>(
        &self,
        client: &C,
        election_event_id: Option<&str>,
    ) -> Result<Option<Uuid>, (Status, String)> {
        let Some(election_event_id) = election_event_id else {
            return Ok(None);
        };
        let event_id = Uuid::parse_str(election_event_id).map_err(|_| {
            (Status::BadRequest, "invalid identifier".to_string())
        })?;
        let owned: bool = client
            .query_one(
                "SELECT EXISTS (
                     SELECT 1 FROM sequent_backend.election_event
                     WHERE id = $1 AND tenant_id = $2
                 )",
                &[&event_id, &self.tenant_id],
            )
            .await
            .map_err(|error| {
                error!("service request event check failed: {error}");
                (Status::InternalServerError, "internal error".to_string())
            })?
            .get(0);
        if !owned {
            let refusal = ServiceScopeRefusal::OTHER_TENANT_EVENT;
            warn!("service request refused: {}", refusal.reason());
            return Err((Status::Forbidden, refusal.reason().to_string()));
        }
        Ok(Some(event_id))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for VerifiedServiceClaims {
    type Error = ServiceTokenRefusal;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let Some(services) = request.rocket().state::<HarvestServices>() else {
            error!("service token guard: no managed services");
            return Outcome::Error((
                Status::InternalServerError,
                ServiceTokenRefusal::KEYS_UNAVAILABLE,
            ));
        };
        let authorization = request.headers().get_one("authorization");
        match verify_service_token(&services.service_keys, authorization).await
        {
            Ok(claims) => Outcome::Success(claims),
            Err(refusal) => {
                warn!("service token refused: {}", refusal.reason());
                Outcome::Error((refusal.status(), refusal))
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/support/service_auth.rs"]
mod boundary_tests;

#[cfg(test)]
#[path = "../../tests/support/hasura_messaging_permissions.rs"]
mod hasura_messaging_permissions;
