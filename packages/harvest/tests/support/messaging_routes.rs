// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging routes on the migrated test database: sending accounts and
//! their credentials, Keycloak's sending and Messenger link calls (with
//! realm-signed service tokens), provider callbacks relayed by the webhook
//! component and the event configuration. Messages leave through Amazon SNS
//! at the loopback provider (`messaging::test_server`), reached through the
//! deployment's AWS endpoint as a real deployment would set it, so nothing
//! reaches a network.

use super::{MessagingRelay, MetaHub, RelaySecret};
use crate::route_services::rows::{self, Event};
use crate::route_services::service_tokens::RealmSigner;
use crate::route_services::{
    http, json, post, post_as, text, Services, RELAY_SECRET,
};
use crate::test_claims::Claims;
use messaging::providers::{AWS_ENDPOINT_URL_ENV, META_GRAPH_URL_ENV};
use messaging::relay::{
    self, CallbackEnvelope, CallbackMethod, CallbackProvider, RelayResponse,
};
use messaging::test_server::{Reply, TestServer};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::messaging::{MessageReason, REALM_ATTR_MESSAGING};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use uuid::Uuid;

const USER_ID: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";
const PHONE: &str = "+34600000001";
const MASTER_SECRET: &str =
    "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";
const APP_SECRET: &str = "synthetic-app-secret-of-a-meta-application";
const CHILD: &str = "HARVEST_MESSAGING_CHILD";

/// The test database, with the master key credentials are encrypted under.
async fn services() -> Services {
    static MASTER: std::sync::Once = std::sync::Once::new();
    MASTER.call_once(|| std::env::set_var("MASTER_SECRET", MASTER_SECRET));
    Services::on_test_database().await
}

fn admin(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, USER_ID).roles([
        Permissions::MESSAGING_ACCOUNT_WRITE,
        Permissions::MESSAGING_CONFIG_WRITE,
    ])
}

/// Keycloak's service account in `tenant_id`.
fn keycloak(tenant_id: &str) -> Claims {
    Claims::new(tenant_id, "service-account-keycloak")
        .roles([Permissions::SERVICE_ACCOUNT])
        .azp(crate::services::service_auth::DEFAULT_SERVICE_CLIENT)
}

/// Keycloak's service token for `tenant_id`, signed by a realm key the
/// route services trust.
fn service_token(services: &Services, tenant_id: &str) -> String {
    let signer = RealmSigner::shared();
    services.trust(tenant_id, signer);
    signer.sign(&keycloak(tenant_id), tenant_id)
}

/// A refusal's status and reason: the account actions answer
/// `{"message": reason}` (for Hasura to pass on), the others plain text.
async fn refusal(response: LocalResponse<'_>) -> (Status, String) {
    let status = response.status();
    let json = response.content_type() == Some(ContentType::JSON);
    let body = response.into_string().await.unwrap_or_default();
    if json {
        let value: Value = serde_json::from_str(&body).expect("a JSON refusal");
        let message = value["message"].as_str().expect("a message").to_string();
        (status, message)
    } else {
        (status, body)
    }
}

fn uuid(id: &str) -> Uuid {
    Uuid::parse_str(id).expect("fixture ids are UUIDs")
}

/// The default SMS account, sending through Amazon SNS.
fn texts(name: &str) -> Value {
    json!({
        "channel": "SMS",
        "name": name,
        "sender": {
            "provider": "AWS_SNS",
            "sender_id": "Elections",
            "origination_number": null,
            "region": "eu-west-1",
        },
        "is_default": true,
    })
}

/// SNS's answer to a Publish.
fn published(message_id: &str) -> Reply {
    Reply::Raw(
        200,
        "text/xml",
        format!(
            r#"<PublishResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/"><PublishResult><MessageId>{message_id}</MessageId></PublishResult><ResponseMetadata><RequestId>r</RequestId></ResponseMetadata></PublishResponse>"#
        ),
    )
}

/// SNS's answer to the account check: out of the SMS sandbox.
fn in_production() -> Reply {
    Reply::Raw(
        200,
        "text/xml",
        r#"<GetSMSSandboxAccountStatusResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/"><GetSMSSandboxAccountStatusResult><IsInSandbox>false</IsInSandbox></GetSMSSandboxAccountStatusResult><ResponseMetadata><RequestId>r</RequestId></ResponseMetadata></GetSMSSandboxAccountStatusResponse>"#.to_string(),
    )
}

/// The loopback Graph API, as the deployment's Meta address, answering the
/// check of a replaced Page token: the Page is found. The check's other
/// requests find no answer, which it reports without refusing the token.
/// The dispatcher reads it once per process: only in an isolated test,
/// before the first request.
async fn page_found() -> TestServer {
    // The Page, then `debug_token`: a replacing token must be able to send
    // (R5-11).
    let server = TestServer::start(vec![
        Reply::Json(200, json!({"id": "page-1", "name": "Elections"})),
        Reply::Json(
            200,
            json!({"data": {"is_valid": true, "expires_at": 0,
                            "scopes": ["pages_messaging"]}}),
        ),
    ])
    .await;
    std::env::set_var(META_GRAPH_URL_ENV, &server.base_url);
    server
}

/// The loopback provider, as the deployment's AWS endpoint. The dispatcher
/// reads it once per process: only in an isolated test.
async fn sns_answering(replies: Vec<Reply>) -> TestServer {
    let server = TestServer::start(replies).await;
    std::env::set_var(AWS_ENDPOINT_URL_ENV, &server.base_url);
    server
}

fn sns(name: &str) -> Value {
    json!({"name": name, "sender": {"provider": "AWS_SNS"}})
}

fn messenger() -> Value {
    json!({
        "name": "Page",
        "sender": {
            "provider": "MESSENGER_SEND_API",
            "page_id": "page-1",
            "page_name": "Elections",
            "page_username": "elections",
            "api_version": "v23.0",
        },
    })
}

/// A provider described by configuration; `reports` makes it take callbacks.
fn http_api(reports: bool) -> Value {
    let mut sender = json!({
        "provider": "HTTP_API",
        "send": {"url": "http://127.0.0.1:1/send"},
    });
    if reports {
        sender["reports"] = json!({"status": {
            "message_id_pointer": "/id",
            "state_pointer": "/status",
            "states": {"delivered": "DELIVERED"},
        }});
    }
    json!({"channel": "SMS", "name": "Gateway", "sender": sender})
}

/// The default SMS account with the AWS keys the SDK signs with.
async fn texts_account(client: &Client, event: &Event) -> String {
    let id = account(client, event, texts("Texts")).await;
    let (status, replaced) = json(
        credentials(
            client,
            event,
            json!({"id": id, "credentials": {
                "AWS_ACCESS_KEY_ID": "AKIDEXAMPLE",
                "AWS_SECRET_ACCESS_KEY": "synthetic-secret",
            }}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{replaced}");
    id
}

/// A stored account: its id.
async fn account(client: &Client, event: &Event, body: Value) -> String {
    let (status, created) = json(
        post(client, "/messaging/accounts/upsert", &admin(event), &body).await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{created}");
    created["id"].as_str().expect("an account id").to_string()
}

async fn credentials<'c>(
    client: &'c Client,
    event: &Event,
    body: Value,
) -> LocalResponse<'c> {
    post(
        client,
        "/messaging/accounts/credentials",
        &admin(event),
        &body,
    )
    .await
}

/// A Messenger account with its Page token and app secret; returns its id
/// and the verify token generated for it.
async fn meta_account(client: &Client, event: &Event) -> (String, String) {
    let id = account(client, event, messenger()).await;
    let (status, replaced) = json(
        credentials(
            client,
            event,
            json!({
                "id": id,
                "credentials": {
                    "ACCESS_TOKEN": "page-token",
                    "APP_SECRET": APP_SECRET,
                },
                "generate_verify_token": true,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{replaced}");
    let token = replaced["verify_token"].as_str().expect("a verify token");
    (id, token.to_string())
}

async fn webhook_key(services: &Services, account_id: &str) -> String {
    rows::query(
        &services.hasura,
        "SELECT webhook_key FROM sequent_backend.messaging_account
         WHERE id = $1",
        &[&uuid(account_id)],
    )
    .await[0]
        .get(0)
}

/// Channel, name and default flag of a stored account.
async fn stored_account(
    services: &Services,
    account_id: &str,
) -> (String, String, bool) {
    let rows = rows::query(
        &services.hasura,
        "SELECT channel, name, is_default
         FROM sequent_backend.messaging_account WHERE id = $1",
        &[&uuid(account_id)],
    )
    .await;
    (rows[0].get(0), rows[0].get(1), rows[0].get(2))
}

/// Store the event's messaging configuration as the save does, without
/// validating it or publishing it to a realm.
async fn configure(services: &Services, event: &Event, config: Value) {
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.messaging_event_config
             (tenant_id, election_event_id, config)
         VALUES ($1, $2, $3)
         ON CONFLICT (election_event_id) DO UPDATE SET config = EXCLUDED.config",
        &[
            &uuid(&event.tenant_id),
            &uuid(&event.election_event_id),
            &config,
        ],
    )
    .await;
}

async fn stored_config(services: &Services, event: &Event) -> Option<Value> {
    rows::query(
        &services.hasura,
        "SELECT config FROM sequent_backend.messaging_event_config
         WHERE election_event_id = $1",
        &[&uuid(&event.election_event_id)],
    )
    .await
    .first()
    .map(|row| row.get(0))
}

/// Mark an account as checked and in production, as a passing check would.
async fn checked(services: &Services, account_id: &str) {
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.messaging_account
         SET status = '{\"connected\": true, \"production_access\": true}'
         WHERE id = $1",
        &[&uuid(account_id)],
    )
    .await;
}

/// Make every credential of the tenant's accounts undecipherable.
async fn corrupt_credentials(services: &Services, event: &Event) {
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.secret SET value = '\\x00'
         WHERE tenant_id = $1 AND key LIKE 'messaging-account-%'",
        &[&uuid(&event.tenant_id)],
    )
    .await;
}

fn notice(event: &Event, logical_key: &str) -> Value {
    json!({
        "tenant_id": event.tenant_id,
        "election_event_id": null,
        "voter_id": "voter-1",
        "channel": "SMS",
        "purpose": "NOTICE",
        "destination": PHONE,
        "language": "en",
        "content": {"text": "Voting opens tomorrow."},
        "logical_key": logical_key,
    })
}

async fn send<'c>(
    client: &'c Client,
    services: &Services,
    tenant_id: &str,
    body: &Value,
) -> LocalResponse<'c> {
    post_as(
        client,
        "/messages/send",
        &service_token(services, tenant_id),
        body,
    )
    .await
}

fn link(event: &Event, expires_in: chrono::Duration) -> Value {
    json!({
        "tenant_id": event.tenant_id,
        "election_event_id": event.election_event_id,
        "auth_session": "session-digest",
        "challenge": "challenge-digest",
        "code": "482619",
        "language": "en",
        "content": {"text": "Your code is 482619."},
        "expires_at": (chrono::Utc::now() + expires_in).to_rfc3339(),
    })
}

fn link_request(tenant_id: &str, reference: &str, session: &str) -> Value {
    json!({
        "tenant_id": tenant_id,
        "reference": reference,
        "auth_session": session,
        "challenge": "challenge-digest",
    })
}

/// `X-Hub-Signature-256` of `body` under the app secret.
fn hub_signature(body: &[u8]) -> String {
    let key = openssl::pkey::PKey::hmac(APP_SECRET.as_bytes()).unwrap();
    let mut signer = openssl::sign::Signer::new(
        openssl::hash::MessageDigest::sha256(),
        &key,
    )
    .unwrap();
    let mac = signer.sign_oneshot_to_vec(body).unwrap();
    format!("sha256={}", hex::encode(mac))
}

#[rocket::async_test]
async fn accounts_take_their_providers_channel_and_one_default_per_channel() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    // Amazon SNS only sends SMS, whatever channel the request names.
    let mut first = sns("Texts");
    first["channel"] = json!("EMAIL");
    first["is_default"] = json!(true);
    let id = account(&client, &event, first).await;
    assert_eq!(
        stored_account(&services, &id).await,
        ("SMS".into(), "Texts".into(), true)
    );

    let other = account(&client, &event, texts("Other texts")).await;
    assert_eq!(
        stored_account(&services, &other).await,
        ("SMS".into(), "Other texts".into(), true)
    );

    let mut renamed = sns("Texts (Lisbon)");
    renamed["id"] = json!(id);
    assert_eq!(
        json(
            post(
                &client,
                "/messaging/accounts/upsert",
                &admin(&event),
                &renamed
            )
            .await
        )
        .await,
        (Status::Ok, json!({ "id": id }))
    );
    assert_eq!(
        stored_account(&services, &id).await,
        ("SMS".into(), "Texts (Lisbon)".into(), false)
    );
}

#[rocket::async_test]
async fn account_changes_that_cannot_be_stored_are_refused() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(&client, &event, sns("Texts")).await;

    let with_id = |mut body: Value, id: &str| {
        body["id"] = json!(id);
        body
    };
    for (body, status, message) in [
        (
            json!({"name": "Console", "sender": {"provider": "CONSOLE"}}),
            Status::BadRequest,
            "this provider needs a channel",
        ),
        // Removed providers: a stored account loads, a new one is refused.
        (
            json!({"channel": "SMS", "name": "Console", "sender": {"provider": "CONSOLE"}}),
            Status::BadRequest,
            "Unsupported provider: replace this account",
        ),
        (sns("  "), Status::BadRequest, "the account needs a name"),
        (
            with_id(sns("Texts"), "not-a-uuid"),
            Status::BadRequest,
            "invalid identifier",
        ),
        (
            with_id(sns("Texts"), &Uuid::new_v4().to_string()),
            Status::NotFound,
            "unknown account",
        ),
        (
            with_id(
                json!({"name": "Mail", "sender": {
                    "provider": "SMTP", "from_address": "elections@example.org",
                    "from_name": null, "server_url": "smtps://smtp.example.org",
                }}),
                &id,
            ),
            Status::BadRequest,
            "the provider and channel of an account cannot change; add another account",
        ),
    ] {
        assert_eq!(
            refusal(
                post(
                    &client,
                    "/messaging/accounts/upsert",
                    &admin(&event),
                    &body
                )
                .await
            )
            .await,
            (status, message.to_string()),
            "{body}"
        );
    }
    assert_eq!(
        stored_account(&services, &id).await,
        ("SMS".into(), "Texts".into(), false)
    );
}

#[rocket::async_test]
async fn a_deleted_account_is_gone_with_its_credentials() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, _) = meta_account(&client, &event).await;
    let delete = |id: String| {
        let (client, claims) = (&client, admin(&event));
        async move {
            post(
                client,
                "/messaging/accounts/delete",
                &claims,
                &json!({ "id": id }),
            )
            .await
        }
    };

    assert_eq!(
        json(delete(id.clone()).await).await,
        (Status::Ok, json!({ "id": id }))
    );
    assert!(rows::query(
        &services.hasura,
        "SELECT key FROM sequent_backend.secret
         WHERE tenant_id = $1 AND key LIKE 'messaging-account-%'",
        &[&uuid(&event.tenant_id)],
    )
    .await
    .is_empty());
    assert_eq!(
        refusal(delete(id).await).await,
        (Status::NotFound, "unknown account".to_string())
    );
    // Refusals reach Hasura as `{"message": reason}` (FX-EF's AB-3).
    assert_eq!(
        json(delete(Uuid::new_v4().to_string()).await).await,
        (Status::NotFound, json!({"message": "unknown account"}))
    );
    assert_eq!(
        refusal(delete("not-a-uuid".to_string()).await).await,
        (Status::BadRequest, "invalid identifier".to_string())
    );
}

/// BC-F13 (T2 gap): the drawer's warning before a Page change, as the
/// route answers it: the voters connected to the account's current Page in
/// the realms of the events that use it, and how many events those are.
#[rocket::async_test]
async fn a_page_change_answers_how_many_voters_must_connect_again() {
    use sequent_core::types::keycloak::MESSENGER_PAGE_ATTR_NAME;
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(&client, &event, messenger()).await;
    configure(
        &services,
        &event,
        json!({"version": 1, "channels": [{"channel": "MESSENGER",
            "account_id": id, "purposes": ["OTP"], "templates": []}],
            "election_channels": {}}),
    )
    .await;
    let realm = rows::keycloak_realm(&services.hasura, &event.realm()).await;
    for (name, page) in [
        ("on-page", "page-1"),
        ("also", "page-1"),
        ("other", "page-2"),
    ] {
        let user =
            rows::keycloak_user(&services.hasura, &realm, name, true).await;
        rows::execute(
            &services.hasura,
            "INSERT INTO user_attribute (name, value, user_id)
             VALUES ($1, $2, $3)",
            &[&MESSENGER_PAGE_ATTR_NAME, &page, &user],
        )
        .await;
    }
    for (new_page, voters) in [("page-3", 2), ("page-1", 0)] {
        assert_eq!(
            json(
                post(
                    &client,
                    "/messaging/accounts/change-impact",
                    &admin(&event),
                    &json!({"account_id": id, "page_id": new_page}),
                )
                .await,
            )
            .await,
            (Status::Ok, json!({"affected_voters": voters, "events": 1})),
            "{new_page}"
        );
    }
    assert_eq!(
        refusal(
            post(
                &client,
                "/messaging/accounts/change-impact",
                &admin(&event),
                &json!({"account_id": Uuid::new_v4(), "page_id": "page-3"}),
            )
            .await,
        )
        .await,
        (Status::NotFound, "unknown account".to_string())
    );
}

#[rocket::async_test]
async fn replaced_credentials_are_named_and_the_verify_token_shown_once() {
    if !isolated("routes::messaging::route_tests::replaced_credentials_are_named_and_the_verify_token_shown_once") {
        return;
    }
    // The replaced token is checked before it is saved.
    let _graph = page_found().await;
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, token) = meta_account(&client, &event).await;
    assert_eq!(token.len(), 64, "{token}");

    let (status, replaced) = json(
        credentials(
            &client,
            &event,
            json!({"id": id, "credentials": {"ACCESS_TOKEN": " rotated "}}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{replaced}");
    assert_eq!(
        replaced,
        json!({
            "replaced": ["ACCESS_TOKEN"], "verify_token": null,
            "report_username": null, "report_password": null,
        })
    );
    // Values never reach the account row, only when each was replaced.
    let recorded: Value = rows::query(
        &services.hasura,
        "SELECT to_jsonb(account) FROM sequent_backend.messaging_account account
         WHERE id = $1",
        &[&uuid(&id)],
    )
    .await[0]
        .get(0);
    let recorded = recorded.to_string();
    for name in ["ACCESS_TOKEN", "APP_SECRET", "VERIFY_TOKEN"] {
        assert!(recorded.contains(name), "{recorded}");
    }
    for value in ["rotated", "page-token", APP_SECRET, token.as_str()] {
        assert!(!recorded.contains(value), "{recorded}");
    }
}

/// R5-04 regression: the candidate check called the provider at the
/// address being saved before the save's own rules ran, so an
/// administrator could make harvest probe an internal `http://` address
/// and read the outcome from the refusal. The save's rules run first; the
/// provider is never called.
#[rocket::async_test]
async fn a_refused_address_is_never_called_by_the_candidate_check() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(
        &client,
        &event,
        json!({"name": "Viber", "sender": {
            "provider": "VIBER_INFOBIP",
            "base_url": "https://127.0.0.1:1",
            "sender": "Elections",
        }}),
    )
    .await;
    let (status, replaced) = json(
        credentials(
            &client,
            &event,
            json!({"id": id, "credentials": {"API_KEY": "first-key"}}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{replaced}");
    let internal = TestServer::start(vec![Reply::Json(404, json!({}))]).await;
    let (status, refused) = refusal(
        post(
            &client,
            "/messaging/accounts/upsert",
            &admin(&event),
            &json!({
                "id": id,
                "name": "Viber",
                "sender": {
                    "provider": "VIBER_INFOBIP",
                    "base_url": internal.base_url,
                    "sender": "Elections",
                },
                "credentials": {"API_KEY": "probe"},
            }),
        )
        .await,
    )
    .await;
    assert_eq!(
        (status, refused.as_str()),
        (
            Status::BadRequest,
            "The provider address must start with https://"
        )
    );
    assert_eq!(internal.served(), 0, "the refused address was called");
}

#[rocket::async_test]
async fn credentials_an_account_cannot_use_are_refused() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(&client, &event, texts("Texts")).await;

    for (body, status, message) in [
        (
            json!({"id": id, "credentials": {"NOPE": "x"}}),
            Status::BadRequest,
            "unknown credential NOPE",
        ),
        (
            json!({"id": id, "credentials": {"API_KEY": "x"}}),
            Status::BadRequest,
            "AWS_SNS accounts do not use API_KEY",
        ),
        (
            json!({"id": id, "credentials": {}, "generate_verify_token": true}),
            Status::BadRequest,
            "AWS_SNS accounts have no verify token",
        ),
        (
            json!({"id": Uuid::new_v4(), "credentials": {}}),
            Status::NotFound,
            "unknown account",
        ),
        (
            json!({"id": "not-a-uuid", "credentials": {}}),
            Status::BadRequest,
            "invalid identifier",
        ),
    ] {
        assert_eq!(
            refusal(credentials(&client, &event, body.clone()).await).await,
            (status, message.to_string()),
            "{body}"
        );
    }
}

#[rocket::async_test]
async fn an_account_check_stores_what_the_provider_answered() {
    if !isolated("routes::messaging::route_tests::an_account_check_stores_what_the_provider_answered") {
        return;
    }
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let sns = sns_answering(vec![in_production()]).await;
    let id = texts_account(&client, &event).await;
    let check = |id: String| {
        let (client, claims) = (&client, admin(&event));
        async move {
            post(
                client,
                "/messaging/accounts/check",
                &claims,
                &json!({ "id": id }),
            )
            .await
        }
    };

    let (status, checked) = json(check(id.clone()).await).await;
    assert_eq!(status, Status::Ok, "{checked}");
    assert_eq!(checked["status"]["connected"], true);
    assert_eq!(checked["status"]["production_access"], true);
    let stored: Value = rows::query(
        &services.hasura,
        "SELECT status FROM sequent_backend.messaging_account WHERE id = $1",
        &[&uuid(&id)],
    )
    .await[0]
        .get(0);
    assert_eq!(stored, checked["status"]);
    assert_eq!(sns.requests().len(), 1);

    assert_eq!(
        refusal(check(Uuid::new_v4().to_string()).await).await,
        (Status::NotFound, "unknown account".to_string())
    );
    assert_eq!(
        refusal(check("not-a-uuid".to_string()).await).await,
        (Status::BadRequest, "invalid identifier".to_string())
    );

    // Credentials that cannot be read fail the check before any request.
    let (page, _) = meta_account(&client, &event).await;
    corrupt_credentials(&services, &event).await;
    assert_eq!(
        refusal(check(page).await).await,
        (Status::InternalServerError, "internal error".to_string())
    );
}

#[rocket::async_test]
async fn a_test_message_goes_through_the_chosen_account() {
    if !isolated("routes::messaging::route_tests::a_test_message_goes_through_the_chosen_account") {
        return;
    }
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let sns = sns_answering(vec![published("sns-1"), published("sns-2")]).await;
    let id = texts_account(&client, &event).await;
    let test = |body: Value| {
        let (client, claims) = (&client, admin(&event));
        async move { post(client, "/messaging/accounts/test", &claims, &body).await }
    };

    let mut sent = vec![];
    for body in [
        json!({"id": id, "purpose": "NOTICE", "destination": PHONE, "template": " "}),
        json!({
            "id": id, "purpose": "OTP", "destination": PHONE,
            "language": "en", "template": "step_code",
        }),
    ] {
        let (status, result) = json(test(body).await).await;
        assert_eq!(status, Status::Ok, "{result}");
        assert_eq!(result["state"], "ACCEPTED", "{result}");
        assert_eq!(result["reason"], Value::Null);
        sent.push(uuid(result["message_id"].as_str().unwrap()));
    }
    let recorded: Vec<(String, String, String)> = rows::query(
        &services.hasura,
        "SELECT purpose, template_alias, masked_destination
         FROM sequent_backend.message WHERE id = ANY($1) ORDER BY purpose",
        &[&sent],
    )
    .await
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get(2)))
    .collect();
    assert_eq!(
        recorded,
        vec![
            (
                "NOTICE".into(),
                "account-test".into(),
                "+34 ••• ••• 001".into()
            ),
            (
                "OTP".into(),
                "account-test".into(),
                "+34 ••• ••• 001".into()
            ),
        ]
    );

    assert_eq!(sns.requests().len(), 2);
    let (status, refused) = json(
        test(json!({"id": id, "purpose": "NOTICE", "destination": "nobody"}))
            .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{refused}");
    assert_eq!(refused["state"], "FAILED", "{refused}");
    assert!(refused["reason"].is_string(), "{refused}");

    assert_eq!(
        refusal(
            test(json!({
                "id": Uuid::new_v4(), "purpose": "NOTICE", "destination": PHONE,
            }))
            .await
        )
        .await,
        (Status::NotFound, "unknown account".to_string())
    );
    assert_eq!(
        refusal(
            test(json!({
                "id": "not-a-uuid", "purpose": "NOTICE", "destination": PHONE,
            }))
            .await
        )
        .await,
        (Status::BadRequest, "invalid identifier".to_string())
    );
}

#[rocket::async_test]
async fn keycloak_sends_a_message_once_per_logical_key() {
    if !isolated("routes::messaging::route_tests::keycloak_sends_a_message_once_per_logical_key") {
        return;
    }
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let sns = sns_answering(vec![published("sns-1"), published("sns-2")]).await;
    let id = texts_account(&client, &event).await;
    checked(&services, &id).await;

    let (status, first) = json(
        send(
            &client,
            &services,
            &event.tenant_id,
            &notice(&event, "enrolled"),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{first}");
    assert_eq!(first["state"], "ACCEPTED", "{first}");
    let (_, replay) = json(
        send(
            &client,
            &services,
            &event.tenant_id,
            &notice(&event, "enrolled"),
        )
        .await,
    )
    .await;
    assert_eq!(replay, first);

    let mut code = notice(&event, "code-1");
    code["purpose"] = json!("OTP");
    code["election_event_id"] = json!(event.election_event_id);
    code["content"]["code"] = json!("482619");
    code["expires_at"] = json!("2099-01-01T00:00:00+02:00");
    let (status, sent) =
        json(send(&client, &services, &event.tenant_id, &code).await).await;
    assert_eq!(status, Status::Ok, "{sent}");
    assert_eq!(sent["state"], "ACCEPTED", "{sent}");

    let keys: Vec<String> = rows::query(
        &services.hasura,
        "SELECT logical_key FROM sequent_backend.message
         WHERE tenant_id = $1 ORDER BY logical_key",
        &[&uuid(&event.tenant_id)],
    )
    .await
    .iter()
    .map(|row| row.get(0))
    .collect();
    assert_eq!(keys, vec!["keycloak:code-1", "keycloak:enrolled"]);
    // The replay wasn't sent again.
    assert_eq!(sns.requests().len(), 2);
}

#[rocket::async_test]
async fn messages_that_cannot_be_sent_are_refused_or_reported_as_failed() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let edited = |field: &str, value: Value| {
        let mut body = notice(&event, "k");
        body[field] = value;
        body
    };

    for (body, message) in [
        (edited("logical_key", json!("  ")), "missing logical_key"),
        (edited("expires_at", json!("tomorrow")), "invalid time"),
        (edited("purpose", json!("OTP")), "codes need expires_at"),
        (
            edited("election_event_id", json!("not-a-uuid")),
            "invalid identifier",
        ),
    ] {
        assert_eq!(
            text(send(&client, &services, &event.tenant_id, &body).await).await,
            (Status::BadRequest, message.to_string()),
            "{body}"
        );
    }
    // The token's tenant must be the request's.
    for tenant_id in ["not-a-uuid".to_string(), Uuid::new_v4().to_string()] {
        assert_eq!(
            text(
                send(
                    &client,
                    &services,
                    &event.tenant_id,
                    &edited("tenant_id", json!(tenant_id))
                )
                .await
            )
            .await,
            (
                Status::Forbidden,
                "The request is for another tenant".to_string()
            )
        );
    }

    // Without an event configuration only email and SMS are offered: one
    // FAILED record, with its reason.
    let (status, refused) = json(
        send(
            &client,
            &services,
            &event.tenant_id,
            &edited("channel", json!("VIBER")),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{refused}");
    assert_eq!(refused["state"], "FAILED", "{refused}");
    assert_eq!(refused["reason"], "Channel not offered for notices");

    configure(
        &services,
        &event,
        json!({"version": 1, "channels": [{
            "channel": "SMS", "account_id": "not-a-uuid", "purposes": ["NOTICE"],
        }]}),
    )
    .await;
    // A stored channel whose account no longer exists isn't sent through
    // (send-time check of the configuration's references).
    let (status, refused) = json(
        send(
            &client,
            &services,
            &event.tenant_id,
            &edited("election_event_id", json!(event.election_event_id)),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{refused}");
    assert_eq!(refused["state"], "FAILED", "{refused}");
    assert!(refused["reason"].is_string(), "{refused}");
}

#[rocket::async_test]
async fn a_messenger_link_is_confirmed_only_in_its_session_once_the_code_was_sent(
) {
    // Get Started reaches Step through the webhook component.
    let services = services()
        .await
        .with_messaging_webhooks_url("https://callbacks.example.org");
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, _) = meta_account(&client, &event).await;
    configure(
        &services,
        &event,
        json!({"version": 1, "channels": [{
            "channel": "MESSENGER", "account_id": id, "purposes": ["OTP"],
        }]}),
    )
    .await;
    let service = service_token(&services, &event.tenant_id);

    let (status, created) = json(
        post_as(
            &client,
            "/messages/link",
            &service,
            &link(&event, chrono::Duration::minutes(5)),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{created}");
    let reference = created["reference"].as_str().unwrap();
    assert_eq!(
        created["link"],
        format!("https://m.me/elections?ref={reference}")
    );
    // No linking word any more: referral and Get Started only.
    assert!(
        created.get("link_word").map_or(true, Value::is_null),
        "{created}"
    );

    let own = link_request(&event.tenant_id, reference, "session-digest");
    let pending = json!({
        "state": "PENDING", "page_scoped_id": null, "page_id": null,
    });
    for path in ["/messages/link/status", "/messages/link/confirm"] {
        assert_eq!(
            json(post_as(&client, path, &service, &own).await).await,
            (Status::Ok, pending.clone()),
            "{path}"
        );
        for other in [
            link_request(&event.tenant_id, reference, "another-session"),
            link_request(&event.tenant_id, "unknown", "session-digest"),
        ] {
            assert_eq!(
                text(post_as(&client, path, &service, &other).await).await,
                (Status::NotFound, "unknown link".to_string()),
                "{path}: {other}"
            );
        }
    }

    // The voter opened the Page and the code was handed to Messenger.
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.messenger_link
         SET state = 'CODE_SENT', page_scoped_id = 'psid-1'
         WHERE tenant_id = $1",
        &[&uuid(&event.tenant_id)],
    )
    .await;
    assert_eq!(
        json(post_as(&client, "/messages/link/confirm", &service, &own).await)
            .await,
        (
            Status::Ok,
            json!({
                "state": "CONFIRMED",
                "page_scoped_id": "psid-1",
                "page_id": "page-1",
            })
        )
    );
    let (_, status) =
        json(post_as(&client, "/messages/link/status", &service, &own).await)
            .await;
    assert_eq!(status["state"], "CONFIRMED");
}

/// R7-10, the contract Keycloak's sign-in uses: a saved voter's link names
/// their Page (`expected_page`) and is refused with the reason when the
/// account is on another one; the reconnect's link names neither the saved
/// conversation nor the Page and is accepted; each created link says the
/// account's Page.
#[rocket::async_test]
async fn a_saved_voters_link_on_another_page_is_refused_with_the_reason() {
    let services = services()
        .await
        .with_messaging_webhooks_url("https://callbacks.example.org");
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, _) = meta_account(&client, &event).await;
    configure(
        &services,
        &event,
        json!({"version": 1, "channels": [{
            "channel": "MESSENGER", "account_id": id, "purposes": ["OTP"],
        }]}),
    )
    .await;
    let service = service_token(&services, &event.tenant_id);
    let saved = |page: Option<&str>| {
        let mut body = link(&event, chrono::Duration::minutes(5));
        body["expected_recipient"] = json!("psid-saved");
        if let Some(page) = page {
            body["expected_page"] = json!(page);
        }
        body
    };

    assert_eq!(
        text(
            post_as(
                &client,
                "/messages/link",
                &service,
                &saved(Some("page-old"))
            )
            .await
        )
        .await,
        (
            Status::Conflict,
            MessageReason::MESSENGER_PAGE_CHANGED.as_str().to_string()
        )
    );
    // On the account's own Page, and from a Keycloak that doesn't send the
    // Page yet, the saved conversation is used as before.
    for body in [saved(Some("page-1")), saved(None)] {
        let (status, created) =
            json(post_as(&client, "/messages/link", &service, &body).await)
                .await;
        assert_eq!(status, Status::Ok, "{created}");
        assert_eq!(created["page_id"], "page-1", "{created}");
    }
    // The reconnect: neither field.
    let (status, created) = json(
        post_as(
            &client,
            "/messages/link",
            &service,
            &link(&event, chrono::Duration::minutes(5)),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{created}");
    assert_eq!(created["page_id"], "page-1", "{created}");
}

#[rocket::async_test]
async fn messenger_links_need_an_event_that_offers_messenger_for_codes() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let service = service_token(&services, &event.tenant_id);

    assert_eq!(
        text(
            post_as(
                &client,
                "/messages/link",
                &service,
                &link(&event, chrono::Duration::minutes(5)),
            )
            .await
        )
        .await,
        (
            Status::UnprocessableEntity,
            "Messenger is not enabled for codes".to_string()
        )
    );
    assert_eq!(
        text(
            post_as(
                &client,
                "/messages/link",
                &service,
                &link(&event, chrono::Duration::minutes(-1)),
            )
            .await
        )
        .await,
        (
            Status::BadRequest,
            "the code has already expired".to_string()
        )
    );

    // Another tenant's links are out of the token's reach.
    for path in ["/messages/link/status", "/messages/link/confirm"] {
        for tenant_id in ["not-a-uuid".to_string(), Uuid::new_v4().to_string()]
        {
            assert_eq!(
                text(
                    post_as(
                        &client,
                        path,
                        &service,
                        &link_request(
                            &tenant_id,
                            "reference",
                            "session-digest"
                        ),
                    )
                    .await
                )
                .await,
                (
                    Status::Forbidden,
                    "The request is for another tenant".to_string()
                ),
                "{path}"
            );
        }
    }
}

/// A provider callback as the webhook component relays it: the provider's
/// answer when harvest gave one (HTTP 200), else harvest's status.
async fn relayed(
    client: &Client,
    provider: CallbackProvider,
    method: CallbackMethod,
    key: &str,
    query: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Result<RelayResponse, Status> {
    let envelope = CallbackEnvelope::new(
        provider,
        key,
        method,
        query,
        headers.iter().copied(),
        body,
        chrono::Utc::now(),
    );
    let payload = serde_json::to_vec(&envelope).expect("an envelope");
    let signature =
        relay::sign(RELAY_SECRET, chrono::Utc::now().timestamp(), &payload);
    let response = client
        .post(relay::CALLBACKS_PATH)
        .header(ContentType::JSON)
        .header(Header::new(relay::RELAY_SIGNATURE_HEADER, signature))
        .body(payload)
        .dispatch()
        .await;
    if response.status() == Status::Ok {
        Ok(response.into_json().await.expect("a relay response"))
    } else {
        Err(response.status())
    }
}

/// The status the provider gets, when harvest answered.
fn provider_status(
    answer: Result<RelayResponse, Status>,
) -> Result<u16, Status> {
    answer.map(|answer| answer.status)
}

/// A Meta POST, signed or not.
async fn meta_post(
    client: &Client,
    key: &str,
    signature: Option<&str>,
    body: &[u8],
) -> Result<u16, Status> {
    let headers: Vec<(&str, &str)> = signature
        .map(|signature| ("x-hub-signature-256", signature))
        .into_iter()
        .collect();
    provider_status(
        relayed(
            client,
            CallbackProvider::META,
            CallbackMethod::POST,
            key,
            "",
            &headers,
            body,
        )
        .await,
    )
}

/// Regression (R1-12): a service token of one tenant naming another
/// tenant's election event is refused before anything is sent or linked.
#[rocket::async_test]
async fn a_service_token_cannot_name_another_tenants_event() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let other = rows::event(&services.hasura).await;
    let token = service_token(&services, &event.tenant_id);
    let mut notice = notice(&event, "other-tenants-event");
    notice["election_event_id"] = json!(other.election_event_id);
    let mut link = link(&event, chrono::Duration::minutes(5));
    link["election_event_id"] = json!(other.election_event_id);
    for (path, body) in [("/messages/send", notice), ("/messages/link", link)] {
        assert_eq!(
            text(post_as(&client, path, &token, &body).await).await,
            (
                Status::Forbidden,
                "The election event isn't one of the tenant's".to_string()
            ),
            "{path}"
        );
    }
    let sent: i64 = rows::query(
        &services.hasura,
        "SELECT count(*) FROM sequent_backend.message WHERE tenant_id = $1",
        &[&uuid(&event.tenant_id)],
    )
    .await[0]
        .get(0);
    assert_eq!(sent, 0);
}

/// Regression (R1-05, R1-06, R4-04): what stops an account's callbacks
/// reached no reader, so the UI showed "Ready for OTP" and "Delivered 0".
/// Readers of the event tab get each account's blockers and reasons.
#[rocket::async_test]
async fn readers_see_what_stops_each_accounts_callbacks() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let viber = account(
        &client,
        &event,
        json!({
            "name": "Viber",
            "sender": {
                "provider": "VIBER_INFOBIP",
                "base_url": "https://a.api.infobip.com",
                "sender": "Elections",
            },
        }),
    )
    .await;
    let sms = texts_account(&client, &event).await;
    let reader = Claims::new(&event.tenant_id, USER_ID)
        .roles([Permissions::MESSAGING_CONFIG_READ]);
    let (status, body) = json(
        post(
            &client,
            "/messaging/accounts/callback-status",
            &reader,
            &json!({}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let of = |id: &str| {
        body["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|account| account["account_id"] == id)
            .cloned()
            .unwrap()
    };
    // The test process sets no public address for the component.
    assert_eq!(
        of(&viber),
        json!({
            "account_id": viber,
            "blockers": ["CALLBACK_NOT_CONFIGURED", "REPORT_CREDENTIALS_MISSING"],
            "reasons": [
                "Callback address not configured",
                "Generate the report credentials and create the Infobip subscription",
            ],
        })
    );
    assert_eq!(
        of(&sms),
        json!({"account_id": sms, "blockers": [], "reasons": []})
    );
    assert!(!body.to_string().contains("webhooks/"));
    let stranger = Claims::new(&event.tenant_id, USER_ID)
        .roles([Permissions::MESSAGING_CONFIG_WRITE]);
    assert_eq!(
        post(
            &client,
            "/messaging/accounts/callback-status",
            &stranger,
            &json!({})
        )
        .await
        .status(),
        Status::Unauthorized
    );
}

#[rocket::async_test]
async fn the_meta_subscription_echoes_the_challenge_for_the_verify_token() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, token) = meta_account(&client, &event).await;
    let key = webhook_key(&services, &id).await;
    let query = |token: &str| {
        format!(
            "hub.mode=subscribe&hub.verify_token={token}&hub.challenge=1158201444"
        )
    };
    let subscribe = |key: String, query: String| {
        let client = &client;
        async move {
            relayed(
                client,
                CallbackProvider::META,
                CallbackMethod::GET,
                &key,
                &query,
                &[],
                b"",
            )
            .await
        }
    };

    // The challenge goes back as plain text, for the component to pass on.
    assert_eq!(
        subscribe(key.clone(), query(&token)).await,
        Ok(RelayResponse::text(200, "1158201444"))
    );
    // Meta may percent-encode the names.
    assert_eq!(
        subscribe(key.clone(), query(&token).replace('.', "%2E")).await,
        Ok(RelayResponse::text(200, "1158201444"))
    );
    for (key, query, status) in [
        (key.clone(), query("guessed"), 403),
        (key.clone(), String::new(), 403),
        (messaging::link::new_reference(), query(&token), 403),
        ("unknown-key".to_string(), query(&token), 404),
    ] {
        assert_eq!(
            provider_status(subscribe(key.clone(), query.clone()).await),
            Ok(status),
            "{key} {query}"
        );
    }
    // Only Meta has a handshake.
    assert_eq!(
        provider_status(
            relayed(
                &client,
                CallbackProvider::VIBER,
                CallbackMethod::GET,
                &key,
                &query(&token),
                &[],
                b"",
            )
            .await
        ),
        Ok(405)
    );

    corrupt_credentials(&services, &event).await;
    assert_eq!(
        subscribe(key, query(&token)).await,
        Err(Status::InternalServerError)
    );
}

#[rocket::async_test]
async fn meta_events_are_accepted_only_with_the_app_secrets_signature() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, _) = meta_account(&client, &event).await;
    let key = webhook_key(&services, &id).await;
    let unsigned = account(&client, &event, messenger()).await;
    let unsigned_key = webhook_key(&services, &unsigned).await;

    // Meta escapes slashes and non-ASCII characters, and signs those exact
    // bytes: the relayed body must be checked as it was sent.
    let page =
        "{\"object\":\"page\",\"entry\":[],\"note\":\"a\\/b \\u00f1 \u{f1}\"}";
    let signed = hub_signature(page.as_bytes());
    assert_eq!(
        meta_post(&client, &key, Some(&signed), page.as_bytes()).await,
        Ok(200)
    );
    assert_eq!(
        meta_post(&client, &key, None, page.as_bytes()).await,
        Ok(401)
    );
    assert_eq!(
        meta_post(
            &client,
            &key,
            Some(&hub_signature(b"another body")),
            page.as_bytes()
        )
        .await,
        Ok(401)
    );
    // The same JSON, re-encoded, is not what Meta signed.
    let reencoded = page.replace("\\/", "/");
    assert_eq!(
        meta_post(&client, &key, Some(&signed), reencoded.as_bytes()).await,
        Ok(401)
    );
    // An account without an app secret cannot verify anything.
    assert_eq!(
        meta_post(&client, &unsigned_key, Some(&signed), page.as_bytes()).await,
        Ok(401)
    );
    for unknown in [messaging::link::new_reference(), "unknown-key".into()] {
        assert_eq!(
            meta_post(&client, &unknown, Some(&signed), page.as_bytes()).await,
            Ok(404),
            "{unknown}"
        );
    }
    // Regression (R1-08): a signed payload of a WhatsApp account is not
    // this Page's. Retrying it can't help (Meta would retry for hours, then
    // disable the subscription the Page's events share): it is taken and
    // ignored, as is a signed body that isn't JSON.
    let whatsapp = r#"{"object":"whatsapp_business_account","entry":[]}"#;
    for body in [whatsapp, "not json"] {
        assert_eq!(
            meta_post(
                &client,
                &key,
                Some(&hub_signature(body.as_bytes())),
                body.as_bytes()
            )
            .await,
            Ok(200),
            "{body}"
        );
    }
}

#[rocket::async_test]
async fn viber_reports_are_accepted_with_the_subscriptions_basic_auth() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(
        &client,
        &event,
        json!({"name": "Viber", "sender": {
            "provider": "VIBER_INFOBIP",
            "base_url": "https://127.0.0.1:1",
            "sender": "Elections",
        }}),
    )
    .await;
    let key = webhook_key(&services, &id).await;
    let report = r#"{"results":[{"messageId":"m-1","status":{"groupName":"DELIVERED"}}]}"#;
    let viber =
        |key: String, authorization: Option<String>, body: &'static str| {
            let client = &client;
            async move {
                let mut headers =
                    vec![("content-type", "application/json".to_string())];
                if let Some(authorization) = authorization {
                    headers.push(("authorization", authorization));
                }
                let headers: Vec<(&str, &str)> =
                    headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
                provider_status(
                    relayed(
                        client,
                        CallbackProvider::VIBER,
                        CallbackMethod::POST,
                        &key,
                        "",
                        &headers,
                        body.as_bytes(),
                    )
                    .await,
                )
            }
        };
    let basic = |user: &str, password: &str| {
        use base64::Engine;
        Some(format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD
                .encode(format!("{user}:{password}"))
        ))
    };

    // Before the report credentials are generated, nothing is accepted.
    assert_eq!(viber(key.clone(), basic("a", "b"), report).await, Ok(401));
    let (status, generated) = json(
        credentials(
            &client,
            &event,
            json!({"id": id, "credentials": {}, "generate_report_credentials": true}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{generated}");
    let (user, password) = (
        generated["report_username"].as_str().unwrap().to_string(),
        generated["report_password"].as_str().unwrap().to_string(),
    );
    assert_eq!(
        generated["replaced"],
        json!(["REPORT_USERNAME", "REPORT_PASSWORD"])
    );
    let authorization = basic(&user, &password);

    assert_eq!(
        viber(key.clone(), authorization.clone(), report).await,
        Ok(200)
    );
    for refused in [None, basic(&user, "wrong"), basic("other", &password)] {
        assert_eq!(viber(key.clone(), refused, report).await, Ok(401));
    }
    for unknown in [messaging::link::new_reference(), "unknown-key".into()] {
        assert_eq!(
            viber(unknown, authorization.clone(), report).await,
            Ok(404)
        );
    }
    // Regression (R7-24): an authenticated report that can't be read was
    // answered 500 (503 to Infobip), so Infobip retried it for ever. It is
    // logged and taken, as Meta's are.
    assert_eq!(
        viber(key.clone(), authorization.clone(), "not json").await,
        Ok(200)
    );
    // Generated, never entered.
    assert_eq!(
        refusal(
            credentials(
                &client,
                &event,
                json!({"id": id, "credentials": {"REPORT_PASSWORD": "chosen"}}),
            )
            .await
        )
        .await,
        (
            Status::BadRequest,
            "REPORT_PASSWORD is generated, not entered".to_string()
        )
    );
    // The key of another provider's account is not a Viber key.
    let (page, _) = meta_account(&client, &event).await;
    let page_key = webhook_key(&services, &page).await;
    assert_eq!(viber(page_key, authorization, report).await, Ok(404));
}

#[rocket::async_test]
async fn ses_notifications_need_the_accounts_topic() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let ses = |topic: Value| {
        json!({"name": "Mail", "sender": {
            "provider": "AWS_SES",
            "from_address": "elections@example.test",
            "notification_topic_arn": topic,
        }})
    };
    let no_topic = account(&client, &event, ses(Value::Null)).await;
    let with_topic = account(
        &client,
        &event,
        ses(json!("arn:aws:sns:eu-west-1:000000000000:ses-events")),
    )
    .await;
    // SNS posts its envelope as text/plain.
    let sns = |key: String| {
        let client = &client;
        async move {
            provider_status(
                relayed(
                    client,
                    CallbackProvider::AWS,
                    CallbackMethod::POST,
                    &key,
                    "",
                    &[
                        ("content-type", "text/plain; charset=UTF-8"),
                        ("x-amz-sns-message-type", "Notification"),
                    ],
                    b"{}",
                )
                .await,
            )
        }
    };

    for id in [no_topic, with_topic] {
        let key = webhook_key(&services, &id).await;
        assert_eq!(sns(key).await, Ok(401));
    }
    assert_eq!(sns(messaging::link::new_reference()).await, Ok(404));
}

#[rocket::async_test]
async fn the_callback_url_is_the_webhook_components() {
    // Regression (T1): the routes read MESSAGING_WEBHOOKS_PUBLIC_URL from
    // the process, not the address harvest was started with. A value in the
    // environment must change nothing.
    if !isolated("routes::messaging::route_tests::the_callback_url_is_the_webhook_components") {
        return;
    }
    std::env::set_var(
        relay::WEBHOOKS_PUBLIC_URL_ENV,
        "https://environment.example.org/",
    );
    let services = services().await;
    let unconfigured = services.client().await;
    let services =
        services.with_messaging_webhooks_url("https://callbacks.example.org");
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    async fn callback_url(
        client: &Client,
        event: &Event,
        id: String,
    ) -> (Status, Value) {
        json(callback_url_text(client, event, id).await).await
    }
    async fn upsert(
        client: &Client,
        event: &Event,
        body: Value,
    ) -> (Status, Value) {
        json(
            post(client, "/messaging/accounts/upsert", &admin(event), &body)
                .await,
        )
        .await
    }
    let (page, _) = meta_account(&client, &event).await;
    let key = webhook_key(&services, &page).await;
    let texts = account(&client, &event, sns("Texts")).await;
    let page_body =
        json!({"id": page, "name": "Page", "sender": messenger()["sender"]});

    assert_eq!(
        callback_url(&unconfigured, &event, page.clone()).await,
        (
            Status::Ok,
            json!({
                "callback_url": null,
                "reason": "Callback address not configured",
            })
        )
    );
    let (status, upserted) =
        upsert(&unconfigured, &event, page_body.clone()).await;
    assert_eq!(status, Status::Ok, "{upserted}");
    assert_eq!(
        upserted["callback_reason"],
        "Callback address not configured"
    );
    assert_eq!(upserted.get("callback_url"), None, "{upserted}");
    let (status, upserted) =
        upsert(&unconfigured, &event, sns("More texts")).await;
    assert_eq!(status, Status::Ok, "{upserted}");
    assert_eq!(upserted.get("callback_reason"), None, "{upserted}");

    assert_eq!(
        callback_url(&client, &event, page.clone()).await,
        (
            Status::Ok,
            json!({
                "callback_url": CallbackProvider::META.callback_url("https://callbacks.example.org", &key),
                "reason": null,
            })
        )
    );
    // SMS through SNS takes no callbacks.
    assert_eq!(
        callback_url(&client, &event, texts).await,
        (Status::Ok, json!({"callback_url": null, "reason": null}))
    );
    assert_eq!(
        refusal(
            callback_url_text(&client, &event, Uuid::new_v4().to_string())
                .await
        )
        .await,
        (Status::NotFound, "unknown account".to_string())
    );
    let (status, upserted) = upsert(&client, &event, page_body).await;
    assert_eq!(status, Status::Ok, "{upserted}");
    assert_eq!(upserted.get("callback_reason"), None, "{upserted}");
    assert_eq!(
        upserted["callback_url"],
        CallbackProvider::META
            .callback_url("https://callbacks.example.org", &key)
    );
}

async fn callback_url_text<'c>(
    client: &'c Client,
    event: &Event,
    id: String,
) -> LocalResponse<'c> {
    post(
        client,
        "/messaging/accounts/callback-url",
        &admin(event),
        &json!({ "id": id }),
    )
    .await
}

#[test]
fn the_meta_handshake_query_is_url_decoded() {
    assert_eq!(
        MetaHub::parse(
            "hub.mode=subscribe&hub%2Everify_token=a%2Bb+c&hub.challenge=12&x=1&&novalue"
        ),
        MetaHub {
            mode: Some("subscribe".into()),
            verify_token: Some("a+b c".into()),
            challenge: Some("12".into()),
        }
    );
    assert_eq!(MetaHub::parse(""), MetaHub::default());
}

#[test]
fn without_a_relay_secret_callbacks_are_not_configured() {
    for secret in [None, Some(&b""[..])] {
        assert!(matches!(
            MessagingRelay::new(secret, 1024).secret,
            RelaySecret::NotConfigured(_)
        ));
    }
    // Regression (R1-14): any non-blank secret was taken, the committed
    // development one too; harvest checks it as the component does.
    let vars = |secret: &'static str, policy: Option<&'static str>| {
        move |name: &str| match name {
            "MESSAGING_RELAY_SECRET" => Some(secret.to_string()),
            "MESSAGING_RELAY_SECRET_POLICY" => policy.map(str::to_string),
            _ => None,
        }
    };
    for (secret, policy, refusal) in [
        ("s3cret", None, relay::RelaySecretRefusal::TOO_SHORT),
        (
            relay::DEV_RELAY_SECRET,
            None,
            relay::RelaySecretRefusal::DEVELOPMENT_VALUE,
        ),
    ] {
        assert!(matches!(
            MessagingRelay::from_vars(vars(secret, policy)).secret,
            RelaySecret::NotConfigured(r) if r == refusal
        ));
    }
    assert!(matches!(
        MessagingRelay::from_vars(vars(
            relay::DEV_RELAY_SECRET,
            Some("ALLOW_DEV_SECRET")
        ))
        .secret,
        RelaySecret::Configured(_)
    ));
    assert!(matches!(
        MessagingRelay::from_vars(vars(
            "0123456789abcdef0123456789abcdef",
            None
        ))
        .secret,
        RelaySecret::Configured(_)
    ));
    let relay = MessagingRelay::new(Some(b"secret"), 1024);
    assert!(
        matches!(relay.secret, RelaySecret::Configured(ref s) if s == b"secret")
    );
    assert_eq!(relay.envelope_limit, relay::envelope_limit(1024));
}

#[rocket::async_test]
async fn an_invalid_event_configuration_is_reported_and_not_stored() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let unchecked = account(&client, &event, texts("Texts")).await;
    let unknown = Uuid::new_v4().to_string();
    let save = |election_event_id: String, config: Value| {
        let (client, claims) = (&client, admin(&event));
        async move {
            post(
                client,
                "/messaging/event-config",
                &claims,
                &json!({
                    "election_event_id": election_event_id,
                    "config": config,
                }),
            )
            .await
        }
    };
    let config = json!({
        "version": 2,
        "channels": [
            {"channel": "SMS", "account_id": unchecked, "purposes": ["NOTICE"]},
            {"channel": "EMAIL", "account_id": unknown},
        ],
    });

    assert_eq!(
        json(save(event.election_event_id.clone(), config.clone()).await).await,
        (
            Status::Ok,
            json!({"errors": [
                {"kind": "UNSUPPORTED_VERSION", "version": 2},
                {
                    "kind": "PURPOSE_NOT_READY",
                    "channel": "SMS",
                    "purpose": "NOTICE",
                    "blockers": ["NOT_CONNECTED", "NEEDS_PRODUCTION_ACCESS"],
                },
                {
                    "kind": "UNKNOWN_ACCOUNT",
                    "channel": "EMAIL",
                    "account_id": unknown,
                },
            ]})
        )
    );
    assert_eq!(stored_config(&services, &event).await, None);

    assert_eq!(
        text(save(Uuid::new_v4().to_string(), config.clone()).await).await,
        (Status::NotFound, "unknown election event".to_string())
    );
    assert_eq!(
        text(save("not-a-uuid".to_string(), config.clone()).await).await,
        (Status::BadRequest, "invalid identifier".to_string())
    );

    // An account row that no longer reads fails the save.
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.messaging_account SET sender = '{}'
         WHERE id = $1",
        &[&uuid(&unchecked)],
    )
    .await;
    assert_eq!(
        text(save(event.election_event_id.clone(), config).await).await,
        (Status::InternalServerError, "internal error".to_string())
    );
}

// Core's Keycloak token cache and KEYCLOAK_URL are global, and so is the
// schema of a test binary's database. Run only this test in a fresh process,
// with a database of its own.
fn isolated(test: &str) -> bool {
    if std::env::var(CHILD).as_deref() == Ok(test) {
        return true;
    }
    let log = tempfile::NamedTempFile::new().unwrap();
    let output = log.reopen().unwrap();
    let mut child =
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(CHILD, test)
            .env("KEYCLOAK_ADMIN_CLIENT_ID", "synthetic-admin")
            .env("KEYCLOAK_ADMIN_CLIENT_SECRET", "synthetic-secret")
            .env("SUPER_ADMIN_TENANT_ID", "fixture-super-admin")
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap();
    let deadline = Instant::now() + Duration::from_secs(45);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("messaging route test timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(log.path()).unwrap()
    );
    false
}

#[rocket::async_test]
async fn a_saved_event_configuration_is_stored_only_once_its_realm_publishes_it(
) {
    if !isolated("routes::messaging::route_tests::a_saved_event_configuration_is_stored_only_once_its_realm_publishes_it") {
        return;
    }
    let services = services().await;
    let event = rows::event(&services.hasura).await;
    let realm = format!("/admin/realms/{}", event.realm());
    let current =
        json!({"realm": event.realm(), "attributes": {"kept": "yes"}});
    let peer = http::HttpServer::start(vec![
        http::Exchange::json(
            "POST",
            "/realms/master/protocol/openid-connect/token",
            200,
            http::token_json(),
        ),
        http::Exchange::json("GET", &realm, 200, current.clone()),
        http::Exchange::json("PUT", &realm, 500, json!({"error": "down"})),
        http::Exchange::json("GET", &realm, 200, current),
        http::Exchange::json("PUT", &realm, 204, json!({})),
    ]);
    std::env::set_var("KEYCLOAK_URL", &peer.url);
    let client = services.client().await;
    let id = account(&client, &event, texts("Texts")).await;
    checked(&services, &id).await;
    let config = json!({
        "version": 1,
        "channels": [
            {"channel": "SMS", "account_id": id, "purposes": ["NOTICE"]},
        ],
    });
    let body = json!({
        "election_event_id": event.election_event_id,
        "config": config,
    });

    // The realm refuses the first publication: nothing is stored.
    assert_eq!(
        text(
            post(&client, "/messaging/event-config", &admin(&event), &body)
                .await
        )
        .await,
        (Status::InternalServerError, "internal error".to_string())
    );
    assert_eq!(stored_config(&services, &event).await, None);

    assert_eq!(
        json(
            post(&client, "/messaging/event-config", &admin(&event), &body)
                .await
        )
        .await,
        (Status::Ok, json!({"errors": []}))
    );
    let stored = stored_config(&services, &event).await.unwrap();
    assert_eq!(stored["channels"][0]["account_id"], id);
    // Regression (R1-15): the save names its author.
    let updated_by: Option<String> = rows::query(
        &services.hasura,
        "SELECT updated_by FROM sequent_backend.messaging_event_config
         WHERE election_event_id = $1",
        &[&uuid(&event.election_event_id)],
    )
    .await[0]
        .get(0);
    assert_eq!(updated_by.as_deref(), Some(USER_ID));

    let requests = peer.finish();
    let published = requests.last().unwrap().json();
    assert_eq!(published["attributes"]["kept"], "yes");
    let projection: Value = serde_json::from_str(
        published["attributes"][REALM_ATTR_MESSAGING]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    // Voters' pages see the channel and its purposes, never the account.
    assert_eq!(
        projection["channels"],
        json!([{
            "channel": "SMS",
            "purposes": ["NOTICE"],
            "sender_label": "Elections",
            "messenger_page": null,
        }])
    );
    assert!(!projection.to_string().contains(&id));
}

#[rocket::async_test]
async fn changes_whose_commit_fails_are_not_reported_as_done() {
    if !isolated("routes::messaging::route_tests::changes_whose_commit_fails_are_not_reported_as_done") {
        return;
    }
    let services = services().await;
    let event = rows::event(&services.hasura).await;
    let realm = format!("/admin/realms/{}", event.realm());
    let peer = http::HttpServer::start(vec![
        http::Exchange::json(
            "POST",
            "/realms/master/protocol/openid-connect/token",
            200,
            http::token_json(),
        ),
        http::Exchange::json(
            "GET",
            &realm,
            200,
            json!({"realm": event.realm()}),
        ),
        http::Exchange::json("PUT", &realm, 204, json!({})),
    ]);
    std::env::set_var("KEYCLOAK_URL", &peer.url);
    // Set before the first request: the dispatcher reads them once. The
    // replaced token's check connects, so the save reaches its commit.
    let _graph = page_found().await;
    let _sns = sns_answering(vec![in_production()]).await;
    let client = services.client().await;
    let (id, _) = meta_account(&client, &event).await;
    let texts = texts_account(&client, &event).await;
    let before = stored_account(&services, &id).await;
    // This process has the database to itself: from here on, every commit
    // that touched an account or an event fails.
    for statement in [
        "CREATE FUNCTION sequent_backend.refuse_commit() RETURNS trigger
         LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
        "CREATE CONSTRAINT TRIGGER refuse_commit
         AFTER INSERT OR UPDATE OR DELETE
         ON sequent_backend.messaging_account
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
         EXECUTE FUNCTION sequent_backend.refuse_commit()",
        "CREATE CONSTRAINT TRIGGER refuse_commit
         AFTER UPDATE ON sequent_backend.election_event
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
         EXECUTE FUNCTION sequent_backend.refuse_commit()",
        "CREATE CONSTRAINT TRIGGER refuse_commit
         AFTER INSERT OR UPDATE ON sequent_backend.messaging_event_config
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
         EXECUTE FUNCTION sequent_backend.refuse_commit()",
    ] {
        rows::execute(&services.hasura, statement, &[]).await;
    }

    for (path, body) in [
        ("/messaging/accounts/upsert", sns("Texts")),
        (
            "/messaging/accounts/credentials",
            json!({"id": id, "credentials": {"ACCESS_TOKEN": "rotated"}}),
        ),
        ("/messaging/accounts/check", json!({ "id": texts })),
        ("/messaging/accounts/delete", json!({ "id": id })),
        (
            "/messaging/event-config",
            json!({
                "election_event_id": event.election_event_id,
                "config": {"version": 1},
            }),
        ),
    ] {
        assert_eq!(
            refusal(post(&client, path, &admin(&event), &body).await).await,
            (Status::InternalServerError, "internal error".to_string()),
            "{path}"
        );
    }
    assert_eq!(stored_account(&services, &id).await, before);
    let accounts: i64 = rows::query(
        &services.hasura,
        "SELECT count(*) FROM sequent_backend.messaging_account
         WHERE tenant_id = $1 AND status = '{}'",
        &[&uuid(&event.tenant_id)],
    )
    .await[0]
        .get(0);
    assert_eq!(accounts, 2);
    assert_eq!(stored_config(&services, &event).await, None);
    // The realm had already taken the publication when the commit failed.
    assert_eq!(peer.finish().len(), 3);
}

#[rocket::async_test]
async fn messaging_routes_answer_without_a_database() {
    let services = Services::without_database();
    let client = services.client().await;
    let event = Event {
        tenant_id: Uuid::new_v4().to_string(),
        election_event_id: Uuid::new_v4().to_string(),
    };
    let id = Uuid::new_v4().to_string();
    let service = service_token(&services, &event.tenant_id);
    let reference =
        link_request(&event.tenant_id, "reference", "session-digest");

    for (path, body) in [
        ("/messages/send", notice(&event, "k")),
        ("/messages/link", link(&event, chrono::Duration::minutes(5))),
        ("/messages/link/status", reference.clone()),
        ("/messages/link/confirm", reference),
    ] {
        assert_eq!(
            text(post_as(&client, path, &service, &body).await).await,
            (Status::InternalServerError, "internal error".to_string()),
            "{path}"
        );
    }
    for (path, body) in [
        ("/messaging/accounts/upsert", sns("Texts")),
        ("/messaging/accounts/delete", json!({ "id": id })),
        (
            "/messaging/accounts/credentials",
            json!({"id": id, "credentials": {"API_KEY": "x"}}),
        ),
        ("/messaging/accounts/check", json!({ "id": id })),
        ("/messaging/accounts/callback-url", json!({ "id": id })),
        (
            "/messaging/accounts/test",
            json!({"id": id, "purpose": "NOTICE", "destination": PHONE}),
        ),
        (
            "/messaging/event-config",
            json!({
                "election_event_id": event.election_event_id,
                "config": {"version": 1},
            }),
        ),
    ] {
        assert_eq!(
            refusal(post(&client, path, &admin(&event), &body).await).await,
            (Status::InternalServerError, "internal error".to_string()),
            "{path}"
        );
    }

    // A callback that was not taken gets 503 from the component, and the
    // provider retries it.
    let key = messaging::link::new_reference();
    for (provider, method) in [
        (CallbackProvider::META, CallbackMethod::GET),
        (CallbackProvider::META, CallbackMethod::POST),
        (CallbackProvider::VIBER, CallbackMethod::POST),
        (CallbackProvider::AWS, CallbackMethod::POST),
    ] {
        assert_eq!(
            relayed(&client, provider, method, &key, "", &[], b"{}").await,
            Err(Status::ServiceUnavailable),
            "{provider} {method}"
        );
    }
}

/// The reason of a message an account that isn't ready refused at send
/// time (`windmill::services::messaging::accounts::not_ready_reason`).
const NOT_CONNECTED_REASON: &str =
    "Not sent: the account isn't ready (NOT_CONNECTED)";
/// `windmill::postgres::rate_bucket::CAPACITY_BUSY`.
const CAPACITY_BUSY: &str =
    "Not sent: the account stayed at its sending limit for too long";

/// Sets the account's sending limits and freezes its bucket at `tokens`
/// for an hour: nothing refills it during the test.
async fn saturated(
    services: &Services,
    account_id: &str,
    limits: Value,
    tokens: f64,
) {
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.messaging_account SET limits = $2
         WHERE id = $1",
        &[&uuid(account_id), &limits],
    )
    .await;
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.messaging_rate_bucket
             (account_id, tokens, refilled_at)
         VALUES ($1, $2, now() + interval '1 hour')
         ON CONFLICT (account_id) DO UPDATE
         SET tokens = EXCLUDED.tokens, refilled_at = EXCLUDED.refilled_at",
        &[&uuid(account_id), &tokens],
    )
    .await;
}

fn code(event: &Event, logical_key: &str) -> Value {
    let mut code = notice(event, logical_key);
    code["purpose"] = json!("OTP");
    code["election_event_id"] = json!(event.election_event_id);
    code["content"]["code"] = json!("482619");
    code["expires_at"] = json!("2099-01-01T00:00:00+02:00");
    code
}

/// R5-01: Keycloak's codes and notices through an account that isn't ready
/// are FAILED with the readiness reason and never reach the provider.
#[rocket::async_test]
async fn keycloak_sends_nothing_through_an_account_that_isnt_ready() {
    if !isolated(
        "routes::messaging::route_tests::keycloak_sends_nothing_through_an_account_that_isnt_ready",
    ) {
        return;
    }
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let sns = sns_answering(vec![published("sns-1")]).await;
    // Never checked: not connected.
    let id = texts_account(&client, &event).await;

    // A code's failure is never retried by its origin; a Keycloak notice's
    // says why next to it (its content isn't stored).
    for (body, reason) in [
        (
            notice(&event, "not-ready-notice"),
            format!(
                "{NOT_CONNECTED_REASON}; Can't be retried: no stored template"
            ),
        ),
        (
            code(&event, "not-ready-code"),
            NOT_CONNECTED_REASON.to_string(),
        ),
    ] {
        let (status, refused) =
            json(send(&client, &services, &event.tenant_id, &body).await).await;
        assert_eq!(status, Status::Ok, "{refused}");
        assert_eq!(refused["state"], "FAILED", "{refused}");
        assert_eq!(refused["reason"], reason, "{refused}");
    }
    assert!(sns.requests().is_empty());

    // Once a check passes, the account sends.
    checked(&services, &id).await;
    let (_, sent) = json(
        send(
            &client,
            &services,
            &event.tenant_id,
            &notice(&event, "ready"),
        )
        .await,
    )
    .await;
    assert_eq!(sent["state"], "ACCEPTED", "{sent}");
    assert_eq!(sns.requests().len(), 1);
}

/// R5-02: under saturation, Keycloak hears FAILED with the capacity reason
/// well before its own 10 s timeout, and nothing reaches the provider.
#[rocket::async_test]
async fn a_saturated_account_answers_keycloak_before_its_timeout() {
    if !isolated(
        "routes::messaging::route_tests::a_saturated_account_answers_keycloak_before_its_timeout",
    ) {
        return;
    }
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let sns = sns_answering(vec![published("sns-1")]).await;
    let id = texts_account(&client, &event).await;
    checked(&services, &id).await;
    saturated(
        &services,
        &id,
        json!({"messages_per_second": 1, "otp_reserved_per_second": 0}),
        0.0,
    )
    .await;

    let asked = Instant::now();
    let (status, busy) = json(
        send(&client, &services, &event.tenant_id, &code(&event, "busy")).await,
    )
    .await;
    assert!(
        asked.elapsed() < Duration::from_secs(8),
        "{:?}",
        asked.elapsed()
    );
    assert_eq!(status, Status::Ok, "{busy}");
    assert_eq!(busy["state"], "FAILED", "{busy}");
    assert_eq!(busy["reason"], CAPACITY_BUSY, "{busy}");
    assert!(sns.requests().is_empty());
}

/// R5-02: a notice Keycloak sends (a reset link, a temporary password) is
/// interactive: it takes from the whole bucket, the OTP reserve included.
#[rocket::async_test]
async fn keycloak_notices_take_from_the_whole_bucket() {
    if !isolated(
        "routes::messaging::route_tests::keycloak_notices_take_from_the_whole_bucket",
    ) {
        return;
    }
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let sns = sns_answering(vec![published("sns-1")]).await;
    let id = texts_account(&client, &event).await;
    checked(&services, &id).await;
    // One token left, all of it in the OTP reserve.
    saturated(
        &services,
        &id,
        json!({"messages_per_second": 10, "otp_reserved_per_second": 9}),
        1.0,
    )
    .await;

    let asked = Instant::now();
    let (_, sent) = json(
        send(
            &client,
            &services,
            &event.tenant_id,
            &notice(&event, "reset"),
        )
        .await,
    )
    .await;
    assert_eq!(sent["state"], "ACCEPTED", "{sent}");
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "{:?}",
        asked.elapsed()
    );
    assert_eq!(sns.requests().len(), 1);
}

/// R5-02: a provider that doesn't answer in time leaves the message
/// UNKNOWN (it may have been sent), answered before Keycloak's timeout.
#[rocket::async_test]
async fn a_provider_that_answers_too_late_leaves_the_message_unknown() {
    if !isolated(
        "routes::messaging::route_tests::a_provider_that_answers_too_late_leaves_the_message_unknown",
    ) {
        return;
    }
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let sns = TestServer::start_with(
        vec![published("sns-1")],
        messaging::test_server::ServerOptions {
            latency: Duration::from_secs(12),
            ..Default::default()
        },
    )
    .await;
    std::env::set_var(AWS_ENDPOINT_URL_ENV, &sns.base_url);
    let id = texts_account(&client, &event).await;
    checked(&services, &id).await;

    let asked = Instant::now();
    let (status, late) = json(
        send(&client, &services, &event.tenant_id, &code(&event, "late")).await,
    )
    .await;
    assert!(
        asked.elapsed() < Duration::from_millis(9500),
        "{:?}",
        asked.elapsed()
    );
    assert_eq!(status, Status::Ok, "{late}");
    assert_eq!(late["state"], "UNKNOWN", "{late}");
    assert_eq!(
        late["reason"],
        "The provider didn't answer in time: the message may have been sent",
        "{late}"
    );
}

/// R5-07: Meta's callback is answered once its events are stored and the
/// link bound; the code it releases is sent afterwards, with its own place,
/// bucket and budget, and its outcome is in the ledger.
#[rocket::async_test]
async fn a_get_started_is_answered_before_the_code_it_releases_is_sent() {
    if !isolated(
        "routes::messaging::route_tests::a_get_started_is_answered_before_the_code_it_releases_is_sent",
    ) {
        return;
    }
    let graph = TestServer::start(vec![Reply::Json(
        200,
        json!({"message_id": "m_code"}),
    )])
    .await;
    std::env::set_var(META_GRAPH_URL_ENV, &graph.base_url);
    let services = services()
        .await
        .with_messaging_webhooks_url("https://callbacks.example.org");
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, _) = meta_account(&client, &event).await;
    checked(&services, &id).await;
    configure(
        &services,
        &event,
        json!({"version": 1, "channels": [{
            "channel": "MESSENGER", "account_id": id, "purposes": ["OTP"],
        }]}),
    )
    .await;
    saturated(
        &services,
        &id,
        json!({"messages_per_second": 1, "otp_reserved_per_second": 0}),
        0.0,
    )
    .await;
    let service = service_token(&services, &event.tenant_id);
    let (status, created) = json(
        post_as(
            &client,
            "/messages/link",
            &service,
            &link(&event, chrono::Duration::minutes(5)),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{created}");
    let reference = created["reference"].as_str().unwrap();
    let body =
        json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
            {"sender": {"id": "6543210"}, "recipient": {"id": "page-1"},
             "timestamp": chrono::Utc::now().timestamp_millis(),
             "postback": {"title": "Get Started", "payload": "GET_STARTED",
                          "referral": {"ref": reference, "source": "SHORTLINK",
                                       "type": "OPEN_THREAD"}}}
        ]}]})
        .to_string();
    let key = webhook_key(&services, &id).await;

    let asked = Instant::now();
    assert_eq!(
        meta_post(
            &client,
            &key,
            Some(&hub_signature(body.as_bytes())),
            body.as_bytes()
        )
        .await,
        Ok(200)
    );
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "{:?}",
        asked.elapsed()
    );
    let bound: Option<String> = rows::query(
        &services.hasura,
        "SELECT page_scoped_id FROM sequent_backend.messenger_link
         WHERE tenant_id = $1",
        &[&uuid(&event.tenant_id)],
    )
    .await[0]
        .get(0);
    assert_eq!(bound.as_deref(), Some("6543210"));

    // The code waited for capacity after the answer, then was recorded.
    let attempts = loop {
        let attempts: Vec<(String, Option<String>)> = rows::query(
            &services.hasura,
            "SELECT state, error FROM sequent_backend.message
             WHERE tenant_id = $1 AND logical_key LIKE 'messenger-link:%'",
            &[&uuid(&event.tenant_id)],
        )
        .await
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
        if !attempts.is_empty() || asked.elapsed() > Duration::from_secs(15) {
            break attempts;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(
        attempts,
        vec![("FAILED".to_string(), Some(CAPACITY_BUSY.to_string()))]
    );
    assert!(graph.requests().is_empty());
}

/// Regression (R7-03): every relayed callback took a connection from the
/// pool Keycloak's codes use, so a flood of well-formed keys could starve
/// the codes. Callbacks hold at most their own share; one that gets none
/// in time is answered 503 (the component answers the provider 503, and it
/// retries) without reaching the database.
#[rocket::async_test]
async fn callbacks_hold_only_their_own_share_of_the_database() {
    let relay =
        MessagingRelay::new(Some(RELAY_SECRET), relay::DEFAULT_MAX_BODY_BYTES)
            .with_callback_share(1, Duration::from_millis(200));
    let services = services().await.with_messaging_relay(relay.clone());
    let client = services.client().await;
    let key = messaging::link::new_reference();
    // The share is free: the unknown key is looked up, 404 for Meta.
    assert_eq!(
        meta_post(&client, &key, Some("sha256=00"), b"{}").await,
        Ok(404)
    );
    // Callbacks in flight hold the whole share.
    let held = relay.callbacks.clone().acquire_owned().await.unwrap();
    let started = Instant::now();
    assert_eq!(
        meta_post(&client, &key, Some("sha256=00"), b"{}").await,
        Err(Status::ServiceUnavailable)
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    drop(held);
    assert_eq!(
        meta_post(&client, &key, Some("sha256=00"), b"{}").await,
        Ok(404)
    );
}
