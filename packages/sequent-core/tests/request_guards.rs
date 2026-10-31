// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise guards through Rocket dispatch. JwtClaims verifies the signature,
//! expiry and issuer of a token; trusted-proxy enforcement belongs upstream.

#![cfg(all(feature = "keycloak", feature = "default_features"))]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rocket::{
    get,
    http::{Header, Status},
    local::asynchronous::Client,
    routes,
};
use sequent_core::services::connection::{
    AuthHeaders, DatafixClaims, LastDatafixAccessToken, UserLocation,
};
use sequent_core::services::jwt::JwtClaims;
use serde_json::{json, Value};

#[path = "support/signing.rs"]
mod signing;

#[get("/headers")]
fn headers(auth: AuthHeaders) -> String {
    format!("{}={}", auth.key, auth.value)
}
#[get("/claims")]
fn claims(claims: JwtClaims) -> String {
    claims.hasura_claims.tenant_id
}
#[get("/location")]
fn location(location: UserLocation) -> String {
    format!(
        "{}|{}",
        location.ip.map(|ip| ip.to_string()).unwrap_or_default(),
        location.country_code.unwrap_or_default()
    )
}
#[get("/location-evidence")]
fn location_evidence(
    location: UserLocation,
) -> rocket::serde::json::Json<Value> {
    let redacted = format!("{location:?}");
    rocket::serde::json::Json(json!({
        "agent": location.user_agent,
        "ip": location.ip.map(|ip| ip.to_string()),
        "country": location.country_code,
        "debug": redacted,
    }))
}
#[get("/datafix")]
fn datafix(claims: DatafixClaims) -> String {
    claims.tenant_id
}

#[rocket::async_test]
async fn missing_credentials_are_rejected_and_admin_headers_have_explicit_precedence(
) {
    let client = Client::tracked(rocket::build().mount("/", routes![headers]))
        .await
        .unwrap();
    assert_eq!(
        client.get("/headers").dispatch().await.status(),
        Status::Unauthorized
    );
    assert_eq!(
        client
            .get("/headers")
            .header(Header::new("Authorization", "Bearer synthetic-token"))
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap(),
        "authorization=Bearer synthetic-token"
    );
    let response = client
        .get("/headers")
        .header(Header::new("Authorization", "Bearer synthetic-token"))
        .header(Header::new(
            "X-Hasura-Admin-Secret",
            "synthetic-admin-secret",
        ))
        .dispatch()
        .await;
    assert_eq!(
        response.into_string().await.unwrap(),
        "X-Hasura-Admin-Secret=synthetic-admin-secret"
    );
}

fn voter_claims(issuer: &str) -> Value {
    json!({"exp": signing::EXPIRY, "iat": 0, "jti": "test", "iss": issuer, "sub": "voter",
        "typ": "Bearer", "azp": "voting-portal", "acr": "1", "allowed-origins": [], "scope": "openid", "email_verified": false,
        "https://hasura.io/jwt/claims": {"x-hasura-default-role": "user", "x-hasura-tenant-id": "north",
            "x-hasura-user-id": "voter", "x-hasura-allowed-roles": ["user"]}})
}

#[rocket::async_test]
async fn claims_guard_requires_bearer_syntax_and_verified_claims() {
    let client = Client::tracked(
        rocket::build()
            .manage(signing::issuers())
            .mount("/", routes![claims]),
    )
    .await
    .unwrap();
    assert_eq!(
        client.get("/claims").dispatch().await.status(),
        Status::Unauthorized
    );
    let claims = voter_claims(&signing::issuer_of("north"));
    let unsigned = format!(
        "fixture.{}.unsigned",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
    );
    let untrusted = signing::token(&voter_claims(
        "https://untrusted.example/realms/tenant-north",
    ));
    let mut stale = claims.clone();
    stale["exp"] = json!(1);
    for authorization in [
        "Basic synthetic".to_string(),
        "Bearer".to_string(),
        "Bearer malformed".to_string(),
        "Bearer h.!.s".to_string(),
        format!("Bearer {unsigned}"),
        format!("Bearer {untrusted}"),
        format!("Bearer {}", signing::token(&stale)),
    ] {
        assert_eq!(
            client
                .get("/claims")
                .header(Header::new("Authorization", authorization.clone()))
                .dispatch()
                .await
                .status(),
            Status::Unauthorized,
            "{authorization}"
        );
    }
    assert_eq!(
        client
            .get("/claims")
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", signing::token(&claims))
            ))
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap(),
        "north"
    );
}

#[rocket::async_test]
async fn location_headers_are_optional_and_cloudflare_takes_precedence() {
    let client = Client::tracked(rocket::build().mount("/", routes![location]))
        .await
        .unwrap();
    assert_eq!(
        client
            .get("/location")
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap(),
        "|"
    );
    assert_eq!(
        client
            .get("/location")
            .header(Header::new("X-Forwarded-For", "2001:db8::1"))
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap(),
        "2001:db8::1|"
    );
    assert_eq!(
        client
            .get("/location")
            .header(Header::new("CF-Connecting-IP", "192.0.2.1"))
            .header(Header::new("X-Forwarded-For", "192.0.2.2"))
            .header(Header::new("CF-IPCountry", "CA"))
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap(),
        "192.0.2.1|CA"
    );
    assert_eq!(
        client
            .get("/location")
            .header(Header::new("CF-Connecting-IP", "not-an-ip"))
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap(),
        "|"
    );
}

#[rocket::async_test]
async fn location_guard_bounds_user_agents_and_redacts_all_network_fields_from_debug(
) {
    let client =
        Client::tracked(rocket::build().mount("/", routes![location_evidence]))
            .await
            .unwrap();
    let absent: Value = client
        .get("/location-evidence")
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();
    assert_eq!(
        absent,
        json!({"agent": null, "ip": null, "country": null, "debug": "UserLocation(<redacted>)"})
    );
    for agent in [
        "Synthetic browser".to_owned(),
        "a".repeat(1025),
        "é".repeat(1025),
    ] {
        let result: Value = client
            .get("/location-evidence")
            .header(Header::new("User-Agent", agent.clone()))
            .header(Header::new("CF-Connecting-IP", "192.0.2.1"))
            .header(Header::new("CF-IPCountry", "CA"))
            .dispatch()
            .await
            .into_json()
            .await
            .unwrap();
        let expected = if agent == "Synthetic browser" {
            agent
        } else if agent.starts_with('a') {
            "a".repeat(1024)
        } else {
            "é".repeat(1024)
        };
        assert_eq!(result["agent"], expected);
        assert_eq!(result["ip"], "192.0.2.1");
        assert_eq!(result["country"], "CA");
        // Debug output is a real logging boundary; none of the forensic fields
        // may escape even when they were supplied to the request guard.
        assert_eq!(result["debug"], "UserLocation(<redacted>)");
    }
}

#[rocket::async_test]
async fn incomplete_datafix_headers_fail_before_requesting_a_token() {
    let client = Client::tracked(
        rocket::build()
            .manage(LastDatafixAccessToken::init())
            .mount("/", routes![datafix]),
    )
    .await
    .unwrap();
    for headers in [
        vec![],
        vec![("tenant-id", "north")],
        vec![("tenant-id", "north"), ("event-id", "mayor")],
        vec![
            ("tenant-id", "north"),
            ("event-id", "mayor"),
            ("authorization", "missing-separator"),
        ],
    ] {
        let mut request = client.get("/datafix");
        for (name, value) in headers {
            request = request.header(Header::new(name, value));
        }
        assert_eq!(request.dispatch().await.status(), Status::BadRequest);
    }
}
