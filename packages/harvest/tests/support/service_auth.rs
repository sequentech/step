// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The verified service boundary of `/messages/*`: a token is accepted only
//! when its tenant realm signed it, it hasn't expired, it holds the
//! service-account role, and the request is for its own tenant.

use super::{
    parse_key_set, verify_service_token, ServiceKeys, ServiceTokenRefusal,
    DEFAULT_SERVICE_CLIENT,
};
use crate::adapters::memory::service_keys::StaticServiceKeys;
use crate::route_services::service_tokens::{issued, realm, RealmSigner, KID};
use crate::route_services::{post_as, Services};
use crate::test_claims::Claims;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Keycloak's service account: a client-credentials access token of the
/// platform's client.
fn keycloak(tenant_id: &str) -> Claims {
    Claims::new(tenant_id, "service-account-keycloak")
        .roles([Permissions::SERVICE_ACCOUNT])
        .azp(DEFAULT_SERVICE_CLIENT)
}

/// An unsigned service token, independent of the shared signed fixture.
fn unsigned(tenant_id: &str) -> String {
    format!(
        "Bearer {}.{}.",
        URL_SAFE_NO_PAD
            .encode(r#"{"alg":"none","typ":"JWT","kid":"realm-signing-key"}"#),
        URL_SAFE_NO_PAD
            .encode(issued(&keycloak(tenant_id), tenant_id).to_string()),
    )
}

fn notice(tenant_id: &str) -> Value {
    json!({
        "tenant_id": tenant_id,
        "election_event_id": null,
        "voter_id": "voter-1",
        "channel": "SMS",
        "purpose": "NOTICE",
        "destination": "+34600000001",
        "language": "en",
        "content": {"text": "Voting opens tomorrow."},
        "logical_key": "forged",
    })
}

/// A realm that published `signer`'s key, and keys that read it.
fn trusting(
    tenant_id: &str,
    signer: &RealmSigner,
) -> (Arc<StaticServiceKeys>, ServiceKeys) {
    let source = Arc::new(StaticServiceKeys::default());
    source.publish(&realm(tenant_id), signer.jwks());
    let keys = ServiceKeys::new(source.clone());
    (source, keys)
}

async fn verify(
    keys: &ServiceKeys,
    authorization: &str,
) -> Result<Uuid, ServiceTokenRefusal> {
    verify_service_token(keys, Some(authorization))
        .await
        .map(|claims| claims.tenant_id())
}

/// Regression (BC-F1): an unsigned token that names the service-account
/// role used to be accepted, so anything on the network could send through
/// a tenant's accounts.
#[rocket::async_test]
async fn a_forged_unsigned_service_token_cannot_send() {
    let services = Services::without_database();
    let client = services.client().await;
    let tenant_id = Uuid::new_v4().to_string();
    let response = post_as(
        &client,
        "/messages/send",
        &unsigned(&tenant_id),
        &notice(&tenant_id),
    )
    .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

/// Regression (R1-12): a token in the realm's shape, naming the realm's
/// key, whose signature is someone else's. Refused at the boundary: this
/// client has no database, so anything past it would answer 500.
#[rocket::async_test]
async fn a_service_token_signed_with_another_key_cannot_send() {
    let services = Services::without_database();
    let client = services.client().await;
    let tenant_id = Uuid::new_v4().to_string();
    services.trust(&tenant_id, RealmSigner::shared());
    let impostor = RealmSigner::new(KID);
    for path in [
        "/messages/send",
        "/messages/link",
        "/messages/link/status",
        "/messages/link/confirm",
    ] {
        let response = post_as(
            &client,
            path,
            &impostor.sign(&keycloak(&tenant_id), &tenant_id),
            &notice(&tenant_id),
        )
        .await;
        assert_eq!(response.status(), Status::Unauthorized, "{path}");
    }
}

/// Regression (R1-11): when Keycloak can't be reached the caller is told
/// to retry (503), not that its token is bad (401).
#[rocket::async_test]
async fn unreachable_realm_keys_answer_503() {
    let services = Services::without_database();
    let client = services.client().await;
    let tenant_id = Uuid::new_v4().to_string();
    // Nothing published: fetching the realm's keys fails.
    let response = post_as(
        &client,
        "/messages/send",
        &RealmSigner::shared().sign(&keycloak(&tenant_id), &tenant_id),
        &notice(&tenant_id),
    )
    .await;
    assert_eq!(response.status(), Status::ServiceUnavailable);
}

/// Regression (R1-11): a failed refetch threw the cached keys away, so
/// every valid token was refused until the next fetch.
#[rocket::async_test]
async fn a_failed_refetch_keeps_the_keys_it_had() {
    let tenant_id = Uuid::new_v4().to_string();
    let signer = RealmSigner::shared();
    let source = Arc::new(StaticServiceKeys::default());
    source.publish(&realm(&tenant_id), signer.jwks());
    // Every use refetches: the TTL has always passed.
    let keys = ServiceKeys::with_timing(
        source.clone(),
        Duration::ZERO,
        Duration::ZERO,
    );
    let token = signer.sign(&keycloak(&tenant_id), &tenant_id);
    assert!(verify(&keys, &token).await.is_ok());
    source.withdraw(&realm(&tenant_id));
    assert!(verify(&keys, &token).await.is_ok());
    assert!(verify(&keys, &token).await.is_ok());
    assert_eq!(source.fetch_count(), 3);
    // A key the stale set doesn't have stays unknown.
    let rotated = RealmSigner::new("rotated");
    assert_eq!(
        verify(&keys, &rotated.sign(&keycloak(&tenant_id), &tenant_id)).await,
        Err(ServiceTokenRefusal::KEYS_UNAVAILABLE)
    );
}

#[rocket::async_test]
async fn messages_routes_refuse_tokens_that_are_not_the_tenants_service_account(
) {
    let services = Services::without_database();
    let client = services.client().await;
    let signer = RealmSigner::shared();
    let tenant_id = Uuid::new_v4().to_string();
    let other_tenant = Uuid::new_v4().to_string();
    services.trust(&tenant_id, signer);
    services.trust(&other_tenant, signer);

    let mut expired = issued(&keycloak(&tenant_id), &tenant_id);
    expired["exp"] = json!(chrono::Utc::now().timestamp() - 3600);
    let voter = Claims::new(&tenant_id, "voter").roles(["user"]);
    for (authorization, status) in [
        (signer.sign_payload(&expired), Status::Unauthorized),
        (signer.sign(&voter, &tenant_id), Status::Unauthorized),
        // A valid service token of another tenant.
        (
            signer.sign(&keycloak(&other_tenant), &other_tenant),
            Status::Forbidden,
        ),
    ] {
        let response = post_as(
            &client,
            "/messages/send",
            &authorization,
            &notice(&tenant_id),
        )
        .await;
        assert_eq!(response.status(), status, "{authorization}");
    }

    // The tenant's own service account gets past the boundary, to the
    // database this client has none of.
    let response = post_as(
        &client,
        "/messages/send",
        &signer.sign(&keycloak(&tenant_id), &tenant_id),
        &notice(&tenant_id),
    )
    .await;
    assert_eq!(response.status(), Status::InternalServerError);
}

#[rocket::async_test]
async fn a_token_signed_by_its_tenant_realm_is_verified() {
    let signer = RealmSigner::shared();
    let tenant_id = Uuid::new_v4().to_string();
    let (_, keys) = trusting(&tenant_id, signer);
    let token = signer.sign(&keycloak(&tenant_id), &tenant_id);
    let claims = verify_service_token(&keys, Some(&token)).await.unwrap();
    assert_eq!(claims.tenant_id().to_string(), tenant_id);
    assert_eq!(claims.require_tenant(&tenant_id), Ok(claims.tenant_id()));
    for other in [Uuid::new_v4().to_string(), "not-a-uuid".to_string()] {
        assert_eq!(
            claims.require_tenant(&other),
            Err((
                Status::Forbidden,
                "The request is for another tenant".to_string()
            ))
        );
    }
}

#[rocket::async_test]
async fn tokens_are_refused_for_one_reason_each() {
    use ServiceTokenRefusal::*;
    let signer = RealmSigner::shared();
    let tenant_id = Uuid::new_v4().to_string();
    let other_tenant = Uuid::new_v4().to_string();
    let (source, keys) = trusting(&tenant_id, signer);
    source.publish(&realm(&other_tenant), signer.jwks());
    let service = issued(&keycloak(&tenant_id), &tenant_id);
    let edited = |field: &str, value: Value| {
        let mut payload = service.clone();
        payload[field] = value;
        signer.sign_payload(&payload)
    };
    let impostor = RealmSigner::new(KID);
    let unsigned = format!(
        "Bearer {}.{}.",
        URL_SAFE_NO_PAD
            .encode(r#"{"alg":"none","typ":"JWT","kid":"realm-signing-key"}"#),
        URL_SAFE_NO_PAD.encode(service.to_string()),
    );
    // Signed with the realm's public modulus as an HMAC secret.
    let mut hmac_header = Header::new(Algorithm::HS256);
    hmac_header.kid = Some(KID.to_string());
    let hmac = format!(
        "Bearer {}",
        encode(
            &hmac_header,
            &service,
            &EncodingKey::from_secret(b"public modulus")
        )
        .unwrap()
    );
    let mut no_tenant = service.clone();
    no_tenant["https://hasura.io/jwt/claims"]["x-hasura-tenant-id"] =
        json!("not-a-uuid");

    for (authorization, refusal) in [
        ("".to_string(), MALFORMED_TOKEN),
        ("Basic dXNlcjpwYXNz".to_string(), MALFORMED_TOKEN),
        (unsigned(&tenant_id), MALFORMED_TOKEN),
        ("Bearer fixture.e30.fixture".to_string(), MALFORMED_TOKEN),
        (unsigned, MALFORMED_TOKEN),
        (hmac, UNSUPPORTED_ALGORITHM),
        (signer.sign_payload(&no_tenant), NO_TENANT),
        (
            impostor.sign(&keycloak(&tenant_id), &tenant_id),
            BAD_SIGNATURE,
        ),
        (
            edited("exp", json!(chrono::Utc::now().timestamp() - 600)),
            EXPIRED,
        ),
        (
            edited(
                "iss",
                json!(format!("http://keycloak/realms/tenant-{other_tenant}")),
            ),
            WRONG_ISSUER,
        ),
        (
            edited("iss", json!(format!("http://keycloak/realms/master"))),
            WRONG_ISSUER,
        ),
        (
            signer.sign(
                &Claims::new(&tenant_id, "admin")
                    .roles([Permissions::MESSAGING_ACCOUNT_WRITE])
                    .azp(DEFAULT_SERVICE_CLIENT),
                &tenant_id,
            ),
            NOT_A_SERVICE_ACCOUNT,
        ),
        // Regression (R1-10): any token the realm signed with the role was
        // taken, whatever it was for.
        (edited("typ", json!("ID")), NOT_AN_ACCESS_TOKEN),
        (edited("typ", json!("Refresh")), NOT_AN_ACCESS_TOKEN),
        (edited("azp", json!("admin-portal")), WRONG_CLIENT),
        (
            // A user of the voting portal given the service-account role.
            signer.sign(&keycloak(&tenant_id).azp("voting-portal"), &tenant_id),
            WRONG_CLIENT,
        ),
        (
            edited("nbf", json!(chrono::Utc::now().timestamp() + 600)),
            NOT_YET_VALID,
        ),
    ] {
        assert_eq!(
            verify(&keys, &authorization).await,
            Err(refusal),
            "{authorization}"
        );
    }
    assert_eq!(
        verify_service_token(&keys, None).await.err(),
        Some(MISSING_TOKEN)
    );

    // A token for a tenant whose realm publishes nothing.
    let stranger = Uuid::new_v4().to_string();
    assert_eq!(
        verify(&keys, &signer.sign(&keycloak(&stranger), &stranger)).await,
        Err(KEYS_UNAVAILABLE)
    );
}

#[rocket::async_test]
async fn nothing_is_fetched_for_a_token_that_cannot_be_read() {
    let source = Arc::new(StaticServiceKeys::default());
    let keys = ServiceKeys::new(source.clone());
    let tenant_id = Uuid::new_v4().to_string();
    for authorization in [
        unsigned(&tenant_id),
        "Bearer fixture.e30.fixture".to_string(),
    ] {
        assert!(verify(&keys, &authorization).await.is_err());
    }
    assert_eq!(source.fetch_count(), 0);
}

#[rocket::async_test]
async fn realm_keys_are_cached_and_fetched_again_once_for_an_unknown_key() {
    let tenant_id = Uuid::new_v4().to_string();
    let first = RealmSigner::new("first");
    let rotated = RealmSigner::new("rotated");
    let source = Arc::new(StaticServiceKeys::default());
    source.publish(&realm(&tenant_id), first.jwks());
    let keys = ServiceKeys::with_timing(
        source.clone(),
        Duration::from_secs(600),
        Duration::ZERO,
    );
    let token = first.sign(&keycloak(&tenant_id), &tenant_id);
    for _ in 0..3 {
        assert!(verify(&keys, &token).await.is_ok());
    }
    assert_eq!(source.fetch_count(), 1);

    // Keycloak rotated its key: one more fetch finds it.
    source.publish(&realm(&tenant_id), rotated.jwks());
    let token = rotated.sign(&keycloak(&tenant_id), &tenant_id);
    assert!(verify(&keys, &token).await.is_ok());
    assert!(verify(&keys, &token).await.is_ok());
    assert_eq!(source.fetch_count(), 2);

    // A key the realm doesn't have is fetched for once, then refused.
    let unknown = RealmSigner::new("unknown");
    let token = unknown.sign(&keycloak(&tenant_id), &tenant_id);
    assert_eq!(
        verify(&keys, &token).await,
        Err(ServiceTokenRefusal::UNKNOWN_KEY)
    );
    assert_eq!(source.fetch_count(), 3);
}

#[rocket::async_test]
async fn unknown_keys_are_not_fetched_again_within_the_cooldown() {
    let tenant_id = Uuid::new_v4().to_string();
    let signer = RealmSigner::shared();
    let (source, keys) = trusting(&tenant_id, signer);
    assert!(
        verify(&keys, &signer.sign(&keycloak(&tenant_id), &tenant_id))
            .await
            .is_ok()
    );
    let unknown = RealmSigner::new("unknown");
    let token = unknown.sign(&keycloak(&tenant_id), &tenant_id);
    for _ in 0..3 {
        assert_eq!(
            verify(&keys, &token).await,
            Err(ServiceTokenRefusal::UNKNOWN_KEY)
        );
    }
    assert_eq!(source.fetch_count(), 1);
}

#[rocket::async_test]
async fn expired_key_sets_are_fetched_again() {
    let tenant_id = Uuid::new_v4().to_string();
    let signer = RealmSigner::shared();
    let source = Arc::new(StaticServiceKeys::default());
    source.publish(&realm(&tenant_id), signer.jwks());
    let keys = ServiceKeys::with_timing(
        source.clone(),
        Duration::ZERO,
        Duration::ZERO,
    );
    let token = signer.sign(&keycloak(&tenant_id), &tenant_id);
    assert!(verify(&keys, &token).await.is_ok());
    assert!(verify(&keys, &token).await.is_ok());
    assert_eq!(source.fetch_count(), 2);
}

#[test]
fn key_sets_keep_the_keys_they_can_read() {
    let signer = RealmSigner::shared();
    let mut body = serde_json::to_value(signer.jwks()).unwrap();
    body["keys"].as_array_mut().unwrap().push(json!({
        "kid": "encryption", "kty": "RSA", "alg": "RSA-OAEP", "use": "enc",
        "n": "AQAB", "e": "AQAB",
    }));
    body["keys"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kid": "future", "kty": "PQC", "alg": "ML-DSA-65"}));
    let keys = parse_key_set(&body);
    assert!(keys.find(KID).is_some());
    assert!(keys.find("encryption").is_some());
    assert!(keys.find("future").is_none());
    assert!(parse_key_set(&json!({"unexpected": true})).keys.is_empty());
}
