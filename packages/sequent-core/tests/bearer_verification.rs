// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Drive the bearer guard through Rocket dispatch with tokens that fail
//! verification in one way each. Every refusal starts from a valid signed
//! token, so only the changed property can explain the 401.

#![cfg(all(feature = "keycloak", feature = "default_features"))]

#[allow(dead_code)]
#[path = "support/http.rs"]
mod http;
#[path = "support/signing.rs"]
mod signing;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use http::{Exchange, HttpServer};
use jsonwebtoken::{Algorithm, EncodingKey, Header as JwtHeader};
use rocket::{
    get,
    http::{Header, Status},
    local::asynchronous::Client,
    routes,
};
use sequent_core::services::connection::BearerIssuers;
use sequent_core::services::jwt::JwtClaims;
use serde_json::{json, Value};

const KEY_PATH: &str = "/realms/tenant-north/protocol/openid-connect/certs";
const ALIAS: &str = "https://alias.example/auth";
const SETTINGS: [&str; 4] = [
    "KEYCLOAK_URL",
    "KEYCLOAK_PUBLIC_URL",
    "KIOSK_KEYCLOAK_URL",
    "HARVEST_JWT_ISSUER_URLS",
];

#[get("/claims")]
fn claims(claims: JwtClaims) -> String {
    claims.hasura_claims.tenant_id
}

fn voter(issuer: &str) -> Value {
    json!({"exp": signing::EXPIRY, "iat": 0, "jti": "test", "iss": issuer, "sub": "voter",
        "typ": "Bearer", "azp": "voting-portal", "acr": "1", "allowed-origins": [], "scope": "openid", "email_verified": false,
        "https://hasura.io/jwt/claims": {"x-hasura-default-role": "user", "x-hasura-tenant-id": "north",
            "x-hasura-election-event-id": "mayor", "x-hasura-user-id": "voter", "x-hasura-allowed-roles": ["user"]}})
}

fn tenant_issuer() -> String {
    signing::issuer_of("north")
}

fn event_issuer() -> String {
    format!("{}-event-mayor", tenant_issuer())
}

fn without(mut claims: Value, name: &str) -> Value {
    claims.as_object_mut().unwrap().remove(name);
    claims
}

async fn client_trusting(issuers: BearerIssuers) -> Client {
    Client::tracked(rocket::build().manage(issuers).mount("/", routes![claims]))
        .await
        .unwrap()
}

async fn answer(client: &Client, token: &str) -> (Status, String) {
    let response = client
        .get("/claims")
        .header(Header::new("Authorization", format!("Bearer {token}")))
        .dispatch()
        .await;
    let status = response.status();
    (status, response.into_string().await.unwrap_or_default())
}

#[rocket::async_test]
async fn only_claims_that_match_their_issuer_realm_are_accepted() {
    let client = client_trusting(signing::issuers()).await;
    for issuer in [tenant_issuer(), event_issuer()] {
        let (status, tenant) =
            answer(&client, &signing::token(&voter(&issuer))).await;
        assert_eq!(
            (status, tenant.as_str()),
            (Status::Ok, "north"),
            "{issuer}"
        );
    }
    let mut no_tenant = voter(&tenant_issuer());
    no_tenant["https://hasura.io/jwt/claims"]
        .as_object_mut()
        .unwrap()
        .remove("x-hasura-tenant-id");
    let mut expired = voter(&tenant_issuer());
    expired["exp"] = json!(1);
    let mut not_yet_valid = voter(&tenant_issuer());
    not_yet_valid["nbf"] = json!(signing::EXPIRY);
    for (reason, claims) in [
        ("no issuer", without(voter(&tenant_issuer()), "iss")),
        ("no tenant", no_tenant),
        ("no expiry", without(voter(&tenant_issuer()), "exp")),
        (
            "no authorized party",
            without(voter(&tenant_issuer()), "azp"),
        ),
        ("expired", expired),
        ("not yet valid", not_yet_valid),
        (
            "other host",
            voter("https://untrusted.example/realms/tenant-north"),
        ),
        ("other tenant realm", voter(&signing::issuer_of("south"))),
        (
            "other event realm",
            voter(&format!("{}-event-council", tenant_issuer())),
        ),
        (
            "master realm",
            voter(&format!("{}/realms/master", signing::ISSUER)),
        ),
        (
            "realm path",
            voter(&format!("{}/../master", tenant_issuer())),
        ),
        ("no realm", voter(signing::ISSUER)),
    ] {
        let (status, _) = answer(&client, &signing::token(&claims)).await;
        assert_eq!(status, Status::Unauthorized, "{reason}");
    }
}

#[rocket::async_test]
async fn tokens_that_cannot_be_verified_are_refused() {
    let client = client_trusting(signing::issuers()).await;
    let claims = voter(&tenant_issuer());
    let valid = signing::token(&claims);
    let parts: Vec<_> = valid.split('.').collect();
    let mut padded = claims.clone();
    padded["padding"] = json!("x".repeat(64 * 1024));
    let mut without_key_id = JwtHeader::new(Algorithm::RS256);
    without_key_id.kid = None;
    let mut unknown_key_id = JwtHeader::new(Algorithm::RS256);
    unknown_key_id.kid = Some("rotated-key".into());
    let mut symmetric = JwtHeader::new(Algorithm::HS256);
    symmetric.kid = Some("test-key".into());
    let symmetric = jsonwebtoken::encode(
        &symmetric,
        &claims,
        &EncodingKey::from_secret(b"untrusted-secret"),
    )
    .unwrap();
    for (reason, token) in [
        ("oversized", signing::token(&padded)),
        ("no key id", signing::token_with(&without_key_id, &claims)),
        (
            "unknown key id",
            signing::token_with(&unknown_key_id, &claims),
        ),
        ("symmetric signature", symmetric),
        (
            "changed signature",
            format!("{}.{}.forged", parts[0], parts[1]),
        ),
        (
            "payload not base64",
            format!("{}.%%%.{}", parts[0], parts[2]),
        ),
        (
            "payload not JSON",
            format!(
                "{}.{}.{}",
                parts[0],
                URL_SAFE_NO_PAD.encode("{"),
                parts[2]
            ),
        ),
        ("not a token", "not-a-token".into()),
    ] {
        let (status, _) = answer(&client, &token).await;
        assert_eq!(status, Status::Unauthorized, "{reason}");
    }
    let (status, tenant) = answer(&client, &valid).await;
    assert_eq!((status, tenant.as_str()), (Status::Ok, "north"));
}

#[rocket::async_test]
async fn unusable_key_servers_refuse_the_token_and_a_usable_one_accepts_it() {
    let key = signing::key_set()["keys"][0].clone();
    let valid = signing::token(&voter(&tenant_issuer()));
    for (reason, exchange, expected) in [
        (
            "key set that is not JSON",
            Exchange::json("GET", KEY_PATH, 200, json!({})).body("{"),
            Status::Unauthorized,
        ),
        (
            "key set cut short",
            Exchange::json("GET", KEY_PATH, 200, signing::key_set())
                .truncate_at(10),
            Status::Unauthorized,
        ),
        (
            "more keys than a realm publishes",
            Exchange::json(
                "GET",
                KEY_PATH,
                200,
                json!({"keys": vec![key.clone(); 129]}),
            ),
            Status::Unauthorized,
        ),
        (
            "realm that is not published",
            Exchange::json("GET", KEY_PATH, 404, json!({})),
            Status::Unauthorized,
        ),
        (
            "usable key set",
            Exchange::json("GET", KEY_PATH, 200, signing::key_set()),
            Status::Ok,
        ),
    ] {
        let server = HttpServer::start(vec![exchange]);
        let client = client_trusting(BearerIssuers {
            internal_base: server.url.clone(),
            trusted_bases: vec![signing::ISSUER.into()],
        })
        .await;
        let (status, _) = answer(&client, &valid).await;
        assert_eq!(status, expected, "{reason}");
        assert_eq!(server.finish().len(), 1, "{reason}");
    }
    for (reason, internal_base) in [
        ("key server that is not a URL", "not-a-url"),
        ("key server nobody listens on", "http://127.0.0.1:1"),
    ] {
        let client = client_trusting(BearerIssuers {
            internal_base: internal_base.into(),
            trusted_bases: vec![signing::ISSUER.into()],
        })
        .await;
        let (status, _) = answer(&client, &valid).await;
        assert_eq!(status, Status::Unauthorized, "{reason}");
    }
}

#[rocket::async_test]
async fn misconfigured_trusted_issuers_are_skipped_not_trusted() {
    let mut issuers = signing::issuers();
    issuers.trusted_bases.insert(0, "not-a-url".into());
    let client = client_trusting(issuers).await;
    let (status, tenant) =
        answer(&client, &signing::token(&voter(&tenant_issuer()))).await;
    assert_eq!((status, tenant.as_str()), (Status::Ok, "north"));
    let (status, _) = answer(
        &client,
        &signing::token(&voter("not-a-url/realms/tenant-north")),
    )
    .await;
    assert_eq!(status, Status::Unauthorized);
}

#[rocket::async_test]
async fn realms_keep_being_served_past_the_bound_of_cached_realms() {
    let client = client_trusting(signing::issuers()).await;
    for number in 0..300 {
        let tenant = format!("crowded-{number}");
        let mut claims = voter(&signing::issuer_of(&tenant));
        claims["https://hasura.io/jwt/claims"]["x-hasura-tenant-id"] =
            json!(tenant);
        let (status, served) = answer(&client, &signing::token(&claims)).await;
        assert_eq!((status, served), (Status::Ok, tenant));
    }
}

struct Settings(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Settings {
    fn clear() -> Self {
        Self(
            SETTINGS
                .iter()
                .map(|name| {
                    let previous = std::env::var_os(name);
                    std::env::remove_var(name);
                    (*name, previous)
                })
                .collect(),
        )
    }

    fn set(&self, name: &str, value: &str) {
        std::env::set_var(name, value);
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        for (name, previous) in &self.0 {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

// The only test of this binary that reads or changes the Keycloak settings;
// the others register their issuers with the application instead.
#[rocket::async_test]
async fn without_managed_issuers_the_guard_trusts_the_configured_ones() {
    let client = Client::tracked(rocket::build().mount("/", routes![claims]))
        .await
        .unwrap();
    let public = signing::token(&voter(&tenant_issuer()));
    let alias = signing::token(&voter(&format!("{ALIAS}/realms/tenant-north")));
    let settings = Settings::clear();
    assert_eq!(answer(&client, &public).await.0, Status::Unauthorized);
    settings.set("KEYCLOAK_URL", "not-a-url");
    assert_eq!(answer(&client, &public).await.0, Status::Unauthorized);
    settings.set("KEYCLOAK_URL", &signing::issuers().internal_base);
    assert_eq!(answer(&client, &public).await.0, Status::Unauthorized);
    settings.set("KEYCLOAK_PUBLIC_URL", signing::ISSUER);
    assert_eq!(answer(&client, &public).await.0, Status::Ok);
    settings.set("KIOSK_KEYCLOAK_URL", "");
    settings.set("HARVEST_JWT_ISSUER_URLS", " , ");
    assert_eq!(answer(&client, &public).await.0, Status::Ok);
    assert_eq!(answer(&client, &alias).await.0, Status::Unauthorized);
    settings.set("HARVEST_JWT_ISSUER_URLS", &format!(" , {ALIAS} ,"));
    assert_eq!(answer(&client, &alias).await.0, Status::Ok);
    settings.set("HARVEST_JWT_ISSUER_URLS", "ftp://alias.example");
    assert_eq!(answer(&client, &public).await.0, Status::Unauthorized);
}
