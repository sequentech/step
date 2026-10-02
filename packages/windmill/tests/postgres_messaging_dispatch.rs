// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending through the ledger, provider callbacks, Messenger links and
//! reconciliation, against the real schema and a loopback provider. Each
//! test commits like production does, under a tenant of its own.

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::{Client, Pool};
use hmac::{Hmac, Mac};
use messaging::providers::Endpoints;
use messaging::test_server::{Reply, TestServer};
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, CreateMessengerLinkRequest, CredentialName,
    EventChannelConfig, EventMessagingConfig, MessageAttemptState, MessageChannel, MessageContent,
    MessagePurpose, MessengerLinkRequest, MessengerLinkState, OutOfWindowPolicy, ProviderApproval,
};
use serde_json::json;
use sha2::Sha256;
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::messaging::{
    insert_messaging_account, list_attempts, update_messaging_account_status, AccountSettings,
    MessagingAccount,
};
use windmill::services::messaging::accounts::replace_credentials;
use windmill::services::messaging::config::{save_event_messaging_config, SaveOutcome};
use windmill::services::messaging::dispatch::{
    deliver, Delivery, DeliveryResult, Dispatcher, FallbackPolicy, Recipient,
};
use windmill::services::messaging::links::{confirm_link, create_link, link_status, LinkError};
use windmill::services::messaging::reconcile::reconcile_messages;
use windmill::services::messaging::webhooks::{meta_events, WebhookOutcome};

const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";
const APP_SECRET: &str = "synthetic-app-secret";

fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

struct World {
    tenant: Uuid,
    event: Uuid,
}

async fn world(pool: &Pool, seed: u32) -> World {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    let world = World {
        tenant: id(seed, 1),
        event: id(seed, 2),
    };
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&world.tenant, &format!("tenant-{}", world.tenant)],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&world.event, &world.tenant],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    world
}

fn ready() -> AccountCheck {
    AccountCheck {
        connected: true,
        production_access: true,
        ..Default::default()
    }
}

async fn account(
    client: &mut Client,
    world: &World,
    channel: MessageChannel,
    sender: AccountSender,
    key: &str,
) -> MessagingAccount {
    let tx = client.transaction().await.unwrap();
    let account = insert_messaging_account(
        &tx,
        &world.tenant,
        &AccountSettings {
            channel,
            name: format!("{channel} account"),
            sender,
            limits: AccountLimits::default(),
            provider_approval: ProviderApproval::PENDING,
            is_default: false,
        },
        key,
    )
    .await
    .unwrap();
    update_messaging_account_status(&tx, &world.tenant, &account.id, &ready())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    account
}

async fn messenger_account(client: &mut Client, world: &World, key: &str) -> MessagingAccount {
    let account = account(
        client,
        world,
        MessageChannel::MESSENGER,
        AccountSender::MESSENGER_SEND_API {
            page_id: "page-1".to_string(),
            page_name: Some("COMELEC".to_string()),
            page_username: Some("comelec".to_string()),
            api_version: "v23.0".to_string(),
        },
        key,
    )
    .await;
    let tx = client.transaction().await.unwrap();
    replace_credentials(
        &tx,
        &world.tenant,
        &account.id,
        &BTreeMap::from([
            (CredentialName::ACCESS_TOKEN, "page-token".to_string()),
            (CredentialName::APP_SECRET, APP_SECRET.to_string()),
        ]),
        true,
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
    account
}

async fn configure(
    client: &mut Client,
    world: &World,
    channels: Vec<(MessageChannel, &MessagingAccount, Vec<MessagePurpose>)>,
    fallback: Vec<MessageChannel>,
) {
    let config = EventMessagingConfig {
        channels: channels
            .into_iter()
            .map(|(channel, account, purposes)| EventChannelConfig {
                channel,
                account_id: account.id.to_string(),
                purposes,
                templates: vec![],
                out_of_window: OutOfWindowPolicy::DISABLED,
            })
            .collect(),
        notice_fallback: fallback,
        ..Default::default()
    };
    let tx = client.transaction().await.unwrap();
    let outcome = save_event_messaging_config(&tx, &world.tenant, &world.event, &config)
        .await
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Saved(_)), "{outcome:?}");
    tx.commit().await.unwrap();
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

fn notice(world: &World, key: &str, destinations: BTreeMap<MessageChannel, String>) -> Delivery {
    let channels: Vec<MessageChannel> = destinations.keys().copied().collect();
    Delivery {
        tenant_id: world.tenant,
        election_event_id: Some(world.event),
        purpose: MessagePurpose::NOTICE,
        first_channel: channels[0],
        fallback: FallbackPolicy::EVENT_ORDER,
        recipient: Recipient {
            voter_id: Some("voter-1".to_string()),
            destinations,
            verified: channels.clone(),
            election_ids: vec![],
        },
        contents: channels
            .iter()
            .map(|c| (*c, content("Voting opens tomorrow.")))
            .collect(),
        language: Some("en".to_string()),
        template_alias: Some("reminder".to_string()),
        logical_key: key.to_string(),
        expires_at: None,
        account_id: None,
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

#[tokio::test]
async fn an_accepted_notice_is_recorded_once_and_never_resent() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6001).await;
    let mut client = pool.get().await.unwrap();
    let sms = account(
        &mut client,
        &world,
        MessageChannel::SMS,
        AccountSender::CONSOLE {},
        "w6001",
    )
    .await;
    configure(
        &mut client,
        &world,
        vec![(MessageChannel::SMS, &sms, vec![MessagePurpose::NOTICE])],
        vec![],
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let delivery = notice(
        &world,
        "send-1:voter-1",
        BTreeMap::from([(MessageChannel::SMS, "+639171234567".to_string())]),
    );

    let first = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(first.state, MessageAttemptState::ACCEPTED);
    let replay = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(replay.message_id, first.message_id);
    assert_eq!(
        attempts(&mut client, &world, "send-1:voter-1").await,
        vec![(MessageChannel::SMS, MessageAttemptState::ACCEPTED)]
    );
    let masked: String = client
        .query_one(
            "SELECT masked_destination FROM sequent_backend.message WHERE id = $1",
            &[&first.message_id.unwrap()],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(masked, "+63******4567");
}

#[tokio::test]
async fn expired_codes_are_recorded_as_failed_and_never_sent() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6002).await;
    let mut client = pool.get().await.unwrap();
    let sms = account(
        &mut client,
        &world,
        MessageChannel::SMS,
        AccountSender::CONSOLE {},
        "w6002",
    )
    .await;
    configure(
        &mut client,
        &world,
        vec![(MessageChannel::SMS, &sms, vec![MessagePurpose::OTP])],
        vec![],
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let mut code = notice(
        &world,
        "otp-1",
        BTreeMap::from([(MessageChannel::SMS, "+639171234567".to_string())]),
    );
    code.purpose = MessagePurpose::OTP;
    code.fallback = FallbackPolicy::NONE;
    code.expires_at = Some(Utc::now() - Duration::seconds(1));
    code.contents.get_mut(&MessageChannel::SMS).unwrap().code = Some("482619".to_string());

    let result = deliver(&mut client, &dispatcher, &code).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(result.reason.as_deref(), Some("CODE_EXPIRED"));
    assert_eq!(
        attempts(&mut client, &world, "otp-1").await,
        vec![(MessageChannel::SMS, MessageAttemptState::FAILED)]
    );
}

#[tokio::test]
async fn a_confirmed_failure_falls_back_and_a_lost_answer_waits() {
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
    let sms = account(
        &mut client,
        &world,
        MessageChannel::SMS,
        AccountSender::CONSOLE {},
        "w6003-sms",
    )
    .await;
    configure(
        &mut client,
        &world,
        vec![
            (
                MessageChannel::MESSENGER,
                &messenger,
                vec![MessagePurpose::NOTICE],
            ),
            (MessageChannel::SMS, &sms, vec![MessagePurpose::NOTICE]),
        ],
        vec![MessageChannel::SMS],
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints {
        meta_graph: server.base_url.clone(),
        aws: None,
    })
    .unwrap();
    // The voter wrote to the Page, so the conversation window is open.
    let referral = json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
        {"sender": {"id": "6543210"}, "recipient": {"id": "page-1"}, "timestamp": Utc::now().timestamp_millis(),
         "message": {"mid": "m_in_1", "text": "hello"}}
    ]}]})
    .to_string();
    assert_eq!(
        meta_events(
            &mut client,
            &dispatcher,
            "w6003",
            Some(&sign(referral.as_bytes())),
            referral.as_bytes()
        )
        .await
        .unwrap(),
        WebhookOutcome::Accepted
    );

    let mut delivery = notice(
        &world,
        "send-2:voter-1",
        BTreeMap::from([
            (MessageChannel::MESSENGER, "6543210".to_string()),
            (MessageChannel::SMS, "+639171234567".to_string()),
        ]),
    );
    delivery.first_channel = MessageChannel::MESSENGER;
    let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::ACCEPTED);
    assert_eq!(result.channel, Some(MessageChannel::SMS));
    assert_eq!(
        attempts(&mut client, &world, "send-2:voter-1").await,
        vec![
            (MessageChannel::MESSENGER, MessageAttemptState::FAILED),
            (MessageChannel::SMS, MessageAttemptState::ACCEPTED),
        ]
    );

    // The provider takes the second message but its answer is lost.
    let mut lost = notice(
        &world,
        "send-3:voter-1",
        BTreeMap::from([
            (MessageChannel::MESSENGER, "6543210".to_string()),
            (MessageChannel::SMS, "+639171234567".to_string()),
        ]),
    );
    lost.first_channel = MessageChannel::MESSENGER;
    let first: DeliveryResult = deliver(&mut client, &dispatcher, &lost).await.unwrap();
    assert_eq!(first.state, MessageAttemptState::UNKNOWN);
    let again = deliver(&mut client, &dispatcher, &lost).await.unwrap();
    assert_eq!(again.message_id, first.message_id);
    assert_eq!(again.state, MessageAttemptState::UNKNOWN);
    assert_eq!(
        attempts(&mut client, &world, "send-3:voter-1").await,
        vec![(MessageChannel::MESSENGER, MessageAttemptState::UNKNOWN)]
    );
    // Two sends reached the provider: no retry and no fallback for the lost one.
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
        vec![(
            MessageChannel::MESSENGER,
            &messenger,
            vec![MessagePurpose::NOTICE],
        )],
        vec![],
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints {
        meta_graph: server.base_url.clone(),
        aws: None,
    })
    .unwrap();
    let inbound = json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
        {"sender": {"id": "6543210"}, "recipient": {"id": "page-1"}, "timestamp": Utc::now().timestamp_millis(),
         "message": {"mid": "m_in_2", "text": "hi"}}
    ]}]})
    .to_string();
    meta_events(
        &mut client,
        &dispatcher,
        "w6004",
        Some(&sign(inbound.as_bytes())),
        inbound.as_bytes(),
    )
    .await
    .unwrap();
    let delivery = notice(
        &world,
        "send-4:voter-1",
        BTreeMap::from([(MessageChannel::MESSENGER, "6543210".to_string())]),
    );
    assert_eq!(
        deliver(&mut client, &dispatcher, &delivery)
            .await
            .unwrap()
            .state,
        MessageAttemptState::ACCEPTED
    );

    let report = json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
        {"sender": {"id": "6543210"}, "recipient": {"id": "page-1"}, "timestamp": Utc::now().timestamp_millis(),
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
        attempts(&mut client, &world, "send-4:voter-1").await,
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
        attempts(&mut client, &world, "send-4:voter-1").await,
        vec![(MessageChannel::MESSENGER, MessageAttemptState::DELIVERED)]
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
        vec![(
            MessageChannel::MESSENGER,
            &messenger,
            vec![MessagePurpose::OTP],
        )],
        vec![],
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints {
        meta_graph: server.base_url.clone(),
        aws: None,
    })
    .unwrap();

    let created = create_link(
        &mut client,
        &CreateMessengerLinkRequest {
            tenant_id: world.tenant.to_string(),
            election_event_id: Some(world.event.to_string()),
            auth_session: "session-digest".to_string(),
            challenge: "challenge-digest".to_string(),
            code: "482619".to_string(),
            language: Some("en".to_string()),
            content: content("Your COMELEC verification code is 482619."),
            expires_at: (Utc::now() + Duration::minutes(30)).to_rfc3339(),
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(created.link.starts_with("https://m.me/comelec?ref="));
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
    // Not before the code was sent, and never from another session.
    assert_eq!(
        confirm_link(&mut client, &request("session-digest"))
            .await
            .unwrap()
            .unwrap()
            .state,
        MessengerLinkState::PENDING
    );

    let referral = json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
        {"sender": {"id": "6543210"}, "recipient": {"id": "page-1"}, "timestamp": Utc::now().timestamp_millis(),
         "referral": {"ref": created.reference, "source": "SHORTLINK", "type": "OPEN_THREAD"}}
    ]}]})
    .to_string();
    meta_events(
        &mut client,
        &dispatcher,
        "w6005",
        Some(&sign(referral.as_bytes())),
        referral.as_bytes(),
    )
    .await
    .unwrap();
    let sent = &server.requests()[0];
    assert_eq!(sent.json()["recipient"]["id"], "6543210");
    assert!(sent.json()["message"]["text"]
        .as_str()
        .unwrap()
        .contains("482619"));

    // A replay of the same referral from another conversation is ignored.
    let replay = json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
        {"sender": {"id": "999"}, "recipient": {"id": "page-1"}, "timestamp": Utc::now().timestamp_millis(),
         "referral": {"ref": created.reference}}
    ]}]})
    .to_string();
    meta_events(
        &mut client,
        &dispatcher,
        "w6005",
        Some(&sign(replay.as_bytes())),
        replay.as_bytes(),
    )
    .await
    .unwrap();
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
    assert_eq!(confirmed.page_scoped_id.as_deref(), Some("6543210"));
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
async fn stale_queued_attempts_become_unknown() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x6006).await;
    let mut client = pool.get().await.unwrap();
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, masked_destination,
                 destination_digest, logical_key, attempt, state, created_at)
            VALUES ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', '+63******4567', 'd', 'stale-1', 1,
                    'QUEUED', now() - interval '1 hour')
            "#,
            &[&world.tenant, &world.event],
        )
        .await
        .unwrap();
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let summary = reconcile_messages(&mut client, &dispatcher).await.unwrap();
    assert!(summary.marked_unknown >= 1);
    assert_eq!(
        attempts(&mut client, &world, "stale-1").await,
        vec![(MessageChannel::SMS, MessageAttemptState::UNKNOWN)]
    );
}
