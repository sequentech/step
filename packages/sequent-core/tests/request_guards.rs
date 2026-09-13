// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise guards through Rocket dispatch. JwtClaims only parses a token;
//! signature verification and trusted-proxy enforcement belong upstream.

#![cfg(feature = "keycloak")]

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
use serde_json::json;

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

#[rocket::async_test]
async fn claims_guard_requires_bearer_syntax_and_parseable_claims() {
    let client = Client::tracked(rocket::build().mount("/", routes![claims]))
        .await
        .unwrap();
    assert_eq!(
        client.get("/claims").dispatch().await.status(),
        Status::Unauthorized
    );
    for authorization in [
        "Basic synthetic",
        "Bearer",
        "Bearer malformed",
        "Bearer h.!.s",
    ] {
        assert_eq!(
            client
                .get("/claims")
                .header(Header::new("Authorization", authorization))
                .dispatch()
                .await
                .status(),
            Status::Unauthorized
        );
    }
    // An unsigned fixture deliberately proves parsing only, not authentication.
    let claims = json!({"exp": 2000000000, "iat": 0, "jti": "test", "iss": "test", "sub": "voter",
        "typ": "Bearer", "azp": "voting-portal", "acr": "1", "allowed-origins": [], "scope": "openid", "email_verified": false,
        "https://hasura.io/jwt/claims": {"x-hasura-default-role": "user", "x-hasura-tenant-id": "north",
            "x-hasura-user-id": "voter", "x-hasura-allowed-roles": ["user"]}});
    let token = format!(
        "Bearer fixture.{}.unsigned",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
    );
    assert_eq!(
        client
            .get("/claims")
            .header(Header::new("Authorization", token))
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
