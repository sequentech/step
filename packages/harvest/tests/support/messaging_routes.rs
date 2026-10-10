// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging routes on the migrated test database: sending accounts and
//! their credentials, Keycloak's sending and Messenger link calls, provider
//! webhooks and the event configuration. Messages leave through the console
//! provider, so nothing reaches a network.

use crate::route_services::rows::{self, Event};
use crate::route_services::{bearer, http, json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::messaging::REALM_ATTR_MESSAGING;
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
}

fn uuid(id: &str) -> Uuid {
    Uuid::parse_str(id).expect("fixture ids are UUIDs")
}

fn console(channel: &str) -> Value {
    json!({
        "channel": channel,
        "name": format!("{channel} console"),
        "sender": {"provider": "CONSOLE"},
        "is_default": true,
    })
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
        "readiness": "ADMIN_CONFIRMED",
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
/// publishing it to a realm.
async fn configure(services: &Services, event: &Event, config: Value) {
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.election_event
         SET annotations = jsonb_build_object('messaging:config', $2::text)
         WHERE id = $1",
        &[&uuid(&event.election_event_id), &config.to_string()],
    )
    .await;
}

async fn stored_config(services: &Services, event: &Event) -> Option<Value> {
    let raw: Option<String> = rows::query(
        &services.hasura,
        "SELECT annotations ->> 'messaging:config'
         FROM sequent_backend.election_event WHERE id = $1",
        &[&uuid(&event.election_event_id)],
    )
    .await[0]
        .get(0);
    raw.map(|raw| serde_json::from_str(&raw).expect("a stored configuration"))
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
    tenant_id: &str,
    body: &Value,
) -> LocalResponse<'c> {
    post(client, "/messages/send", &keycloak(tenant_id), body).await
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
fn signature(body: &str) -> Header<'static> {
    let key = openssl::pkey::PKey::hmac(APP_SECRET.as_bytes()).unwrap();
    let mut signer = openssl::sign::Signer::new(
        openssl::hash::MessageDigest::sha256(),
        &key,
    )
    .unwrap();
    let mac = signer.sign_oneshot_to_vec(body.as_bytes()).unwrap();
    Header::new(
        "X-Hub-Signature-256",
        format!("sha256={}", hex::encode(mac)),
    )
}

async fn callback(client: &Client, path: String, body: &str) -> Status {
    client.post(path).body(body).dispatch().await.status()
}

#[rocket::async_test]
async fn accounts_take_their_providers_channel_and_one_default_per_channel() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    // Amazon SNS only sends SMS, whatever channel the request names.
    let mut texts = sns("Texts");
    texts["channel"] = json!("EMAIL");
    texts["is_default"] = json!(true);
    let id = account(&client, &event, texts).await;
    assert_eq!(
        stored_account(&services, &id).await,
        ("SMS".into(), "Texts".into(), true)
    );

    let other = account(&client, &event, console("SMS")).await;
    assert_eq!(
        stored_account(&services, &other).await,
        ("SMS".into(), "SMS console".into(), true)
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
            with_id(console("SMS"), &id),
            Status::BadRequest,
            "the provider and channel of an account cannot change; add another account",
        ),
    ] {
        assert_eq!(
            text(
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
        text(delete(id).await).await,
        (Status::NotFound, "unknown account".to_string())
    );
    assert_eq!(
        text(delete("not-a-uuid".to_string()).await).await,
        (Status::BadRequest, "invalid identifier".to_string())
    );
}

#[rocket::async_test]
async fn replaced_credentials_are_named_and_the_verify_token_shown_once() {
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
        json!({"replaced": ["ACCESS_TOKEN"], "verify_token": null})
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

#[rocket::async_test]
async fn credentials_an_account_cannot_use_are_refused() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(&client, &event, console("SMS")).await;

    for (body, status, message) in [
        (
            json!({"id": id, "credentials": {"NOPE": "x"}}),
            Status::BadRequest,
            "unknown credential NOPE",
        ),
        (
            json!({"id": id, "credentials": {"API_KEY": "x"}}),
            Status::BadRequest,
            "CONSOLE accounts do not use API_KEY",
        ),
        (
            json!({"id": id, "credentials": {}, "generate_verify_token": true}),
            Status::BadRequest,
            "CONSOLE accounts have no verify token",
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
            text(credentials(&client, &event, body.clone()).await).await,
            (status, message.to_string()),
            "{body}"
        );
    }
}

#[rocket::async_test]
async fn an_account_check_stores_what_the_provider_answered() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(&client, &event, console("SMS")).await;
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

    assert_eq!(
        text(check(Uuid::new_v4().to_string()).await).await,
        (Status::NotFound, "unknown account".to_string())
    );
    assert_eq!(
        text(check("not-a-uuid".to_string()).await).await,
        (Status::BadRequest, "invalid identifier".to_string())
    );

    // Credentials that cannot be read fail the check before any request.
    let (page, _) = meta_account(&client, &event).await;
    corrupt_credentials(&services, &event).await;
    assert_eq!(
        text(check(page).await).await,
        (Status::InternalServerError, "internal error".to_string())
    );
}

#[rocket::async_test]
async fn a_test_message_goes_through_the_chosen_account() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(&client, &event, console("SMS")).await;
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
                "+34*****0001".into()
            ),
            ("OTP".into(), "account-test".into(), "+34*****0001".into()),
        ]
    );

    let (status, refused) = json(
        test(json!({"id": id, "purpose": "NOTICE", "destination": "nobody"}))
            .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{refused}");
    assert_eq!(refused["state"], "FAILED", "{refused}");
    assert!(refused["reason"].is_string(), "{refused}");

    assert_eq!(
        text(
            test(json!({
                "id": Uuid::new_v4(), "purpose": "NOTICE", "destination": PHONE,
            }))
            .await
        )
        .await,
        (Status::NotFound, "unknown account".to_string())
    );
    assert_eq!(
        text(
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
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    account(&client, &event, console("SMS")).await;

    let (status, first) = json(
        send(&client, &event.tenant_id, &notice(&event, "enrolled")).await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{first}");
    assert_eq!(first["state"], "ACCEPTED", "{first}");
    let (_, replay) = json(
        send(&client, &event.tenant_id, &notice(&event, "enrolled")).await,
    )
    .await;
    assert_eq!(replay, first);

    let mut code = notice(&event, "code-1");
    code["purpose"] = json!("OTP");
    code["election_event_id"] = json!(event.election_event_id);
    code["content"]["code"] = json!("482619");
    code["expires_at"] = json!("2099-01-01T00:00:00+02:00");
    let (status, sent) =
        json(send(&client, &event.tenant_id, &code).await).await;
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
            text(send(&client, &event.tenant_id, &body).await).await,
            (Status::BadRequest, message.to_string()),
            "{body}"
        );
    }
    assert_eq!(
        text(
            send(
                &client,
                "not-a-uuid",
                &edited("tenant_id", json!("not-a-uuid"))
            )
            .await
        )
        .await,
        (Status::BadRequest, "invalid identifier".to_string())
    );

    // Without an event configuration only email and SMS are offered.
    assert_eq!(
        json(
            send(
                &client,
                &event.tenant_id,
                &edited("channel", json!("VIBER"))
            )
            .await
        )
        .await,
        (
            Status::Ok,
            json!({
                "message_id": "",
                "state": "FAILED",
                "reason": "no eligible channel",
            })
        )
    );

    configure(
        &services,
        &event,
        json!({"version": 1, "channels": [{
            "channel": "SMS", "account_id": "not-a-uuid", "purposes": ["NOTICE"],
        }]}),
    )
    .await;
    assert_eq!(
        text(
            send(
                &client,
                &event.tenant_id,
                &edited("election_event_id", json!(event.election_event_id)),
            )
            .await
        )
        .await,
        (Status::InternalServerError, "internal error".to_string())
    );
}

#[rocket::async_test]
async fn a_messenger_link_is_confirmed_only_in_its_session_once_the_code_was_sent(
) {
    let services = services().await;
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
    let service = keycloak(&event.tenant_id);

    let (status, created) = json(
        post(
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
    assert_eq!(created["link_word"].as_str().unwrap().len(), 9);

    let own = link_request(&event.tenant_id, reference, "session-digest");
    let pending = json!({
        "state": "PENDING", "page_scoped_id": null, "page_id": null,
    });
    for path in ["/messages/link/status", "/messages/link/confirm"] {
        assert_eq!(
            json(post(&client, path, &service, &own).await).await,
            (Status::Ok, pending.clone()),
            "{path}"
        );
        for other in [
            link_request(&event.tenant_id, reference, "another-session"),
            link_request(&event.tenant_id, "unknown", "session-digest"),
        ] {
            assert_eq!(
                text(post(&client, path, &service, &other).await).await,
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
        json(post(&client, "/messages/link/confirm", &service, &own).await)
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
        json(post(&client, "/messages/link/status", &service, &own).await)
            .await;
    assert_eq!(status["state"], "CONFIRMED");
}

#[rocket::async_test]
async fn messenger_links_need_an_event_that_offers_messenger_for_codes() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let service = keycloak(&event.tenant_id);

    assert_eq!(
        text(
            post(
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
            post(
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

    // A tenant that is no UUID has no link key to look references up with.
    let stranger = keycloak("not-a-uuid");
    for path in ["/messages/link/status", "/messages/link/confirm"] {
        assert_eq!(
            text(
                post(
                    &client,
                    path,
                    &stranger,
                    &link_request("not-a-uuid", "reference", "session-digest"),
                )
                .await
            )
            .await,
            (Status::InternalServerError, "internal error".to_string()),
            "{path}"
        );
    }
}

#[rocket::async_test]
async fn the_meta_subscription_echoes_the_challenge_for_the_verify_token() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (id, token) = meta_account(&client, &event).await;
    let key = webhook_key(&services, &id).await;
    let subscribe = |key: &str, token: &str| {
        let uri = format!(
            "/webhooks/meta/{key}?hub.mode=subscribe&hub.verify_token={token}&hub.challenge=1158201444"
        );
        let client = &client;
        async move {
            let response = client.get(uri).dispatch().await;
            (response.status(), response.into_string().await)
        }
    };

    assert_eq!(
        subscribe(&key, &token).await,
        (Status::Ok, Some("1158201444".to_string()))
    );
    assert_eq!(subscribe(&key, "guessed").await.0, Status::Forbidden);
    assert_eq!(subscribe("unknown-key", &token).await.0, Status::Forbidden);
    assert_eq!(
        client
            .get(format!("/webhooks/meta/{key}"))
            .dispatch()
            .await
            .status(),
        Status::Forbidden
    );

    corrupt_credentials(&services, &event).await;
    assert_eq!(subscribe(&key, &token).await.0, Status::InternalServerError);
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
    let deliver = |key: &str, body: &'static str, signed: Option<&str>| {
        let mut request = client.post(format!("/webhooks/meta/{key}"));
        if let Some(signed) = signed {
            request = request.header(signature(signed));
        }
        async move { request.body(body).dispatch().await.status() }
    };

    let page = r#"{"object":"page","entry":[]}"#;
    assert_eq!(deliver(&key, page, Some(page)).await, Status::Ok);
    assert_eq!(deliver(&key, page, None).await, Status::Unauthorized);
    assert_eq!(
        deliver(&key, page, Some("another body")).await,
        Status::Unauthorized
    );
    // An account without an app secret cannot verify anything.
    assert_eq!(
        deliver(&unsigned_key, page, Some(page)).await,
        Status::Unauthorized
    );
    assert_eq!(
        deliver("unknown-key", page, Some(page)).await,
        Status::NotFound
    );
    // A signed payload of a WhatsApp account is not this Page's.
    let whatsapp = r#"{"object":"whatsapp_business_account","entry":[]}"#;
    assert_eq!(
        deliver(&key, whatsapp, Some(whatsapp)).await,
        Status::InternalServerError
    );
}

#[rocket::async_test]
async fn viber_reports_are_accepted_on_the_accounts_webhook_key() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(
        &client,
        &event,
        json!({"name": "Viber", "sender": {
            "provider": "VIBER_INFOBIP",
            "base_url": "http://127.0.0.1:1",
            "sender": "Elections",
        }}),
    )
    .await;
    let key = webhook_key(&services, &id).await;
    let report = r#"{"results":[{"messageId":"m-1","status":{"groupName":"DELIVERED"}}]}"#;

    assert_eq!(
        callback(&client, format!("/webhooks/viber/{key}"), report).await,
        Status::Ok
    );
    assert_eq!(
        callback(&client, "/webhooks/viber/unknown-key".into(), report).await,
        Status::NotFound
    );
    assert_eq!(
        callback(&client, format!("/webhooks/viber/{key}"), "not json").await,
        Status::InternalServerError
    );
    // The key of another provider's account is not a Viber key.
    let (page, _) = meta_account(&client, &event).await;
    let page_key = webhook_key(&services, &page).await;
    assert_eq!(
        callback(&client, format!("/webhooks/viber/{page_key}"), report).await,
        Status::NotFound
    );
}

#[rocket::async_test]
async fn configured_providers_report_with_a_body_or_with_query_parameters() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let id = account(&client, &event, http_api(true)).await;
    let key = webhook_key(&services, &id).await;
    let silent = account(&client, &event, http_api(false)).await;
    let silent_key = webhook_key(&services, &silent).await;
    let report = r#"{"id":"m-1","status":"delivered"}"#;

    assert_eq!(
        callback(&client, format!("/webhooks/http/{key}?batch=1"), report)
            .await,
        Status::Ok
    );
    assert_eq!(
        client
            .get(format!("/webhooks/http/{key}?id=m-1&status=delivered"))
            .header(Header::new("X-Provider", "gateway"))
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    assert_eq!(
        callback(&client, format!("/webhooks/http/{key}"), "not json").await,
        Status::Unauthorized
    );
    // An account that declares no reports takes no callbacks.
    for key in [silent_key.as_str(), "unknown-key"] {
        assert_eq!(
            callback(&client, format!("/webhooks/http/{key}"), report).await,
            Status::NotFound
        );
        assert_eq!(
            client
                .get(format!("/webhooks/http/{key}"))
                .dispatch()
                .await
                .status(),
            Status::NotFound
        );
    }

    let (status, replaced) = json(
        credentials(
            &client,
            &event,
            json!({"id": id, "credentials": {"WEBHOOK_SECRET": "shared"}}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{replaced}");
    corrupt_credentials(&services, &event).await;
    assert_eq!(
        callback(&client, format!("/webhooks/http/{key}"), report).await,
        Status::InternalServerError
    );
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

    for id in [no_topic, with_topic] {
        let key = webhook_key(&services, &id).await;
        assert_eq!(
            callback(&client, format!("/webhooks/aws/{key}"), "{}").await,
            Status::Unauthorized
        );
    }
    assert_eq!(
        callback(&client, "/webhooks/aws/unknown-key".into(), "{}").await,
        Status::NotFound
    );
}

#[rocket::async_test]
async fn webhook_bodies_over_the_limit_are_refused_unread() {
    let services = Services::without_database();
    let client = services.client().await;
    let body = "x".repeat(512 * 1024 + 1);

    for path in [
        "/webhooks/meta/key",
        "/webhooks/viber/key",
        "/webhooks/http/key",
        "/webhooks/aws/key",
    ] {
        assert_eq!(
            callback(&client, path.to_string(), &body).await,
            Status::PayloadTooLarge,
            "{path}"
        );
    }
}

#[rocket::async_test]
async fn an_invalid_event_configuration_is_reported_and_not_stored() {
    let services = services().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let unchecked = account(&client, &event, console("SMS")).await;
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
    let id = account(&client, &event, console("SMS")).await;
    let (status, checked) = json(
        post(
            &client,
            "/messaging/accounts/check",
            &admin(&event),
            &json!({ "id": id }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{checked}");
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
            "sender_label": null,
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
    let client = services.client().await;
    let (id, _) = meta_account(&client, &event).await;
    let console = account(&client, &event, console("SMS")).await;
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
    ] {
        rows::execute(&services.hasura, statement, &[]).await;
    }

    for (path, body) in [
        ("/messaging/accounts/upsert", sns("Texts")),
        (
            "/messaging/accounts/credentials",
            json!({"id": id, "credentials": {"ACCESS_TOKEN": "rotated"}}),
        ),
        ("/messaging/accounts/check", json!({ "id": console })),
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
            text(post(&client, path, &admin(&event), &body).await).await,
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
    let service = keycloak(&event.tenant_id);
    let reference =
        link_request(&event.tenant_id, "reference", "session-digest");

    for (path, claims, body) in [
        ("/messages/send", &service, notice(&event, "k")),
        (
            "/messages/link",
            &service,
            link(&event, chrono::Duration::minutes(5)),
        ),
        ("/messages/link/status", &service, reference.clone()),
        ("/messages/link/confirm", &service, reference),
        ("/messaging/accounts/upsert", &admin(&event), sns("Texts")),
        (
            "/messaging/accounts/delete",
            &admin(&event),
            json!({ "id": id }),
        ),
        (
            "/messaging/accounts/credentials",
            &admin(&event),
            json!({"id": id, "credentials": {"API_KEY": "x"}}),
        ),
        (
            "/messaging/accounts/check",
            &admin(&event),
            json!({ "id": id }),
        ),
        (
            "/messaging/accounts/test",
            &admin(&event),
            json!({"id": id, "purpose": "NOTICE", "destination": PHONE}),
        ),
        (
            "/messaging/event-config",
            &admin(&event),
            json!({
                "election_event_id": event.election_event_id,
                "config": {"version": 1},
            }),
        ),
    ] {
        assert_eq!(
            text(post(&client, path, claims, &body).await).await,
            (Status::InternalServerError, "internal error".to_string()),
            "{path}"
        );
    }

    // Providers retry a callback that was not taken.
    for path in [
        "/webhooks/meta/key",
        "/webhooks/viber/key",
        "/webhooks/http/key",
        "/webhooks/aws/key",
    ] {
        assert_eq!(
            client
                .post(path)
                .header(ContentType::JSON)
                .header(bearer(&service))
                .body("{}")
                .dispatch()
                .await
                .status(),
            Status::ServiceUnavailable,
            "{path}"
        );
    }
    assert_eq!(
        client.get("/webhooks/http/key").dispatch().await.status(),
        Status::ServiceUnavailable
    );
    assert_eq!(
        client.get("/webhooks/meta/key").dispatch().await.status(),
        Status::InternalServerError
    );
}
