// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Unmounted operational route guard. A direct stream cannot trust a
//! decoded JWT or security headers supplied by the browser. Use the same
//! approved realm issuer and exact Admin Portal client/audience settings,
//! verified locally; installation must provision the existing pinned keys.

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use sequent_core::services::jwt::JwtClaims;
use uuid::Uuid;

const MAX_TOKEN_LEN: usize = 32768;

pub(crate) struct OperationalPortalAuth {
    tenant: Uuid,
    issuer: String,
    client: String,
    audience: String,
    // Outside the normal admin-route User JWT decoder. Build only from
    // immutable exact preprovisioned realm JWKS, not discovery from a claim.
    keys: JwkSet,
}

impl OperationalPortalAuth {
    pub(crate) fn new(
        tenant: Uuid,
        issuer: String,
        client: String,
        audience: String,
        verified_installed_keys: JwkSet,
    ) -> Option<Self> {
        let parsed = reqwest::Url::parse(&issuer).ok()?;
        if tenant.is_nil()
            || parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.path().ends_with(&format!("/realms/tenant-{tenant}"))
            || client.is_empty()
            || audience.is_empty()
            || verified_installed_keys.keys.is_empty()
            || verified_installed_keys.keys.len() > 32
        {
            return None;
        }
        Some(Self {
            tenant,
            issuer,
            client,
            audience,
            keys: verified_installed_keys,
        })
    }

    fn validate(&self, token: &str) -> Option<JwtClaims> {
        if token.len() > MAX_TOKEN_LEN || token.is_empty() {
            return None;
        }
        let header = decode_header(token).ok()?;
        if header.alg != Algorithm::RS256 {
            return None;
        }
        let id = header.kid.as_deref()?.trim();
        if id.is_empty() {
            return None;
        }
        let mut candidates = self
            .keys
            .keys
            .iter()
            .filter(|key| key.common.key_id.as_deref() == Some(id));
        let key = DecodingKey::from_jwk(candidates.next()?).ok()?;
        if candidates.next().is_some() {
            return None;
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.validate_nbf = true;
        validation.leeway = 0;
        let claims = decode::<JwtClaims>(token, &key, &validation).ok()?.claims;
        let actor = Uuid::parse_str(&claims.sub).ok()?;
        if actor.is_nil()
            || claims.azp != self.client
            || claims.typ != "Bearer"
            || claims.hasura_claims.user_id != actor.to_string()
            || claims.hasura_claims.tenant_id != self.tenant.to_string()
        {
            return None;
        }
        Some(claims)
    }
}

/// This type is never built from x-hasura headers or an unsigned token.
pub(crate) struct OperationalPortalClaims(pub JwtClaims);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OperationalPortalClaims {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        let Some(authority) = request.rocket().state::<OperationalPortalAuth>()
        else {
            return Outcome::Error((Status::ServiceUnavailable, ()));
        };
        let mut headers = request.headers().get("Authorization");
        let Some(token) = headers
            .next()
            .and_then(|value| value.strip_prefix("Bearer "))
        else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        if headers.next().is_some() {
            return Outcome::Error((Status::Unauthorized, ()));
        }
        match authority.validate(token) {
            Some(claims) => Outcome::Success(Self(claims)),
            None => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
