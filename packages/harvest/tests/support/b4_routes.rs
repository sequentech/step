// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The `/b4` routes with a local HTTP peer as b4: who may write to a board,
//! what reaches b4, and what comes back. Membership is read from the
//! migrated test database.

use super::{
    ReadOrigins, ALLOW_ORIGIN, ANY_ORIGIN, BODY_LIMIT_BYTES, UNREACHABLE,
};
use crate::route_services::http::{Exchange, HttpServer, Request};
use crate::route_services::rows::{self, Event};
use crate::route_services::Services;
use crate::test_claims::Claims;
use deadpool_postgres::Pool;
use protocol_board::BoardName;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use uuid::Uuid;

/// The one trustee of the board's keys ceremony.
const MEMBER: &str = "trustee-1";
/// A trustee of the same tenant outside the ceremony.
const OUTSIDER: &str = "trustee-2";
/// Nothing listens on port 1, so every connection is refused.
const UNREACHABLE_B4: &str = "http://127.0.0.1:1";
const MESSAGE_ID: &str = "4f0e9a9e-2b1c-4a57-9b7e-0c6d2f1a3b5c";
const INITIATE_BODY: &str = r#"{"size":1234}"#;
const CONFIRM_BODY: &str = r#"{"data":null,"version":"0.6"}"#;

/// A trustee's token in `tenant_id`, with the `trustee` claim, which the
/// shared claims builder doesn't set.
fn token(
    tenant_id: &str,
    trustee: Option<&str>,
    permissions: &[Permissions],
) -> Header<'static> {
    let mut claims = Claims::new(tenant_id, "trustee-user")
        .roles(permissions)
        .build();
    claims.trustee = trustee.map(str::to_owned);
    Header::new(
        "Authorization",
        format!(
            "Bearer {}",
            crate::test_claims::signing::token(
                &serde_json::to_value(&claims).unwrap()
            )
        ),
    )
}

fn trustee_token(tenant_id: &str, trustee: Option<&str>) -> Header<'static> {
    token(tenant_id, trustee, &[Permissions::TRUSTEE_CEREMONY])
}

fn initiate_path(board: &str) -> String {
    format!("/b4/boards/{board}/messages/initiate")
}

fn confirm_path(board: &str, id: &str) -> String {
    format!("/b4/boards/{board}/messages/{id}/confirm")
}

/// A trustee named `name` of `tenant_id`; returns its id.
async fn trustee(pool: &Pool, tenant_id: &str, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.trustee (id, tenant_id, name)
         VALUES ($1, $2, $3)",
        &[&id, &Uuid::parse_str(tenant_id).unwrap(), &name],
    )
    .await;
    id
}

/// An event with a DKG board whose keys ceremony has [`MEMBER`] as its one
/// trustee, and [`OUTSIDER`] as another trustee of the tenant.
async fn board(pool: &Pool) -> (Event, BoardName) {
    let event = rows::event(pool).await;
    let member = trustee(pool, &event.tenant_id, MEMBER).await;
    trustee(pool, &event.tenant_id, OUTSIDER).await;
    let tenant_id = Uuid::parse_str(&event.tenant_id).unwrap();
    let election_event_id = Uuid::parse_str(&event.election_event_id).unwrap();
    let keys_ceremony_id = Uuid::new_v4();
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.keys_ceremony
            (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, $4, 1)",
        &[
            &keys_ceremony_id,
            &tenant_id,
            &election_event_id,
            &vec![member],
        ],
    )
    .await;
    let name = BoardName::for_dkg(&keys_ceremony_id);
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.protocol_board
            (tenant_id, election_event_id, keys_ceremony_id, name,
             manager_message)
         VALUES ($1, $2, $3, $4, $5)",
        &[
            &tenant_id,
            &election_event_id,
            &keys_ceremony_id,
            &name.as_str(),
            &Vec::<u8>::new(),
        ],
    )
    .await;
    (event, name)
}

async fn write<'c>(
    client: &'c Client,
    path: String,
    authorization: Option<Header<'static>>,
    body: &str,
) -> LocalResponse<'c> {
    let mut request = client
        .post(path)
        .header(ContentType::JSON)
        .body(body.to_string());
    if let Some(authorization) = authorization {
        request = request.header(authorization);
    }
    request.dispatch().await
}

/// The status and plain-text body of one of Harvest's own refusals.
async fn refusal(response: LocalResponse<'_>) -> (Status, String) {
    let status = response.status();
    assert_eq!(
        response.content_type(),
        Some(ContentType::Plain),
        "{status}"
    );
    (status, response.into_string().await.unwrap_or_default())
}

fn allowed_origin(response: &LocalResponse<'_>) -> Option<String> {
    response.headers().get_one(ALLOW_ORIGIN).map(str::to_owned)
}

/// What reached b4: nothing but the request itself, never the caller's
/// credentials.
fn assert_forwarded(request: &Request, path: &str, body: &str) {
    assert_eq!(request.method, "POST");
    assert_eq!(request.url.path(), path);
    assert_eq!(request.body, body);
    assert_eq!(
        request.headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    assert!(
        !request.headers.contains_key("authorization"),
        "{:?}",
        request.headers
    );
}

#[rocket::async_test]
async fn b4_writes_without_valid_claims_are_refused_before_b4() {
    let peer = HttpServer::start(vec![]);
    let client = Services::without_database()
        .with_b4_url(&peer.url)
        .client()
        .await;
    let board = BoardName::for_dkg(&Uuid::new_v4());
    for path in [
        initiate_path(board.as_str()),
        confirm_path(board.as_str(), MESSAGE_ID),
    ] {
        for authorization in [
            None,
            Some(Header::new("Authorization", "Bearer fixture.e30.fixture")),
        ] {
            let response =
                write(&client, path.clone(), authorization, "{}").await;
            assert_eq!(response.status(), Status::Unauthorized, "{path}");
        }
        // The current answer of every trustee route without the permission.
        let response = write(
            &client,
            path.clone(),
            Some(token(&Uuid::new_v4().to_string(), Some(MEMBER), &[])),
            "{}",
        )
        .await;
        assert_eq!(refusal(response).await.0, Status::Unauthorized, "{path}");
    }
    assert!(peer.finish().is_empty());
}

#[rocket::async_test]
async fn b4_board_names_and_message_ids_that_do_not_parse_are_not_found() {
    let peer = HttpServer::start(vec![]);
    let client = Services::without_database()
        .with_b4_url(&peer.url)
        .client()
        .await;
    let tenant_id = Uuid::new_v4().to_string();
    let board = BoardName::for_dkg(&Uuid::new_v4());
    for name in ["not-a-board", "dkg_not-a-uuid"] {
        let response = client
            .get(format!("/b4/boards/{name}/messages"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound, "{name}");
        for path in [initiate_path(name), confirm_path(name, MESSAGE_ID)] {
            let response = write(
                &client,
                path.clone(),
                Some(trustee_token(&tenant_id, Some(MEMBER))),
                CONFIRM_BODY,
            )
            .await;
            assert_eq!(refusal(response).await.0, Status::NotFound, "{path}");
        }
    }
    let response = write(
        &client,
        confirm_path(board.as_str(), "not-a-uuid"),
        Some(trustee_token(&tenant_id, Some(MEMBER))),
        CONFIRM_BODY,
    )
    .await;
    assert_eq!(refusal(response).await.0, Status::NotFound);
    assert!(peer.finish().is_empty());
}

#[rocket::async_test]
async fn b4_writes_by_callers_that_are_not_the_boards_trustees_are_forbidden() {
    let peer = HttpServer::start(vec![]);
    let services = Services::on_test_database().await.with_b4_url(&peer.url);
    let client = services.client().await;
    let (event, board) = board(&services.hasura).await;
    // A tenant with a trustee of the same name, but not the board.
    let other = rows::event(&services.hasura).await;
    trustee(&services.hasura, &other.tenant_id, MEMBER).await;

    for (tenant_id, claim) in [
        (&other.tenant_id, Some(MEMBER)),
        (&event.tenant_id, None),
        (&event.tenant_id, Some("unknown-trustee")),
        (&event.tenant_id, Some(OUTSIDER)),
    ] {
        for (path, body) in [
            (initiate_path(board.as_str()), INITIATE_BODY),
            (confirm_path(board.as_str(), MESSAGE_ID), CONFIRM_BODY),
        ] {
            let response = write(
                &client,
                path.clone(),
                Some(trustee_token(tenant_id, claim)),
                body,
            )
            .await;
            assert_eq!(
                refusal(response).await.0,
                Status::Forbidden,
                "{path} as {claim:?} of {tenant_id}"
            );
        }
    }
    assert!(peer.finish().is_empty());
}

#[rocket::async_test]
async fn b4_writes_by_a_member_reach_b4_unchanged_and_without_credentials() {
    let services = Services::on_test_database().await;
    let (event, board) = board(&services.hasura).await;
    let initiated = json!({
        "message_id": MESSAGE_ID,
        "upload_url": "https://storage.test/upload?signature=short-lived",
        "should_upload": true,
    });
    let initiate = format!("/boards/{board}/messages/initiate");
    let confirm = format!("/boards/{board}/messages/{MESSAGE_ID}/confirm");
    let peer = HttpServer::start(vec![
        Exchange::json("POST", &initiate, 200, initiated.clone()),
        // b4's own refusal is passed back as it is.
        Exchange::json("POST", &confirm, 404, json!({"error": "no board"})),
    ]);
    let client = services.with_b4_url(&peer.url).client().await;
    let authorization = trustee_token(&event.tenant_id, Some(MEMBER));

    let response = write(
        &client,
        initiate_path(board.as_str()),
        Some(authorization.clone()),
        INITIATE_BODY,
    )
    .await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    assert_eq!(allowed_origin(&response), None);
    let body: Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert_eq!(body, initiated);

    let response = write(
        &client,
        confirm_path(board.as_str(), MESSAGE_ID),
        Some(authorization),
        CONFIRM_BODY,
    )
    .await;
    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    assert_eq!(allowed_origin(&response), None);
    assert_eq!(
        response.into_string().await.unwrap(),
        json!({"error": "no board"}).to_string()
    );

    let requests = peer.finish();
    assert_eq!(requests.len(), 2);
    assert_forwarded(&requests[0], &initiate, INITIATE_BODY);
    assert_forwarded(&requests[1], &confirm, CONFIRM_BODY);
}

#[rocket::async_test]
async fn b4_writes_larger_than_b4_reads_are_refused() {
    let peer = HttpServer::start(vec![]);
    let services = Services::on_test_database().await.with_b4_url(&peer.url);
    let client = services.client().await;
    let (event, board) = board(&services.hasura).await;
    let body = " ".repeat(usize::try_from(BODY_LIMIT_BYTES).unwrap() + 1);

    let response = write(
        &client,
        initiate_path(board.as_str()),
        Some(trustee_token(&event.tenant_id, Some(MEMBER))),
        &body,
    )
    .await;
    assert_eq!(refusal(response).await.0, Status::PayloadTooLarge);
    assert!(peer.finish().is_empty());
}

#[rocket::async_test]
async fn b4_writes_answer_bad_gateway_without_naming_b4_when_it_is_unreachable()
{
    let services = Services::on_test_database()
        .await
        .with_b4_url(UNREACHABLE_B4);
    let client = services.client().await;
    let (event, board) = board(&services.hasura).await;

    let response = write(
        &client,
        initiate_path(board.as_str()),
        Some(trustee_token(&event.tenant_id, Some(MEMBER))),
        INITIATE_BODY,
    )
    .await;
    assert_eq!(
        refusal(response).await,
        (Status::BadGateway, UNREACHABLE.to_string())
    );
}

#[rocket::async_test]
async fn b4_board_reads_are_public_and_pass_the_query_on() {
    let board = BoardName::for_dkg(&Uuid::new_v4());
    let path = format!("/boards/{board}/messages");
    let messages = json!({"messages": []});
    let peer = HttpServer::start(vec![
        Exchange::json("GET", &path, 200, messages.clone()),
        Exchange::json("GET", &path, 500, json!({"error": "b4 failed"})),
    ]);
    let client = Services::without_database()
        .with_b4_url(&peer.url)
        .client()
        .await;

    let response = client
        .get(format!("/b4{path}?last_id=7&limit=5"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    // By default only the admin portal reads, from its own origin.
    assert_eq!(allowed_origin(&response), None);
    let body: Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert_eq!(body, messages);

    // b4's failure is passed back as it is.
    let response = client.get(format!("/b4{path}")).dispatch().await;
    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(allowed_origin(&response), None);

    let requests = peer.finish();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url.path(), path);
    assert_eq!(requests[0].url.query(), Some("last_id=7&limit=5"));
    assert_eq!(requests[1].url.query(), None);
    assert!(!requests[0].headers.contains_key("authorization"));
}

#[rocket::async_test]
async fn b4_board_reads_answer_bad_gateway_without_naming_b4() {
    let board = BoardName::for_dkg(&Uuid::new_v4());
    for services in [
        Services::without_database().with_b4_url(UNREACHABLE_B4),
        // WBRAID_B4_URL unset.
        Services::without_database(),
    ] {
        let client = services.client().await;
        let response = client
            .get(format!("/b4/boards/{board}/messages"))
            .dispatch()
            .await;
        assert_eq!(allowed_origin(&response), None);
        assert_eq!(
            refusal(response).await,
            (Status::BadGateway, UNREACHABLE.to_string())
        );
    }
}

#[test]
fn b4_read_origins_are_parsed_from_their_variable() {
    for unset in [None, Some(""), Some("  ")] {
        assert_eq!(ReadOrigins::parse(unset), ReadOrigins::SameOrigin);
    }
    assert_eq!(ReadOrigins::parse(Some(" * ")), ReadOrigins::AnyOrigin);
    assert_eq!(
        ReadOrigins::parse(Some(
            "https://verifier.example, http://localhost:3000/,not an origin,\
             https://x.example/path"
        )),
        ReadOrigins::Listed(vec![
            "https://verifier.example".to_string(),
            "http://localhost:3000".to_string(),
        ])
    );
    // Nothing usable listed: only the admin portal's own origin.
    assert_eq!(
        ReadOrigins::parse(Some("ftp://x.example")),
        ReadOrigins::SameOrigin
    );
}

/// A board read from a page on `origin`, or from no page.
async fn read_board<'c>(
    client: &'c Client,
    path: &str,
    origin: Option<&str>,
) -> LocalResponse<'c> {
    let mut request = client.get(path.to_string());
    if let Some(origin) = origin {
        request = request.header(Header::new("Origin", origin.to_string()));
    }
    request.dispatch().await
}

#[rocket::async_test]
async fn b4_board_reads_allow_only_the_configured_origins() {
    const VERIFIER: &str = "https://verifier.example";
    let board = BoardName::for_dkg(&Uuid::new_v4());
    let path = format!("/b4/boards/{board}/messages");

    let client = Services::without_database()
        .with_b4_read_origins(ReadOrigins::AnyOrigin)
        .client()
        .await;
    let response = read_board(&client, &path, Some(VERIFIER)).await;
    assert_eq!(allowed_origin(&response).as_deref(), Some(ANY_ORIGIN));

    let client = Services::without_database()
        .with_b4_read_origins(ReadOrigins::Listed(vec![VERIFIER.to_string()]))
        .client()
        .await;
    let response = read_board(&client, &path, Some(VERIFIER)).await;
    assert_eq!(allowed_origin(&response).as_deref(), Some(VERIFIER));
    assert_eq!(response.headers().get_one("Vary"), Some("Origin"));
    for other in [Some("https://elsewhere.example"), None] {
        let response = read_board(&client, &path, other).await;
        assert_eq!(allowed_origin(&response), None);
        assert_eq!(response.headers().get_one("Vary"), Some("Origin"));
    }
}
