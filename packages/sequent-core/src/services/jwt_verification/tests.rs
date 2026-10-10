// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::json;

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

#[test]
fn signed_token_is_accepted() {
    assert_eq!(
        verify_with_jwks(&token(&claims()), ISSUER, &keys()).unwrap()["sub"],
        json!("voter")
    );
}

#[test]
fn forged_signature_is_rejected() {
    let original = token(&claims());
    let parts: Vec<_> = original.split('.').collect();
    let forged = format!("{}.{}.forged", parts[0], parts[1]);
    assert!(verify_with_jwks(&forged, ISSUER, &keys()).is_err());
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
    assert!(verify_with_jwks(&forged, ISSUER, &keys()).is_err());
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
        assert!(verify_with_jwks(&token(&invalid), ISSUER, &keys()).is_err());
    }
}

#[test]
fn wrong_issuer_and_unknown_keys_are_rejected() {
    assert!(verify_with_jwks(
        &token(&claims()),
        "https://untrusted.example/realms/tenant-one-event-election",
        &keys()
    )
    .is_err());
    let mut no_key = keys();
    no_key.keys.clear();
    assert!(verify_with_jwks(&token(&claims()), ISSUER, &no_key).is_err());
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
    assert!(verify_with_jwks(&forged, ISSUER, &keys()).is_err());
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
        trusted_realm(&valid, &bases).unwrap(),
        "tenant-one-event-election"
    );
    valid["iss"] = json!("https://keycloak.example/realms/tenant-one");
    assert_eq!(trusted_realm(&valid, &bases).unwrap(), "tenant-one");
    valid["iss"] =
        json!("https://kiosk-id.example/auth/realms/tenant-one-event-election");
    assert_eq!(
        trusted_realm(&valid, &bases).unwrap(),
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
        assert!(trusted_realm(&valid, &bases).is_err(), "accepted {issuer}");
    }
    valid = claims();
    valid["https://hasura.io/jwt/claims"]["x-hasura-tenant-id"] =
        json!("other");
    assert!(trusted_realm(&valid, &bases).is_err());
    valid = claims();
    valid["https://hasura.io/jwt/claims"]
        .as_object_mut()
        .unwrap()
        .remove("x-hasura-election-event-id");
    assert_eq!(
        trusted_realm(&valid, &bases).unwrap(),
        "tenant-one-event-election"
    );
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

#[tokio::test]
async fn signed_public_and_kiosk_tokens_use_internal_keys_and_cached_requests()
{
    let (internal, served) =
        key_server("200 OK", serde_json::to_string(&keys()).unwrap(), "").await;
    let bases = ["https://keycloak.example", "https://kiosk-id.example/auth"];
    let valid = token(&claims());
    assert_eq!(
        verify_bearer_with_config(&valid, &bases, &internal)
            .await
            .unwrap()["sub"],
        json!("voter")
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
        key_server("503 Service Unavailable", String::new(), "").await;
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
    assert!(realm_keys(&internal, realm, "rotated-key").await.is_err());
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
async fn key_fetch_redirects_and_oversized_bodies_are_rejected() {
    let (internal, served) = key_server(
        "302 Found",
        String::new(),
        "Location: http://127.0.0.1:1/keys\r\n",
    )
    .await;
    assert!(download_keys(&internal, "tenant-one").await.is_err());
    served.await.unwrap();
    let (internal, served) =
        key_server("200 OK", "x".repeat(MAX_JWKS_BYTES + 1), "").await;
    assert!(download_keys(&internal, "tenant-one").await.is_err());
    served.await.unwrap();
}

#[cfg(feature = "keycloak")]
#[rocket::get("/authenticated")]
fn authenticated(_claims: crate::services::jwt::JwtClaims) -> &'static str {
    "authenticated"
}

#[cfg(feature = "keycloak")]
#[tokio::test]
async fn request_guard_rejects_forged_tokens_and_accepts_signed_tokens() {
    use rocket::http::{Header, Status};
    // Run the real guard in its own test process so its environment cannot race
    // with parallel tests that read Keycloak configuration.
    const ISOLATED: &str = "SEQUENT_JWT_GUARD_TEST_ISOLATED";
    if std::env::var_os(ISOLATED).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "services::jwt_verification::tests::request_guard_rejects_forged_tokens_and_accepts_signed_tokens"])
            .env(ISOLATED, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Isolated guard test failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (internal, served) =
        key_server("200 OK", serde_json::to_string(&keys()).unwrap(), "").await;
    std::env::set_var("KEYCLOAK_URL", &internal);
    std::env::set_var("KEYCLOAK_PUBLIC_URL", "https://keycloak.example");
    let client = rocket::local::asynchronous::Client::tracked(
        rocket::build().mount("/", rocket::routes![authenticated]),
    )
    .await
    .unwrap();
    assert_eq!(
        client.get("/authenticated").dispatch().await.status(),
        Status::Unauthorized
    );
    let valid = token(&claims());
    let parts: Vec<_> = valid.split('.').collect();
    let forged = format!("{}.{}.forged", parts[0], parts[1]);
    assert_eq!(
        client
            .get("/authenticated")
            .header(Header::new("Authorization", format!("Bearer {forged}")))
            .dispatch()
            .await
            .status(),
        Status::Unauthorized
    );
    served.await.unwrap();
    assert_eq!(
        client
            .get("/authenticated")
            .header(Header::new("Authorization", format!("Bearer {valid}")))
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    std::env::set_var("KIOSK_KEYCLOAK_URL", "not-a-url");
    let error = verify_bearer(&valid).await.unwrap_err();
    assert!(format!("{error:#}").contains("KIOSK_KEYCLOAK_URL"));
}
