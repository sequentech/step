// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use crate::services::test_support::{
    claims, clear_optional_issuer_settings, in_child_process, keys,
    one_shot_server, token, ISSUER,
};
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};

fn realm_of(claims: &Value, bases: &[&str]) -> Result<String> {
    trusted_realm(&serde_json::from_value(claims.clone())?, bases)
}

#[test]
fn signed_token_is_accepted() {
    let verified: JwtClaims =
        verify_with_jwks(&token(&claims()), ISSUER, &keys()).unwrap();
    assert_eq!(verified.sub, "voter");
}

#[test]
fn forged_signature_is_rejected() {
    let original = token(&claims());
    let parts: Vec<_> = original.split('.').collect();
    let forged = format!("{}.{}.forged", parts[0], parts[1]);
    assert!(verify_with_jwks::<Value>(&forged, ISSUER, &keys()).is_err());
}

#[test]
fn changed_claims_are_rejected() {
    let original = token(&claims());
    let parts: Vec<_> = original.split('.').collect();
    let mut changed = claims();
    changed["sub"] = json!("another-voter");
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&changed).unwrap());
    let forged = format!("{}.{payload}.{}", parts[0], parts[2]);
    assert!(verify_with_jwks::<Value>(&forged, ISSUER, &keys()).is_err());
}

#[test]
fn expired_missing_expiry_and_future_tokens_are_rejected() {
    for field in ["expired", "missing", "future"] {
        let mut invalid = claims();
        match field {
            "expired" => invalid["exp"] = json!(1),
            "missing" => {
                invalid.as_object_mut().unwrap().remove("exp");
            }
            _ => invalid["nbf"] = json!(4102444800_u64),
        }
        assert!(verify_with_jwks::<Value>(&token(&invalid), ISSUER, &keys())
            .is_err());
    }
}

#[test]
fn wrong_issuer_and_unknown_keys_are_rejected() {
    assert!(verify_with_jwks::<Value>(
        &token(&claims()),
        "https://untrusted.example/realms/tenant-one-event-election",
        &keys()
    )
    .is_err());
    let mut no_key = keys();
    no_key.keys.clear();
    assert!(
        verify_with_jwks::<Value>(&token(&claims()), ISSUER, &no_key).is_err()
    );
}

#[test]
fn symmetric_tokens_are_rejected() {
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("test-key".into());
    let forged = encode(
        &header,
        &claims(),
        &EncodingKey::from_secret(b"untrusted-secret"),
    )
    .unwrap();
    assert!(verify_with_jwks::<Value>(&forged, ISSUER, &keys()).is_err());
}

#[test]
fn issuer_is_bound_to_tenant_and_event() {
    let bases = [
        "https://keycloak.example",
        "http://keycloak:8090",
        "https://kiosk-id.example/auth",
    ];
    let mut valid = claims();
    assert_eq!(
        realm_of(&valid, &bases).unwrap(),
        "tenant-one-event-election"
    );
    valid["iss"] = json!("https://keycloak.example/realms/tenant-one");
    assert_eq!(realm_of(&valid, &bases).unwrap(), "tenant-one");
    valid["iss"] =
        json!("https://kiosk-id.example/auth/realms/tenant-one-event-election");
    assert_eq!(
        realm_of(&valid, &bases).unwrap(),
        "tenant-one-event-election"
    );
    for issuer in [
        "https://untrusted.example/realms/tenant-one-event-election",
        "https://keycloak.example.untrusted.example/realms/tenant-one-event-election",
        "https://keycloak.example/realms/tenant-other-event-election",
        "https://keycloak.example/realms/tenant-one-event-other",
        "https://keycloak.example/realms/tenant-one-event-election?query=x",
        "https://keycloak.example/realms/tenant-one-event-election/../master",
        "https://keycloak.example/realms/tenant-one-event-election%2fmaster",
        "https://keycloak.example/realms/master",
    ] {
        valid["iss"] = json!(issuer);
        assert!(realm_of(&valid, &bases).is_err(), "accepted {issuer}");
    }
    valid = claims();
    valid["https://hasura.io/jwt/claims"]["x-hasura-tenant-id"] =
        json!("other");
    assert!(realm_of(&valid, &bases).is_err());
    valid = claims();
    valid["https://hasura.io/jwt/claims"]
        .as_object_mut()
        .unwrap()
        .remove("x-hasura-election-event-id");
    assert_eq!(
        realm_of(&valid, &bases).unwrap(),
        "tenant-one-event-election"
    );
}

#[test]
fn invalid_configured_bases_report_distinct_reasons_without_echoing_them() {
    let scheme = configured_base("ftp://private-host.example")
        .unwrap_err()
        .to_string();
    let userinfo = configured_base("https://user:private@keycloak.example")
        .unwrap_err()
        .to_string();
    assert_ne!(scheme, userinfo);
    for message in [scheme, userinfo] {
        assert!(!message.contains("private"), "{message}");
    }
    assert!(configured_base("https://keycloak.example/auth").is_ok());
}

#[tokio::test]
async fn signed_public_and_kiosk_tokens_use_internal_keys_and_cached_requests()
{
    let (internal, served) =
        one_shot_server("200 OK", serde_json::to_string(&keys()).unwrap(), "")
            .await;
    let bases = ["https://keycloak.example", "https://kiosk-id.example/auth"];
    let valid = token(&claims());
    assert_eq!(
        verify_bearer_with_config(&valid, &bases, &internal)
            .await
            .unwrap()
            .sub,
        "voter"
    );
    let request = served.await.unwrap();
    assert!(request.starts_with("GET /realms/tenant-one-event-election/protocol/openid-connect/certs HTTP/1.1"));
    // The one-shot key server has stopped; the next valid tokens must use cache.
    let mut kiosk = claims();
    kiosk["iss"] =
        json!("https://kiosk-id.example/auth/realms/tenant-one-event-election");
    assert!(verify_bearer_with_config(&token(&kiosk), &bases, &internal)
        .await
        .is_ok());
    let mut legacy = claims();
    legacy["https://hasura.io/jwt/claims"]
        .as_object_mut()
        .unwrap()
        .remove("x-hasura-election-event-id");
    assert!(
        verify_bearer_with_config(&token(&legacy), &bases, &internal)
            .await
            .is_ok()
    );
    let parts: Vec<_> = valid.split('.').collect();
    let forged = format!("{}.{}.forged", parts[0], parts[1]);
    assert!(verify_bearer_with_config(&forged, &bases, &internal)
        .await
        .is_err());
}

#[tokio::test]
async fn failed_key_refresh_preserves_only_unexpired_keys() {
    let (internal, served) =
        one_shot_server("503 Service Unavailable", String::new(), "").await;
    let realm = "tenant-one-event-election";
    let cache_key = format!("{internal}/realms/{realm}");
    let entry = Arc::new(Mutex::new(CachedKeys {
        keys: Some(Arc::new(keys())),
        fetched: Some(
            Instant::now() - REFRESH_INTERVAL - Duration::from_secs(1),
        ),
        ..CachedKeys::default()
    }));
    cache()
        .lock()
        .await
        .insert(cache_key, (Instant::now(), Arc::clone(&entry)));
    assert!(realm_keys(&DOWNLOADS, &internal, realm, "rotated-key")
        .await
        .is_err());
    served.await.unwrap();
    let valid = token(&claims());
    assert!(verify_bearer_with_config(
        &valid,
        &["https://keycloak.example"],
        &internal
    )
    .await
    .is_ok());
    // A failed refresh must never extend the lifetime of the previous keys.
    entry.lock().await.fetched =
        Some(Instant::now() - CACHE_TTL - Duration::from_secs(1));
    assert!(verify_bearer_with_config(
        &valid,
        &["https://keycloak.example"],
        &internal
    )
    .await
    .is_err());
}

#[tokio::test]
async fn key_downloads_need_a_free_slot_and_refusals_do_not_delay_retries() {
    let (internal, served) =
        one_shot_server("200 OK", serde_json::to_string(&keys()).unwrap(), "")
            .await;
    let realm = "tenant-one";
    assert!(realm_keys(&Semaphore::new(0), &internal, realm, "test-key")
        .await
        .is_err());
    assert!(!served.is_finished());
    assert!(realm_keys(&Semaphore::new(1), &internal, realm, "test-key")
        .await
        .is_ok());
    served.await.unwrap();
}

fn cached_realm(
    verified: bool,
    age_secs: u64,
) -> (Instant, Arc<Mutex<CachedKeys>>) {
    let cached = CachedKeys {
        keys: verified.then(|| Arc::new(keys())),
        ..CachedKeys::default()
    };
    (
        Instant::now() - Duration::from_secs(age_secs),
        Arc::new(Mutex::new(cached)),
    )
}

#[test]
fn unverified_realms_are_evicted_first_and_realms_in_use_are_kept() {
    let mut entries = RealmCache::new();
    entries.insert("verified".into(), cached_realm(true, 30));
    entries.insert("unknown-old".into(), cached_realm(false, 20));
    entries.insert("unknown-new".into(), cached_realm(false, 10));
    make_room(&mut entries).unwrap();
    assert!(!entries.contains_key("unknown-old"));
    assert!(entries.contains_key("verified"));
    assert!(entries.contains_key("unknown-new"));
    let in_use: Vec<_> = entries
        .values()
        .map(|(_, realm)| Arc::clone(realm))
        .collect();
    assert!(make_room(&mut entries).is_err());
    assert_eq!(entries.len(), 2);
    drop(in_use);
    make_room(&mut entries).unwrap();
    assert!(entries.contains_key("verified"));
}

#[tokio::test]
async fn key_fetch_redirects_and_oversized_bodies_are_rejected() {
    let (internal, served) = one_shot_server(
        "302 Found",
        serde_json::to_string(&keys()).unwrap(),
        "Location: http://127.0.0.1:1/keys\r\n",
    )
    .await;
    assert!(download_keys(&internal, "tenant-one").await.is_err());
    served.await.unwrap();
    let (internal, served) =
        one_shot_server("200 OK", "x".repeat(MAX_JWKS_BYTES + 1), "").await;
    assert!(download_keys(&internal, "tenant-one").await.is_err());
    served.await.unwrap();
}

#[tokio::test]
async fn empty_optional_issuer_settings_are_ignored_and_invalid_ones_reported()
{
    if !in_child_process(
        module_path!(),
        "empty_optional_issuer_settings_are_ignored_and_invalid_ones_reported",
    ) {
        return;
    }
    clear_optional_issuer_settings();
    let (internal, served) =
        one_shot_server("200 OK", serde_json::to_string(&keys()).unwrap(), "")
            .await;
    std::env::set_var(KEYCLOAK_URL, &internal);
    std::env::set_var(KEYCLOAK_PUBLIC_URL, "https://keycloak.example");
    std::env::set_var(KIOSK_KEYCLOAK_URL, "");
    std::env::set_var(HARVEST_JWT_ISSUER_URLS, " , ");
    let valid = token(&claims());
    assert!(verify_bearer(&valid).await.is_ok());
    served.await.unwrap();
    std::env::set_var(KIOSK_KEYCLOAK_URL, "not-a-url");
    let error = verify_bearer(&valid).await.unwrap_err();
    assert!(format!("{error:#}").contains(KIOSK_KEYCLOAK_URL));
}
