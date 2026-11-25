// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Operational Initialize request guard. A direct stream cannot trust a
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
        {
            return None;
        }
        Self::from_pins(
            tenant,
            issuer,
            client,
            audience,
            verified_installed_keys,
        )
    }

    /// Called only by private installation code with the ordinary app's
    /// settings and independent tenant realm/client registration and keys.
    #[cfg(all(
        feature = "felix3-local-initialize",
        not(feature = "operational-signing-demo-fixture")
    ))]
    pub(crate) fn for_local_initialize(
        tenant: Uuid,
        portal_origin: &str,
        portal_keycloak_base: &str,
        portal_client: &str,
        registered_issuer: String,
        registered_client: String,
        verified_installed_keys: JwkSet,
    ) -> Option<Self> {
        if tenant.is_nil()
            || portal_origin != "http://localhost:3002"
            || portal_keycloak_base != "http://localhost:18090/"
            || registered_issuer
                != format!("{portal_keycloak_base}realms/tenant-{tenant}")
            || portal_client != registered_client
            || portal_client.len() > 120
            || portal_client.starts_with("sequent-operational-")
            || !portal_client
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return None;
        }
        Self::from_pins(
            tenant,
            registered_issuer,
            registered_client.clone(),
            registered_client,
            verified_installed_keys,
        )
    }

    fn from_pins(
        tenant: Uuid,
        issuer: String,
        client: String,
        audience: String,
        verified_installed_keys: JwkSet,
    ) -> Option<Self> {
        if client.is_empty()
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

    #[cfg(all(
        feature = "felix3-local-initialize",
        not(feature = "operational-signing-demo-fixture")
    ))]
    pub(crate) fn local_approval_endpoint(
        &self,
        registered_authorization: &str,
        registered_callback: &str,
        approval_client: &str,
    ) -> Option<(Uuid, reqwest::Url)> {
        if self.issuer
            != format!("http://localhost:18090/realms/tenant-{}", self.tenant)
            || registered_authorization
                != format!("{}/protocol/openid-connect/auth", self.issuer)
            || registered_callback
                != "http://localhost:3002/operational-signing/return"
            || approval_client == self.client
            || approval_client == "sequent-operational-demo-14990"
            || approval_client.is_empty()
            || approval_client.len() > 150
            || !approval_client
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return None;
        }
        Some((
            self.tenant,
            reqwest::Url::parse(registered_authorization).ok()?,
        ))
    }

    #[cfg(all(
        feature = "felix3-local-initialize",
        not(feature = "operational-signing-demo-fixture")
    ))]
    pub(crate) fn requires_local_ingress(&self) -> bool {
        self.issuer.starts_with("http://")
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
        #[cfg(all(
            feature = "felix3-local-initialize",
            not(feature = "operational-signing-demo-fixture")
        ))]
        if authority.requires_local_ingress() {
            use crate::routes::operational_initialize::local::LocalIngress;
            if !request
                .rocket()
                .state::<LocalIngress>()
                .is_some_and(|pin| pin.accepts(request))
            {
                return Outcome::Error((Status::Forbidden, ()));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route_services::service_tokens::{self, RealmSigner};
    use crate::test_claims::Claims;
    use rocket::http::Header;
    use rocket::local::asynchronous::Client;
    use serde_json::json;

    #[rocket::get("/actor")]
    fn actor(claims: OperationalPortalClaims) -> String {
        claims.0.sub
    }

    fn fixture_tenant() -> Uuid {
        Uuid::from_u128(0x5b2af5e3_f1cb_4871_893f_2362033c5020)
    }

    #[rocket::async_test]
    async fn no_header_or_claim_can_substitute_for_a_verified_portal_actor() {
        const CLAIMS: &str = "https://hasura.io/jwt/claims";
        let tenant = fixture_tenant();
        let subject = Uuid::new_v4().to_string();
        let issuer =
            format!("https://portal-test.invalid/realms/tenant-{tenant}");
        let signer = RealmSigner::shared();
        let mut payload = service_tokens::issued(
            &Claims::new(&tenant.to_string(), &subject),
            &tenant.to_string(),
        );
        payload["iss"] = json!(issuer);
        payload["aud"] = json!("admin-portal");

        let key_state = OperationalPortalAuth::new(
            tenant,
            issuer,
            "admin-portal".into(),
            "admin-portal".into(),
            signer.jwks(),
        )
        .unwrap();
        let client = Client::tracked(
            rocket::build()
                .mount("/", rocket::routes![actor])
                .manage(key_state),
        )
        .await
        .unwrap();

        let sign = |value: &serde_json::Value| -> Header<'static> {
            Header::new("Authorization", signer.sign_payload(value))
        };
        let success =
            client.get("/actor").header(sign(&payload)).dispatch().await;
        assert_eq!(success.status(), Status::Ok);
        assert_eq!(
            success.into_string().await.as_deref(),
            Some(subject.as_str())
        );
        let missing = client
            .get("/actor")
            .header(Header::new("x-hasura-user-id", subject.clone()))
            .header(Header::new("x-hasura-tenant-id", tenant.to_string()))
            .dispatch()
            .await;
        assert_eq!(missing.status(), Status::Unauthorized);
        let duplicate = client
            .get("/actor")
            .header(sign(&payload))
            .header(sign(&payload))
            .dispatch()
            .await;
        assert_eq!(duplicate.status(), Status::Unauthorized);
        let unresolved =
            Client::tracked(rocket::build().mount("/", rocket::routes![actor]))
                .await
                .unwrap();
        assert_eq!(
            unresolved
                .get("/actor")
                .header(sign(&payload))
                .dispatch()
                .await
                .status(),
            Status::ServiceUnavailable
        );

        let mut mutants = Vec::new();
        for (field, value) in [
            (
                "iss",
                json!("https://elsewhere.invalid/realms/tenant-other"),
            ),
            ("aud", json!("other-client")),
            ("azp", json!("other-client")),
            ("typ", json!("ID")),
            ("sub", json!(Uuid::nil())),
            ("sub", json!("not-an-id")),
            ("exp", json!(chrono::Utc::now().timestamp() - 1)),
        ] {
            let mut invalid = payload.clone();
            invalid[field] = value;
            mutants.push(invalid);
        }
        for field in ["x-hasura-user-id", "x-hasura-tenant-id"] {
            let mut invalid = payload.clone();
            invalid[CLAIMS][field] = json!(Uuid::new_v4());
            mutants.push(invalid);
        }
        for invalid in mutants {
            let response =
                client.get("/actor").header(sign(&invalid)).dispatch().await;
            assert_eq!(response.status(), Status::Unauthorized);
        }
    }

    #[test]
    fn constructor_requires_an_exact_https_tenant_issuer_and_bounded_pins() {
        let tenant = fixture_tenant();
        let path = format!("/realms/tenant-{tenant}");
        for issuer in [
            format!("http://localhost:18090{path}"),
            format!("http://fixture.invalid{path}"),
            format!("https://password@fixture.invalid{path}"),
            format!("https://fixture.invalid{path}?extra=yes"),
            format!("https://fixture.invalid{path}#extra"),
            "https://fixture.invalid/realms/tenant-other".into(),
            "not-a-url".into(),
        ] {
            assert!(OperationalPortalAuth::new(
                tenant,
                issuer,
                "admin-portal".into(),
                "admin-portal".into(),
                RealmSigner::shared().jwks(),
            )
            .is_none());
        }
        let issuer = || format!("https://fixture.invalid{path}");
        let keys = RealmSigner::shared().jwks();
        let empty = JwkSet { keys: vec![] };
        let many = JwkSet {
            keys: vec![keys.keys[0].clone(); 33],
        };
        assert!(OperationalPortalAuth::new(
            tenant,
            issuer(),
            "admin-portal".into(),
            "admin-portal".into(),
            empty
        )
        .is_none());
        assert!(OperationalPortalAuth::new(
            tenant,
            issuer(),
            "admin-portal".into(),
            "admin-portal".into(),
            many
        )
        .is_none());
        assert!(OperationalPortalAuth::new(
            tenant,
            issuer(),
            "".into(),
            "admin-portal".into(),
            keys
        )
        .is_none());
    }

    #[cfg(all(
        feature = "felix3-local-initialize",
        not(feature = "operational-signing-demo-fixture")
    ))]
    #[test]
    fn local_portal_requires_the_registered_ordinary_realm_client_and_signed_actor(
    ) {
        let tenant = Uuid::new_v4();
        let actor = Uuid::new_v4().to_string();
        let issuer = format!("http://localhost:18090/realms/tenant-{tenant}");
        let base = "http://localhost:18090/";
        let portal = "http://localhost:3002";
        let client = "synthetic-admin";
        let signer = RealmSigner::shared();
        let install =
            |origin: &str, keycloak: &str, realm: &str, registration: &str| {
                OperationalPortalAuth::for_local_initialize(
                    tenant,
                    origin,
                    keycloak,
                    client,
                    realm.into(),
                    registration.into(),
                    signer.jwks(),
                )
            };
        assert!(
            install("http://127.0.0.1:3002", base, &issuer, client).is_none()
        );
        assert!(install(portal, "http://localhost:8090/", &issuer, client)
            .is_none());
        assert!(install(
            portal,
            base,
            &issuer.replace("/realms", "/x/../realms"),
            client
        )
        .is_none());
        assert!(install(portal, base, &issuer, "other-client").is_none());
        let authority = install(portal, base, &issuer, client).unwrap();
        let authorization = format!("{issuer}/protocol/openid-connect/auth");
        let callback = "http://localhost:3002/operational-signing/return";
        assert!(authority
            .local_approval_endpoint(&authorization, callback, "synthetic-gold")
            .is_some());
        assert!(authority
            .local_approval_endpoint(&authorization, callback, client)
            .is_none());
        assert!(authority
            .local_approval_endpoint(
                &authorization,
                "http://localhost:3002/*",
                "gold"
            )
            .is_none());
        let mut claims = service_tokens::issued(
            &Claims::new(&tenant.to_string(), &actor),
            &tenant.to_string(),
        );
        claims["iss"] = json!(issuer);
        claims["azp"] = json!(client);
        claims["aud"] = json!(client);
        let valid = |payload: &serde_json::Value| {
            let bearer = signer.sign_payload(payload);
            authority
                .validate(bearer.strip_prefix("Bearer ").unwrap())
                .is_some()
        };
        assert!(valid(&claims));
        for (key, value) in [
            (
                "iss",
                json!(issuer.replace("localhost:18090", "keycloak:8090")),
            ),
            ("sub", json!(Uuid::new_v4())),
            ("azp", json!("synthetic-gold")),
            ("aud", json!("another-client")),
            ("typ", json!("ID")),
            ("exp", json!(chrono::Utc::now().timestamp() - 1)),
        ] {
            let mut wrong = claims.clone();
            wrong[key] = value;
            assert!(!valid(&wrong));
        }
    }
}
