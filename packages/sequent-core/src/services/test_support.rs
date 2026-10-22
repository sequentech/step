// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::jwt_verification::{HARVEST_JWT_ISSUER_URLS, KIOSK_KEYCLOAK_URL};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};

pub(super) const ISSUER: &str =
    "https://keycloak.example/realms/tenant-one-event-election";

pub(super) fn claims() -> Value {
    json!({"iss": ISSUER, "exp": 4102444800_u64, "sub": "voter", "iat": 1,
        "jti": "test", "typ": "Bearer", "azp": "voting-portal", "acr": "1",
        "allowed-origins": [], "scope": "openid", "email_verified": true,
        "https://hasura.io/jwt/claims": {
            "x-hasura-tenant-id": "one", "x-hasura-election-event-id": "election",
            "x-hasura-default-role": "user", "x-hasura-user-id": "voter",
            "x-hasura-allowed-roles": ["user"]}})
}

pub(super) fn keys() -> JwkSet {
    serde_json::from_str(include_str!(
        "jwt_verification/fixtures/test-jwks.json"
    ))
    .unwrap()
}

pub(super) fn token(claims: &Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("test-key".into());
    encode(
        &header,
        claims,
        &EncodingKey::from_rsa_pem(include_bytes!(
            "jwt_verification/fixtures/test-key.pem"
        ))
        .unwrap(),
    )
    .unwrap()
}

pub(super) async fn one_shot_server(
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

/// Runs `test` of `module` again in a child process whose environment it may
/// change, and tells the caller whether it already is that child.
pub(super) fn in_child_process(module: &str, test: &str) -> bool {
    const CHILD: &str = "SEQUENT_JWT_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    // Test names in the harness omit the crate name.
    let module = module.split_once("::").map_or(module, |(_, path)| path);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{module}::{test}")])
        .env(CHILD, "1")
        .env(KIOSK_KEYCLOAK_URL, "not-a-url")
        .env(HARVEST_JWT_ISSUER_URLS, "not-a-url")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains(" 1 passed"),
        "{test} failed: {stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

pub(super) fn clear_optional_issuer_settings() {
    for variable in [KIOSK_KEYCLOAK_URL, HARVEST_JWT_ISSUER_URLS] {
        std::env::remove_var(variable);
    }
}
