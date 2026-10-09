// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![recursion_limit = "256"]

//! Sending through the ledger, provider callbacks, Messenger links and the
//! sweep, against the real schema and a loopback provider. Each test
//! commits like production does, under a tenant of its own.
//!
//! Accounts and event configurations are written directly, so these tests
//! don't depend on how they are validated or where the configuration is
//! read from (both the first release's annotation and the table are
//! written).

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::{Client, Pool};
use hmac::{Hmac, Mac};
use messaging::providers::Endpoints;
use messaging::test_server::{Reply, TestServer};
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CreateMessengerLinkRequest, CredentialName, EventChannelConfig,
    EventMessagingConfig, MessageAttemptState, MessageChannel, MessageContent, MessagePurpose,
    MessageReason, MessengerLinkRequest, MessengerLinkState, OutOfWindowPolicy, TemplateBinding,
};
use serde_json::{json, Value};
use sha2::Sha256;
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::messaging::list_attempts;
use windmill::services::messaging::accounts::replace_credentials;
use windmill::services::messaging::audit::MessageKind;
use windmill::services::messaging::dispatch::{
    deliver, record_without_channel, Delivery, DeliveryResult, Dispatcher, Recipient,
};
use windmill::services::messaging::links::{confirm_link, create_link, link_status, LinkError};
use windmill::services::messaging::reconcile::reconcile_messages;
use windmill::services::messaging::webhooks::{meta_events, WebhookOutcome};

const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";
const APP_SECRET: &str = "synthetic-app-secret";
const PSID: &str = "6543210";

fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

struct World {
    tenant: Uuid,
    event: Uuid,
}

async fn world(pool: &Pool, seed: u32) -> World {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    // Messenger codes need the webhook component's address (Get Started
    // arrives through it).
    std::env::set_var(
        "MESSAGING_WEBHOOKS_PUBLIC_URL",
        "https://callbacks.example.org",
    );
    let world = World {
        tenant: id(seed, 1),
        event: id(seed, 2),
    };
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&world.tenant, &format!("tenant-{}", world.tenant)],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&world.event, &world.tenant],
        )
        .await
        .unwrap();
    world
}

fn ready(approved: &[MessagePurpose]) -> AccountCheck {
    AccountCheck {
        connected: true,
        production_access: true,
        approved_templates: approved
            .iter()
            .map(|purpose| (*purpose, vec!["en".to_string()]))
            .collect(),
        ..Default::default()
    }
}

/// A ready account, written directly.
async fn account(
    client: &mut Client,
    world: &World,
    channel: MessageChannel,
    sender: Value,
    key: &str,
    credentials: &[(CredentialName, &str)],
) -> Uuid {
    let provider = sender["provider"].as_str().unwrap().to_string();
    let account: Uuid = client
        .query_one(
            r#"
            INSERT INTO sequent_backend.messaging_account
                (tenant_id, channel, provider, name, sender, limits, status, webhook_key)
            VALUES ($1, $2, $3, $4, $5, '{}'::jsonb, $6, $7)
            RETURNING id
            "#,
            &[
                &world.tenant,
                &channel.to_string(),
                &provider,
                &format!("{channel} account"),
                &sender,
                &serde_json::to_value(ready(&[MessagePurpose::OTP, MessagePurpose::NOTICE]))
                    .unwrap(),
                &key,
            ],
        )
        .await
        .unwrap()
        .get(0);
    if !credentials.is_empty() {
        let tx = client.transaction().await.unwrap();
        replace_credentials(
            &tx,
            &world.tenant,
            &account,
            &credentials
                .iter()
                .map(|(name, value)| (*name, value.to_string()))
                .collect(),
            false,
        )
        .await
        .unwrap()
        .unwrap();
        tx.commit().await.unwrap();
    }
    account
}

async fn viber_account(client: &mut Client, world: &World, base_url: &str, key: &str) -> Uuid {
    let sender = AccountSender::VIBER_INFOBIP {
        base_url: base_url.to_string(),
        sender: "Elections".to_string(),
        approved_templates: BTreeMap::from([
            (
                MessagePurpose::NOTICE,
                BTreeMap::from([("en".to_string(), "tpl-notice".to_string())]),
            ),
            (
                MessagePurpose::OTP,
                BTreeMap::from([("en".to_string(), "tpl-otp".to_string())]),
            ),
        ]),
    };
    account(
        client,
        world,
        MessageChannel::VIBER,
        serde_json::to_value(sender).unwrap(),
        key,
        &[(CredentialName::API_KEY, "key-1")],
    )
    .await
}

async fn messenger_account(client: &mut Client, world: &World, key: &str) -> Uuid {
    let sender = AccountSender::MESSENGER_SEND_API {
        page_id: "page-1".to_string(),
        page_name: Some("Elections".to_string()),
        page_username: Some("elections".to_string()),
        api_version: "v23.0".to_string(),
        api_base_url: None,
    };
    account(
        client,
        world,
        MessageChannel::MESSENGER,
        serde_json::to_value(sender).unwrap(),
        key,
        &[
            (CredentialName::ACCESS_TOKEN, "page-token"),
            (CredentialName::APP_SECRET, APP_SECRET),
        ],
    )
    .await
}

fn channel(
    channel: MessageChannel,
    account: &Uuid,
    purposes: &[MessagePurpose],
) -> EventChannelConfig {
    EventChannelConfig {
        channel,
        account_id: account.to_string(),
        purposes: purposes.to_vec(),
        templates: vec![],
        out_of_window: OutOfWindowPolicy::DISABLED,
    }
}

/// Writes the event's configuration where the first release read it (the
/// annotation) and where it is read now (the table).
async fn configure(client: &mut Client, world: &World, config: EventMessagingConfig) {
    let config = serde_json::to_value(&config).unwrap();
    client
        .execute(
            "UPDATE sequent_backend.election_event
             SET annotations = jsonb_build_object('messaging:config', $3::jsonb::text)
             WHERE tenant_id = $1 AND id = $2",
            &[&world.tenant, &world.event, &config],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.messaging_event_config
                (tenant_id, election_event_id, config)
             VALUES ($1, $2, $3)
             ON CONFLICT (election_event_id) DO UPDATE SET config = EXCLUDED.config",
            &[&world.tenant, &world.event, &config],
        )
        .await
        .unwrap();
}

fn config(channels: Vec<EventChannelConfig>) -> EventMessagingConfig {
    EventMessagingConfig {
        channels,
        ..Default::default()
    }
}

fn content(text: &str) -> MessageContent {
    MessageContent {
        subject: Some("Election".to_string()),
        text: text.to_string(),
        html: None,
        template_parameters: vec![],
        code: None,
    }
}

fn notice(world: &World, key: &str, channel: MessageChannel, destination: &str) -> Delivery {
    Delivery {
        tenant_id: world.tenant,
        election_event_id: Some(world.event),
        purpose: MessagePurpose::NOTICE,
        recipient: Recipient {
            voter_id: Some("voter-1".to_string()),
            channel,
            destination: destination.to_string(),
            election_ids: vec![],
        },
        content: Some(content("Voting opens tomorrow.")),
        language: Some("en".to_string()),
        template_alias: Some("reminder".to_string()),
        logical_key: key.to_string(),
        expires_at: None,
        account_id: None,
        template_key: None,
        provider_template: None,
    }
}

async fn attempts(
    client: &mut Client,
    world: &World,
    key: &str,
) -> Vec<(MessageChannel, MessageAttemptState)> {
    let tx = client.transaction().await.unwrap();
    let attempts = list_attempts(&tx, &world.tenant, key).await.unwrap();
    tx.commit().await.unwrap();
    attempts.into_iter().map(|a| (a.channel, a.state)).collect()
}

fn sign(body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(APP_SECRET.as_bytes()).unwrap();
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

fn meta_dispatcher(server: &TestServer) -> Dispatcher {
    Dispatcher::new(Endpoints {
        meta_graph: Some(server.base_url.clone()),
        ..Endpoints::default()
    })
    .unwrap()
}

/// A signed Messenger webhook with these events.
async fn messenger_webhook(
    client: &mut Client,
    dispatcher: &Dispatcher,
    key: &str,
    events: Vec<Value>,
) -> WebhookOutcome {
    let body =
        json!({"object": "page", "entry": [{"id": "page-1", "messaging": events}]}).to_string();
    meta_events(
        client,
        dispatcher,
        key,
        Some(&sign(body.as_bytes())),
        body.as_bytes(),
    )
    .await
    .unwrap()
}

fn get_started() -> Value {
    json!({"sender": {"id": PSID}, "recipient": {"id": "page-1"},
           "timestamp": Utc::now().timestamp_millis(),
           "postback": {"title": "Get Started", "payload": "GET_STARTED"}})
}

const MASKED: &str = "+63 \u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022} 567";

#[tokio::test]
async fn an_accepted_notice_is_recorded_once_and_logged_once() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6001).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![Reply::Json(
        200,
        json!({"messages": [{"messageId": "ib-1"}]}),
    )])
    .await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w6001").await;
    configure(
        &mut client,
        &world,
        config(vec![channel(
            MessageChannel::VIBER,
            &viber,
            &[MessagePurpose::NOTICE],
        )]),
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let delivery = notice(
        &world,
        "send-template:s1:voter-1",
        MessageChannel::VIBER,
        "+639171234567",
    );

    let first = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(first.state, MessageAttemptState::ACCEPTED);
    assert!(first.newly_sent());
    // The electoral log entry of the attempt: masked receiver, account,
    // provider, purpose, kind, attempt and outcome.
    assert_eq!(first.recorded.len(), 1);
    let logged: Value = serde_json::from_str(&first.recorded[0].log_body()).unwrap();
    assert_eq!(
        logged,
        json!({
            "channel": "viber",
            "account_id": viber.to_string(),
            "provider": "VIBER_INFOBIP",
            "purpose": "NOTICE",
            "kind": "BULK_NOTICE",
            "message_id": first.message_id.unwrap().to_string(),
            "attempt": 1,
            "outcome": "ACCEPTED",
            "receiver": MASKED,
        })
    );
    assert_eq!(first.recorded[0].kind, MessageKind::BULK_NOTICE);

    let replay = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(replay.message_id, first.message_id);
    assert!(replay.recorded.is_empty());
    assert_eq!(server.requests().len(), 1);
    let row = client
        .query_one(
            "SELECT masked_destination, destination_country, billing_category
             FROM sequent_backend.message WHERE id = $1",
            &[&first.message_id.unwrap()],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), MASKED);
    assert_eq!(row.get::<_, Option<String>>(1).as_deref(), Some("PH"));
    assert_eq!(
        row.get::<_, Option<String>>(2).as_deref(),
        Some("TRANSACTIONAL")
    );
}

#[tokio::test]
async fn expired_codes_are_recorded_as_failed_and_never_sent() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6002).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w6002").await;
    configure(
        &mut client,
        &world,
        config(vec![channel(
            MessageChannel::VIBER,
            &viber,
            &[MessagePurpose::OTP],
        )]),
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let mut code = notice(
        &world,
        "keycloak:otp-1",
        MessageChannel::VIBER,
        "+639171234567",
    );
    code.purpose = MessagePurpose::OTP;
    code.expires_at = Some(Utc::now() - Duration::seconds(1));
    code.content.as_mut().unwrap().code = Some("482619".to_string());

    let result = deliver(&mut client, &dispatcher, &code).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(result.reason.as_deref(), Some("CODE_EXPIRED"));
    assert_eq!(result.next_retry_at, None);
    assert_eq!(
        attempts(&mut client, &world, "keycloak:otp-1").await,
        vec![(MessageChannel::VIBER, MessageAttemptState::FAILED)]
    );
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn a_channel_the_post_does_not_offer_fails_once_and_nothing_goes_elsewhere() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x600a).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w600a").await;
    let mut event_config = config(vec![channel(
        MessageChannel::VIBER,
        &viber,
        &[MessagePurpose::NOTICE],
    )]);
    event_config.election_channels =
        BTreeMap::from([("post-1".to_string(), vec![MessageChannel::SMS])]);
    configure(&mut client, &world, event_config).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();

    // The voter's Post offers SMS only: their Viber notice fails, and no
    // SMS is sent instead.
    let mut restricted = notice(
        &world,
        "send-template:s2:voter-1",
        MessageChannel::VIBER,
        "+639171234567",
    );
    restricted.recipient.election_ids = vec!["post-1".to_string()];
    let result = deliver(&mut client, &dispatcher, &restricted)
        .await
        .unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(
        result.reason.as_deref(),
        Some(MessageReason::CHANNEL_NOT_OFFERED_FOR_NOTICES.as_str())
    );
    assert_eq!(result.recorded.len(), 1);
    // Not offered at all for the event: the same one reason.
    let whatsapp = notice(
        &world,
        "send-template:s2:voter-2",
        MessageChannel::WHATSAPP,
        "+639171234568",
    );
    let result = deliver(&mut client, &dispatcher, &whatsapp).await.unwrap();
    assert_eq!(
        result.reason.as_deref(),
        Some("Channel not offered for notices")
    );
    // A replay records nothing new.
    assert!(deliver(&mut client, &dispatcher, &whatsapp)
        .await
        .unwrap()
        .recorded
        .is_empty());
    assert_eq!(
        attempts(&mut client, &world, "send-template:s2:voter-1").await,
        vec![(MessageChannel::VIBER, MessageAttemptState::FAILED)]
    );
    assert_eq!(
        attempts(&mut client, &world, "send-template:s2:voter-2").await,
        vec![(MessageChannel::WHATSAPP, MessageAttemptState::FAILED)]
    );
    assert!(server.requests().is_empty());
}

async fn record_no_channel(client: &mut Client, world: &World) -> DeliveryResult {
    record_without_channel(
        client,
        world.tenant,
        Some(world.event),
        Some("voter-9".to_string()),
        MessagePurpose::NOTICE,
        Some("reminder".to_string()),
        None,
        "send-template:s3:voter-9",
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_voter_without_a_channel_gets_one_failed_record() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x600b).await;
    let mut client = pool.get().await.unwrap();
    let first: DeliveryResult = record_no_channel(&mut client, &world).await;
    assert_eq!(first.state, MessageAttemptState::FAILED);
    assert_eq!(first.reason.as_deref(), Some("No contact channel"));
    assert_eq!(first.recorded.len(), 1);
    let replay = record_no_channel(&mut client, &world).await;
    assert_eq!(replay.message_id, first.message_id);
    assert!(replay.recorded.is_empty());
}

#[tokio::test]
async fn a_confirmed_failure_stays_on_its_channel_and_a_lost_answer_waits() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6003).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![
        Reply::Json(
            400,
            json!({"error": {"code": 551, "message": "user unavailable"}}),
        ),
        Reply::Drop,
    ])
    .await;
    let messenger = messenger_account(&mut client, &world, "w6003").await;
    let sms = viber_account(&mut client, &world, &server.base_url, "w6003-viber").await;
    configure(
        &mut client,
        &world,
        config(vec![
            channel(
                MessageChannel::MESSENGER,
                &messenger,
                &[MessagePurpose::NOTICE],
            ),
            channel(MessageChannel::VIBER, &sms, &[MessagePurpose::NOTICE]),
        ]),
    )
    .await;
    let dispatcher = meta_dispatcher(&server);
    // The voter pressed Get Started: the conversation window is open.
    assert_eq!(
        messenger_webhook(&mut client, &dispatcher, "w6003", vec![get_started()]).await,
        WebhookOutcome::Accepted
    );

    let delivery = notice(
        &world,
        "send-template:s4:voter-1",
        MessageChannel::MESSENGER,
        PSID,
    );
    let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(result.channel, MessageChannel::MESSENGER);
    assert_eq!(result.next_retry_at, None);
    assert_eq!(
        attempts(&mut client, &world, "send-template:s4:voter-1").await,
        vec![(MessageChannel::MESSENGER, MessageAttemptState::FAILED)]
    );

    // The provider takes the second message but its answer is lost.
    let lost = notice(
        &world,
        "send-template:s5:voter-1",
        MessageChannel::MESSENGER,
        PSID,
    );
    let first = deliver(&mut client, &dispatcher, &lost).await.unwrap();
    assert_eq!(first.state, MessageAttemptState::UNKNOWN);
    let again = deliver(&mut client, &dispatcher, &lost).await.unwrap();
    assert_eq!(again.message_id, first.message_id);
    assert_eq!(again.state, MessageAttemptState::UNKNOWN);
    assert!(again.recorded.is_empty());
    assert_eq!(
        attempts(&mut client, &world, "send-template:s5:voter-1").await,
        vec![(MessageChannel::MESSENGER, MessageAttemptState::UNKNOWN)]
    );
    // Two sends reached the provider: no retry, and never another channel.
    assert_eq!(server.requests().len(), 2);
}

#[tokio::test]
async fn signed_reports_update_attempts_and_unsigned_ones_are_refused() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6004).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![Reply::Json(200, json!({"message_id": "m_out_1"}))]).await;
    let messenger = messenger_account(&mut client, &world, "w6004").await;
    configure(
        &mut client,
        &world,
        config(vec![channel(
            MessageChannel::MESSENGER,
            &messenger,
            &[MessagePurpose::NOTICE],
        )]),
    )
    .await;
    let dispatcher = meta_dispatcher(&server);
    messenger_webhook(&mut client, &dispatcher, "w6004", vec![get_started()]).await;
    let delivery = notice(
        &world,
        "send-template:s6:voter-1",
        MessageChannel::MESSENGER,
        PSID,
    );
    assert_eq!(
        deliver(&mut client, &dispatcher, &delivery)
            .await
            .unwrap()
            .state,
        MessageAttemptState::ACCEPTED
    );
    // Within the window: a reply, not the utility template.
    assert_eq!(server.requests()[0].json()["messaging_type"], "RESPONSE");

    let report = json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
        {"sender": {"id": PSID}, "recipient": {"id": "page-1"}, "timestamp": Utc::now().timestamp_millis(),
         "delivery": {"mids": ["m_out_1"], "watermark": Utc::now().timestamp_millis()}}
    ]}]})
    .to_string();
    assert_eq!(
        meta_events(
            &mut client,
            &dispatcher,
            "w6004",
            Some("sha256=00"),
            report.as_bytes()
        )
        .await
        .unwrap(),
        WebhookOutcome::Unauthorized
    );
    assert_eq!(
        meta_events(
            &mut client,
            &dispatcher,
            "unknown-key",
            Some(&sign(report.as_bytes())),
            report.as_bytes()
        )
        .await
        .unwrap(),
        WebhookOutcome::NotFound
    );
    assert_eq!(
        attempts(&mut client, &world, "send-template:s6:voter-1").await,
        vec![(MessageChannel::MESSENGER, MessageAttemptState::ACCEPTED)]
    );
    assert_eq!(
        meta_events(
            &mut client,
            &dispatcher,
            "w6004",
            Some(&sign(report.as_bytes())),
            report.as_bytes()
        )
        .await
        .unwrap(),
        WebhookOutcome::Accepted
    );
    assert_eq!(
        attempts(&mut client, &world, "send-template:s6:voter-1").await,
        vec![(MessageChannel::MESSENGER, MessageAttemptState::DELIVERED)]
    );
}

#[tokio::test]
async fn messages_voters_write_are_dropped_and_open_no_window() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x600c).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![]).await;
    let messenger = messenger_account(&mut client, &world, "w600c").await;
    configure(
        &mut client,
        &world,
        config(vec![channel(
            MessageChannel::MESSENGER,
            &messenger,
            &[MessagePurpose::NOTICE],
        )]),
    )
    .await;
    let dispatcher = meta_dispatcher(&server);
    let text = json!({"sender": {"id": PSID}, "recipient": {"id": "page-1"},
                      "timestamp": Utc::now().timestamp_millis(),
                      "message": {"mid": "m_in_1", "text": "hello"}});
    assert_eq!(
        messenger_webhook(&mut client, &dispatcher, "w600c", vec![text]).await,
        WebhookOutcome::Accepted
    );
    let stored: i64 = client
        .query_one(
            "SELECT (SELECT count(*) FROM sequent_backend.message WHERE tenant_id = $1)
                  + (SELECT count(*) FROM sequent_backend.messenger_contact WHERE tenant_id = $1)",
            &[&world.tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(stored, 0);
    // So a notice outside the window needs the utility template, which the
    // event didn't bind: nothing reaches the provider.
    let delivery = notice(
        &world,
        "send-template:s7:voter-1",
        MessageChannel::MESSENGER,
        PSID,
    );
    let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert!(server.requests().is_empty());
    // No automatic reply either.
    assert_eq!(
        attempts(&mut client, &world, "send-template:s7:voter-1").await,
        vec![(MessageChannel::MESSENGER, MessageAttemptState::FAILED)]
    );
}

#[tokio::test]
async fn messenger_links_send_the_code_only_to_the_linking_conversation() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6005).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![Reply::Json(200, json!({"message_id": "m_code"}))]).await;
    let messenger = messenger_account(&mut client, &world, "w6005").await;
    configure(
        &mut client,
        &world,
        config(vec![channel(
            MessageChannel::MESSENGER,
            &messenger,
            &[MessagePurpose::OTP],
        )]),
    )
    .await;
    let dispatcher = meta_dispatcher(&server);

    let created = create_link(
        &mut client,
        &CreateMessengerLinkRequest {
            tenant_id: world.tenant.to_string(),
            election_event_id: Some(world.event.to_string()),
            auth_session: "session-digest".to_string(),
            challenge: "challenge-digest".to_string(),
            code: "482619".to_string(),
            language: Some("en".to_string()),
            content: content("Your verification code is 482619."),
            expires_at: (Utc::now() + Duration::minutes(30)).to_rfc3339(),
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(created.link.starts_with("https://m.me/elections?ref="));
    let lifetime = chrono::DateTime::parse_from_rfc3339(&created.expires_at)
        .unwrap()
        .with_timezone(&Utc)
        - Utc::now();
    assert!(lifetime <= Duration::minutes(10));

    let request = |session: &str| MessengerLinkRequest {
        tenant_id: world.tenant.to_string(),
        reference: created.reference.clone(),
        auth_session: session.to_string(),
        challenge: "challenge-digest".to_string(),
    };
    assert_eq!(
        link_status(&mut client, &request("session-digest"))
            .await
            .unwrap()
            .unwrap()
            .state,
        MessengerLinkState::PENDING
    );
    assert_eq!(
        confirm_link(&mut client, &request("session-digest"))
            .await
            .unwrap()
            .unwrap()
            .state,
        MessengerLinkState::PENDING
    );

    // Typing something in the chat links nothing: there is no link word.
    let typed = json!({"sender": {"id": PSID}, "recipient": {"id": "page-1"},
                       "timestamp": Utc::now().timestamp_millis(),
                       "message": {"mid": "m_in", "text": created.reference}});
    messenger_webhook(&mut client, &dispatcher, "w6005", vec![typed]).await;
    assert!(server.requests().is_empty());

    // Get Started with the reference: the window opens before the code is
    // sent, so the code goes out as a reply, not refused as out of window.
    let referral = json!({"sender": {"id": PSID}, "recipient": {"id": "page-1"},
                          "timestamp": Utc::now().timestamp_millis(),
                          "postback": {"title": "Get Started", "payload": "GET_STARTED",
                                       "referral": {"ref": created.reference, "source": "SHORTLINK",
                                                    "type": "OPEN_THREAD"}}});
    messenger_webhook(&mut client, &dispatcher, "w6005", vec![referral]).await;
    let sent = &server.requests()[0];
    assert_eq!(sent.json()["recipient"]["id"], PSID);
    assert_eq!(sent.json()["messaging_type"], "RESPONSE");
    assert!(sent.json()["message"]["text"]
        .as_str()
        .unwrap()
        .contains("482619"));

    // A replay of the same referral from another conversation is ignored.
    let replay = json!({"sender": {"id": "999"}, "recipient": {"id": "page-1"},
                        "timestamp": Utc::now().timestamp_millis(),
                        "referral": {"ref": created.reference}});
    messenger_webhook(&mut client, &dispatcher, "w6005", vec![replay]).await;
    assert_eq!(server.requests().len(), 1);

    assert_eq!(
        confirm_link(&mut client, &request("another-session"))
            .await
            .unwrap(),
        Err(LinkError::NotFound)
    );
    let confirmed = confirm_link(&mut client, &request("session-digest"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(confirmed.state, MessengerLinkState::CONFIRMED);
    assert_eq!(confirmed.page_scoped_id.as_deref(), Some(PSID));
    assert_eq!(confirmed.page_id.as_deref(), Some("page-1"));
    let again = confirm_link(&mut client, &request("session-digest"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(again.state, MessengerLinkState::CONFIRMED);
    assert_eq!(again.page_scoped_id, None);

    let payload: Option<Vec<u8>> = client
        .query_one(
            "SELECT encrypted_payload FROM sequent_backend.messenger_link WHERE tenant_id = $1",
            &[&world.tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(payload, None);
}

#[tokio::test]
async fn a_stopped_worker_leaves_never_sent_attempts_failed_and_maybe_sent_ones_unknown() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6006).await;
    let mut client = pool.get().await.unwrap();
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, template_alias,
                 masked_destination, destination_digest, logical_key, attempt, state,
                 created_at, dispatched_at)
            VALUES
                ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', 'reminder', '-', 'd',
                 'send-template:stale:never', 1, 'QUEUED', now() - interval '5 minutes', NULL),
                ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', 'reminder', '-', 'd',
                 'send-template:stale:maybe', 1, 'QUEUED', now() - interval '5 minutes',
                 now() - interval '5 minutes'),
                ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', 'reminder', '-', 'd',
                 'send-template:stale:fresh', 1, 'QUEUED', now(), NULL)
            "#,
            &[&world.tenant, &world.event],
        )
        .await
        .unwrap();
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let summary = reconcile_messages(&mut client, &dispatcher).await.unwrap();
    assert!(summary.never_sent >= 1 && summary.maybe_sent >= 1);
    assert_eq!(
        attempts(&mut client, &world, "send-template:stale:never").await,
        vec![(MessageChannel::SMS, MessageAttemptState::FAILED)]
    );
    assert_eq!(
        attempts(&mut client, &world, "send-template:stale:maybe").await,
        vec![(MessageChannel::SMS, MessageAttemptState::UNKNOWN)]
    );
    assert_eq!(
        attempts(&mut client, &world, "send-template:stale:fresh").await,
        vec![(MessageChannel::SMS, MessageAttemptState::QUEUED)]
    );
    let never = client
        .query_one(
            "SELECT error, failure, next_retry_at IS NOT NULL FROM sequent_backend.message
             WHERE tenant_id = $1 AND logical_key = 'send-template:stale:never'",
            &[&world.tenant],
        )
        .await
        .unwrap();
    assert_eq!(
        never.get::<_, Option<String>>(0).as_deref(),
        Some("The worker stopped before sending")
    );
    assert_eq!(
        never.get::<_, Option<String>>(1).as_deref(),
        Some("TRANSIENT")
    );
    // Never sent, so it is retried on the same channel.
    assert!(never.get::<_, bool>(2));
}

#[tokio::test]
async fn a_bound_template_goes_out_as_a_utility_message_outside_the_window() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6007).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![Reply::Json(200, json!({"message_id": "m_util"}))]).await;
    let messenger = messenger_account(&mut client, &world, "w6007").await;
    configure(
        &mut client,
        &world,
        config(vec![EventChannelConfig {
            templates: vec![
                TemplateBinding {
                    purpose: MessagePurpose::NOTICE,
                    key: None,
                    language: "en".to_string(),
                    provider_template: "generic_notice".to_string(),
                    provider_language: None,
                },
                TemplateBinding {
                    purpose: MessagePurpose::NOTICE,
                    key: Some("reminder".to_string()),
                    language: "en".to_string(),
                    provider_template: "voting_reminder".to_string(),
                    provider_language: Some("en_US".to_string()),
                },
            ],
            // The first release's "do not send": Messenger now always uses
            // the utility template outside the window.
            out_of_window: OutOfWindowPolicy::DISABLED,
            ..channel(
                MessageChannel::MESSENGER,
                &messenger,
                &[MessagePurpose::NOTICE],
            )
        }]),
    )
    .await;
    let dispatcher = meta_dispatcher(&server);

    // The voter never pressed Get Started: there is no conversation window.
    let mut delivery = notice(
        &world,
        "send-template:s8:voter-1",
        MessageChannel::MESSENGER,
        PSID,
    );
    delivery.template_key = Some("reminder".to_string());
    delivery.content.as_mut().unwrap().template_parameters = vec!["Ana".to_string()];
    let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::ACCEPTED);
    let body = server.requests()[0].json();
    assert_eq!(body["messaging_type"], "UTILITY");
    assert_eq!(body["message"]["template"]["name"], "voting_reminder");
    assert_eq!(body["message"]["template"]["language"]["code"], "en_US");
    assert_eq!(
        body["message"]["template"]["components"][0]["parameters"][0]["text"],
        "Ana"
    );
    let category: Option<String> = client
        .query_one(
            "SELECT billing_category FROM sequent_backend.message WHERE id = $1",
            &[&result.message_id.unwrap()],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(category.as_deref(), Some("UTILITY"));
}

#[tokio::test]
async fn a_removed_provider_loads_and_cannot_send() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6008).await;
    let mut client = pool.get().await.unwrap();
    let viber = account(
        &mut client,
        &world,
        MessageChannel::VIBER,
        json!({
            "provider": "HTTP_API",
            "label": "Partner",
            "send": {"url": "https://partner.example.org/send", "body": {"to": "{{to}}"}},
            "message_id_pointer": "/id",
        }),
        "w6008",
        &[],
    )
    .await;
    // Through the account itself: an event configuration naming it no
    // longer offers the channel (the send-time check drops it).
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let mut delivery = notice(
        &world,
        "account-test:9",
        MessageChannel::VIBER,
        "+639171234567",
    );
    delivery.election_event_id = None;
    delivery.account_id = Some(viber);
    let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(
        result.reason.as_deref(),
        Some("Unsupported provider: replace this account")
    );
}

#[tokio::test]
async fn a_test_send_through_one_account_needs_no_event_configuration() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6009).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![Reply::Json(
        200,
        json!({"messages": [{"messageId": "ib-test"}]}),
    )])
    .await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w6009").await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let mut delivery = notice(
        &world,
        "account-test:1",
        MessageChannel::VIBER,
        "+639171234567",
    );
    delivery.election_event_id = None;
    delivery.account_id = Some(viber);
    let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::ACCEPTED);
    assert_eq!(result.channel, MessageChannel::VIBER);
    assert_eq!(result.recorded[0].kind, MessageKind::ACCOUNT_TEST);
}
