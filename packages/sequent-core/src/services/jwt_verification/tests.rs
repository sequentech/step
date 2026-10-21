// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};

const ISSUER: &str =
    "https://keycloak.example/realms/tenant-one-event-election";

fn claims() -> Value {
    json!({"iss": ISSUER, "exp": 4102444800_u64, "sub": "voter", "iat": 1,
        "jti": "test", "typ": "Bearer", "azp": "voting-portal", "acr": "1",
        "allowed-origins": [], "scope": "openid", "email_verified": true,
        "https://hasura.io/jwt/claims": {
            "x-hasura-tenant-id": "one", "x-hasura-election-event-id": "election",
            "x-hasura-default-role": "user", "x-hasura-user-id": "voter",
            "x-hasura-allowed-roles": ["user"]}})
}

fn keys() -> JwkSet {
    serde_json::from_str(include_str!("fixtures/test-jwks.json")).unwrap()
}

fn token(claims: &Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("test-key".into());
    encode(
        &header,
        claims,
        &EncodingKey::from_rsa_pem(include_bytes!("fixtures/test-key.pem"))
            .unwrap(),
    )
    .unwrap()
}

async fn key_server(
    status: &str,
    body: String,
    extra_headers: &str,
) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n{body}", body.len());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 4096];
        let length = socket.read(&mut request).await.unwrap();
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(request[..length].to_vec()).unwrap()
    });
    (base, task)
}

fn base(url: &str) -> Url {
    configured_base(url).unwrap()
}

fn issuers_with(internal: &str, trusted: &[&str]) -> BearerIssuers {
    BearerIssuers::new(internal, trusted).unwrap()
}

/// Reads the issuer settings from `values` instead of the environment.
fn from_settings(values: &[(&str, &str)]) -> Result<BearerIssuers> {
    let values: HashMap<_, _> = values.iter().copied().collect();
    BearerIssuers::from_vars(|name| values.get(name).map(|v| v.to_string()))
}

fn realm_of(claims: &Value, bases: &[&str]) -> Result<String> {
    let bases: Vec<_> = bases.iter().map(|url| base(url)).collect();
    trusted_realm(&serde_json::from_value(claims.clone())?, &bases)
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
fn malformed_tokens_and_unusable_keys_are_rejected() {
    assert!(verify_with_jwks::<Value>("not-a-token", ISSUER, &keys()).is_err());
    let without_key_id = encode(
        &Header::new(Algorithm::RS256),
        &claims(),
        &EncodingKey::from_rsa_pem(include_bytes!("fixtures/test-key.pem"))
            .unwrap(),
    )
    .unwrap();
    let error = verify_with_jwks::<Value>(&without_key_id, ISSUER, &keys())
        .unwrap_err();
    assert_eq!(error.to_string(), "Missing signing key ID");
    let mut unusable = serde_json::to_value(keys()).unwrap();
    unusable["keys"][0]["n"] = json!("!");
    let unusable: JwkSet = serde_json::from_value(unusable).unwrap();
    let error = verify_with_jwks::<Value>(&token(&claims()), ISSUER, &unusable)
        .unwrap_err();
    assert_ne!(error.to_string(), "Unknown signing key");
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
        key_server("200 OK", serde_json::to_string(&keys()).unwrap(), "").await;
    let issuers = issuers_with(
        &internal,
        &["https://keycloak.example", "https://kiosk-id.example/auth"],
    );
    let valid = token(&claims());
    assert_eq!(verify_bearer(&valid, &issuers).await.unwrap().sub, "voter");
    let request = served.await.unwrap();
    assert!(request.starts_with("GET /realms/tenant-one-event-election/protocol/openid-connect/certs HTTP/1.1"));
    // The one-shot key server has stopped; the next valid tokens must use cache.
    let mut kiosk = claims();
    kiosk["iss"] =
        json!("https://kiosk-id.example/auth/realms/tenant-one-event-election");
    assert!(verify_bearer(&token(&kiosk), &issuers).await.is_ok());
    let mut legacy = claims();
    legacy["https://hasura.io/jwt/claims"]
        .as_object_mut()
        .unwrap()
        .remove("x-hasura-election-event-id");
    assert!(verify_bearer(&token(&legacy), &issuers).await.is_ok());
    let parts: Vec<_> = valid.split('.').collect();
    let forged = format!("{}.{}.forged", parts[0], parts[1]);
    assert!(verify_bearer(&forged, &issuers).await.is_err());
}

#[tokio::test]
async fn failed_key_refresh_preserves_only_unexpired_keys() {
    let (internal, served) =
        key_server("503 Service Unavailable", String::new(), "").await;
    let issuers = issuers_with(&internal, &["https://keycloak.example"]);
    let realm = "tenant-one-event-election";
    let cache_key = realm_url(&issuers.internal_base, realm);
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
    assert!(realm_keys(
        &DOWNLOADS,
        &issuers.internal_base,
        realm,
        "rotated-key"
    )
    .await
    .is_err());
    served.await.unwrap();
    let valid = token(&claims());
    assert!(verify_bearer(&valid, &issuers).await.is_ok());
    // A failed refresh must never extend the lifetime of the previous keys.
    entry.lock().await.fetched =
        Some(Instant::now() - CACHE_TTL - Duration::from_secs(1));
    assert!(verify_bearer(&valid, &issuers).await.is_err());
}

#[tokio::test]
async fn key_downloads_need_a_free_slot_and_refusals_do_not_delay_retries() {
    let (internal, served) =
        key_server("200 OK", serde_json::to_string(&keys()).unwrap(), "").await;
    let (internal, realm) = (base(&internal), "tenant-one");
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
    let (internal, served) = key_server(
        "302 Found",
        serde_json::to_string(&keys()).unwrap(),
        "Location: http://127.0.0.1:1/keys\r\n",
    )
    .await;
    assert!(download_keys(&base(&internal), "tenant-one").await.is_err());
    served.await.unwrap();
    let (internal, served) =
        key_server("200 OK", "x".repeat(MAX_JWKS_BYTES + 1), "").await;
    assert!(download_keys(&base(&internal), "tenant-one").await.is_err());
    served.await.unwrap();
}

#[test]
fn settings_trust_every_configured_base_and_ignore_empty_entries() {
    let issuers = from_settings(&[
        (KEYCLOAK_URL, "http://keycloak:8090"),
        (KEYCLOAK_PUBLIC_URL, "https://keycloak.example"),
        (KIOSK_KEYCLOAK_URL, ""),
        (
            HARVEST_JWT_ISSUER_URLS,
            " , https://alias.example/auth ,https://other.example,",
        ),
    ])
    .unwrap();
    assert_eq!(issuers.internal_base, base("http://keycloak:8090"));
    assert_eq!(
        issuers.trusted_bases,
        [
            "http://keycloak:8090",
            "https://keycloak.example",
            "https://alias.example/auth",
            "https://other.example",
        ]
        .map(base)
    );
    let only_required =
        from_settings(&[(KEYCLOAK_URL, "http://keycloak:8090")]).unwrap();
    assert_eq!(only_required.trusted_bases, [base("http://keycloak:8090")]);
}

#[test]
fn missing_and_invalid_settings_are_named_without_echoing_values() {
    let error =
        from_settings(&[(KEYCLOAK_PUBLIC_URL, "https://keycloak.example")])
            .unwrap_err();
    assert!(format!("{error:#}").contains(KEYCLOAK_URL), "{error:#}");
    for (setting, value) in [
        (KEYCLOAK_URL, "https://user:private@keycloak.example"),
        (
            KEYCLOAK_PUBLIC_URL,
            "https://keycloak.example,ftp://private.example",
        ),
        (KIOSK_KEYCLOAK_URL, "private.example"),
        (
            HARVEST_JWT_ISSUER_URLS,
            "https://alias.example, https://private.example/?realm=x",
        ),
    ] {
        let error = from_settings(&[
            (KEYCLOAK_URL, "http://keycloak:8090"),
            (setting, value),
        ])
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains(setting), "{message}");
        assert!(!message.contains("private"), "{message}");
    }
}

#[test]
fn issuers_built_directly_reject_invalid_bases() {
    assert!(
        BearerIssuers::new("not-a-url", &["https://keycloak.example"]).is_err()
    );
    assert!(BearerIssuers::new(
        "http://keycloak:8090",
        &["https://keycloak.example", "not-a-url"]
    )
    .is_err());
}
