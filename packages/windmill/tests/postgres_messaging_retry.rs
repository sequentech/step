// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![recursion_limit = "256"]

//! Same-channel retries across jobs, the reconciliation sweep under a
//! backlog, and replayed jobs, against the real schema and a loopback
//! provider (WM-B1, WM-B2, WM-B3). Each test commits like production does,
//! under a tenant of its own.

#[path = "support/schema.rs"]
mod schema;

use async_trait::async_trait;
use deadpool_postgres::{Client, Pool};
use messaging::providers::Endpoints;
use messaging::test_server::{Reply, TestServer};
use sequent_core::types::keycloak::User;
use sequent_core::types::messaging::MessageReason;
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, EventChannelConfig, EventMessagingConfig,
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose, OutOfWindowPolicy,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;
use windmill::postgres::messaging::{list_attempts, MessageRecord};
use windmill::services::messaging::accounts::replace_credentials;
use windmill::services::messaging::audit::MessageKind;
use windmill::services::messaging::dispatch::{deliver, Delivery, Dispatcher, Recipient};
use windmill::services::messaging::reconcile::{
    reconcile_messages, sweep_messages, RetryContent, RetryRenderer,
};
use windmill::tasks::send_template::{send_voter_notice, RetryByOrigin, RetryDirectory};

const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";

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

/// A ready Viber (Infobip) account on the loopback provider, written
/// directly so the test does not depend on how accounts are validated.
async fn viber_account(client: &mut Client, world: &World, base_url: &str, key: &str) -> Uuid {
    let sender = AccountSender::VIBER_INFOBIP {
        base_url: base_url.to_string(),
        sender: "Elections".to_string(),
        approved_templates: BTreeMap::from([(
            MessagePurpose::NOTICE,
            BTreeMap::from([("en".to_string(), "tpl-notice".to_string())]),
        )]),
    };
    let check = AccountCheck {
        connected: true,
        production_access: true,
        approved_templates: BTreeMap::from([(MessagePurpose::NOTICE, vec!["en".to_string()])]),
        ..Default::default()
    };
    let account: Uuid = client
        .query_one(
            r#"
            INSERT INTO sequent_backend.messaging_account
                (tenant_id, channel, provider, name, sender, limits, status, webhook_key)
            VALUES ($1, 'VIBER', 'VIBER_INFOBIP', 'Viber', $2, '{}'::jsonb, $3, $4)
            RETURNING id
            "#,
            &[
                &world.tenant,
                &serde_json::to_value(&sender).unwrap(),
                &serde_json::to_value(&check).unwrap(),
                &key,
            ],
        )
        .await
        .unwrap()
        .get(0);
    let tx = client.transaction().await.unwrap();
    replace_credentials(
        &tx,
        &world.tenant,
        &account,
        &BTreeMap::from([(CredentialName::API_KEY, "key-1".to_string())]),
        false,
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
    account
}

/// The event sends notices on Viber through `account`. Written both where
/// the first release read it (the annotation) and where it is read now.
async fn configure_viber(client: &mut Client, world: &World, account: &Uuid) {
    let config = EventMessagingConfig {
        channels: vec![EventChannelConfig {
            channel: MessageChannel::VIBER,
            account_id: account.to_string(),
            purposes: vec![MessagePurpose::NOTICE],
            templates: vec![],
            out_of_window: OutOfWindowPolicy::DISABLED,
        }],
        ..Default::default()
    };
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
             VALUES ($1, $2, $3)",
            &[&world.tenant, &world.event, &config],
        )
        .await
        .unwrap();
}

fn notice(world: &World, key: &str, _channel: MessageChannel) -> Delivery {
    Delivery {
        tenant_id: world.tenant,
        election_event_id: Some(world.event),
        purpose: MessagePurpose::NOTICE,
        recipient: Recipient {
            voter_id: Some("voter-1".to_string()),
            channel: MessageChannel::VIBER,
            destination: "+639171234567".to_string(),
            election_ids: vec![],
            messenger_page: None,
        },
        content: Some(MessageContent {
            subject: None,
            text: "Voting opens tomorrow.".to_string(),
            html: None,
            template_parameters: vec![],
            code: None,
        }),
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
    attempts
        .into_iter()
        .map(|a| (a.channel.expect("a channel"), a.state))
        .collect()
}

/// A bulk send whose payload is kept, as `/scheduled-event` records it:
/// returns the logical key prefix of its notices.
async fn bulk_send(client: &mut Client, world: &World, payload: serde_json::Value) -> String {
    let tx = client.transaction().await.unwrap();
    let send_id = windmill::postgres::messaging::record_bulk_send(
        &tx,
        &world.tenant,
        Some(&world.event),
        Some("admin-1"),
        &payload,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    format!("send-template:{send_id}")
}

fn reminder() -> serde_json::Value {
    json!({"audience_selection": "ALL_USERS",
           "viber": {"message": "Voting opens tomorrow."}})
}

fn rate_limited() -> Reply {
    Reply::Json(
        429,
        json!({"requestError": {"serviceException": {"messageId": "TOO_MANY_REQUESTS"}}}),
    )
}

/// The backoff passes: every scheduled retry of the tenant is due.
async fn backoff_passes(client: &Client, world: &World) {
    client
        .execute(
            "UPDATE sequent_backend.message
             SET next_retry_at = now() - interval '1 second'
             WHERE tenant_id = $1 AND next_retry_at IS NOT NULL",
            &[&world.tenant],
        )
        .await
        .unwrap();
}

/// Renders every retry as the test's notice, as the stored template would.
struct SameNotice(Delivery);

#[async_trait]
impl RetryRenderer for SameNotice {
    async fn render(&self, record: &MessageRecord) -> anyhow::Result<RetryContent> {
        // Other tests' retries are left alone (still due) for their own
        // sweep.
        if record.tenant_id != self.0.tenant_id {
            anyhow::bail!("not this test's message");
        }
        Ok(RetryContent::Message(Box::new(self.0.clone())))
    }
}

fn accepted(id: &str) -> Reply {
    Reply::Json(200, json!({"messages": [{"messageId": id}]}))
}

/// WM-B1: a 429 is retried on the same channel by a later job once its
/// backoff has passed, and not before.
#[tokio::test]
async fn a_rate_limited_notice_is_sent_again_on_its_channel_by_a_later_job() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7001).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![rate_limited(), accepted("ib-2")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7001").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let key = format!(
        "{}:voter-1",
        bulk_send(&mut client, &world, reminder()).await
    );
    let delivery = notice(&world, &key, MessageChannel::VIBER);

    let first = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(first.state, MessageAttemptState::FAILED);
    let due = first.next_retry_at.expect("a retry is scheduled");
    let wait = due - chrono::Utc::now();
    assert!(wait > chrono::Duration::seconds(50) && wait <= chrono::Duration::minutes(1));
    // The same job, or another one, before the backoff has passed: nothing
    // is sent.
    deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(server.requests().len(), 1);

    // The backoff passes.
    client
        .execute(
            "UPDATE sequent_backend.message
             SET next_retry_at = now() - interval '1 second',
                 failed_at = now() - interval '2 minutes',
                 created_at = now() - interval '2 minutes'
             WHERE tenant_id = $1",
            &[&world.tenant],
        )
        .await
        .unwrap();
    let retried = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(retried.state, MessageAttemptState::ACCEPTED);
    assert_eq!(retried.channel, Some(MessageChannel::VIBER));
    assert_eq!(
        attempts(&mut client, &world, &key).await,
        vec![
            (MessageChannel::VIBER, MessageAttemptState::FAILED),
            (MessageChannel::VIBER, MessageAttemptState::ACCEPTED),
        ]
    );
    assert_eq!(server.requests().len(), 2);
}

/// WM-B2: attempts that can never be resolved don't keep newer ones from
/// being looked at.
#[tokio::test]
async fn a_backlog_of_unresolvable_attempts_does_not_starve_newer_ones() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7002).await;
    let mut client = pool.get().await.unwrap();
    // Spare answers: another test's pass may ask about the row too.
    let delivered = || {
        Reply::Json(
            200,
            json!({"results": [{"messageId": "send-template:s2:new:1",
                                "status": {"groupName": "DELIVERED"}}]}),
        )
    };
    let server = TestServer::start(vec![delivered(), delivered(), delivered()]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7002").await;
    // 500 unknown outcomes sent through the environment transport, which
    // no provider can ever resolve, older than anything else.
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, masked_destination,
                 destination_digest, logical_key, attempt, state, created_at, dispatched_at)
            SELECT $1, $2, 'SMS', 'OUTBOUND', 'NOTICE', '+63 ••• ••• 567', 'd',
                   'send-template:backlog:' || n, 1, 'UNKNOWN',
                   now() - interval '1 hour', now() - interval '1 hour'
            FROM generate_series(1, 500) AS n
            "#,
            &[&world.tenant, &world.event],
        )
        .await
        .unwrap();
    // A newer attempt whose worker stopped after handing it to the
    // environment transport, and a newer unknown one the provider can
    // resolve.
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, account_id, channel, direction, purpose,
                 masked_destination, destination_digest, logical_key, attempt, state,
                 created_at, dispatched_at)
            VALUES
                ($1, $2, NULL, 'SMS', 'OUTBOUND', 'NOTICE', '+63 ••• ••• 567', 'd',
                 'send-template:s2:stale', 1, 'QUEUED',
                 now() - interval '20 minutes', now() - interval '20 minutes'),
                ($1, $2, $3, 'VIBER', 'OUTBOUND', 'NOTICE', '+63 ••• ••• 567', 'd',
                 'send-template:s2:new', 1, 'UNKNOWN',
                 now() - interval '20 minutes', now() - interval '20 minutes')
            "#,
            &[&world.tenant, &world.event, &viber],
        )
        .await
        .unwrap();
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();

    // Each pass asks a bounded number of rows; other tests' backlogs may
    // come first.
    for _ in 0..6 {
        reconcile_messages(&mut client, &dispatcher).await.unwrap();
        if attempts(&mut client, &world, "send-template:s2:new").await
            == vec![(MessageChannel::VIBER, MessageAttemptState::DELIVERED)]
        {
            break;
        }
    }

    assert_eq!(
        attempts(&mut client, &world, "send-template:s2:stale").await,
        vec![(MessageChannel::SMS, MessageAttemptState::UNKNOWN)]
    );
    assert_eq!(
        attempts(&mut client, &world, "send-template:s2:new").await,
        vec![(MessageChannel::VIBER, MessageAttemptState::DELIVERED)]
    );
}

/// WM-B1 through the sweep: the due retry is attempt 2 on the same channel,
/// and the third failure is final.
#[tokio::test]
async fn the_sweep_retries_on_the_same_channel_up_to_three_attempts() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7003).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![rate_limited(), rate_limited(), rate_limited()]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7003").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let key = format!(
        "{}:voter-1",
        bulk_send(&mut client, &world, reminder()).await
    );
    let delivery = notice(&world, &key, MessageChannel::VIBER);
    let renderer = SameNotice(delivery.clone());

    let first = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(first.state, MessageAttemptState::FAILED);
    // Not due: the sweep sends nothing.
    sweep_messages(&mut client, &dispatcher, &renderer)
        .await
        .unwrap();
    assert_eq!(server.requests().len(), 1);

    backoff_passes(&client, &world).await;
    let (_, recorded) = sweep_messages(&mut client, &dispatcher, &renderer)
        .await
        .unwrap();
    let mine: Vec<_> = recorded
        .iter()
        .filter(|o| o.tenant_id == world.tenant)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].attempt, 2);
    assert_eq!(mine[0].channel, Some(MessageChannel::VIBER));
    // The second wait is five minutes.
    let next: Option<chrono::DateTime<chrono::Utc>> = client
        .query_one(
            "SELECT next_retry_at FROM sequent_backend.message
             WHERE tenant_id = $1 AND attempt = 2",
            &[&world.tenant],
        )
        .await
        .unwrap()
        .get(0);
    let wait = next.unwrap() - chrono::Utc::now();
    assert!(wait > chrono::Duration::minutes(4) && wait <= chrono::Duration::minutes(5));

    backoff_passes(&client, &world).await;
    sweep_messages(&mut client, &dispatcher, &renderer)
        .await
        .unwrap();
    backoff_passes(&client, &world).await;
    sweep_messages(&mut client, &dispatcher, &renderer)
        .await
        .unwrap();
    assert_eq!(
        attempts(&mut client, &world, &key).await,
        vec![
            (MessageChannel::VIBER, MessageAttemptState::FAILED),
            (MessageChannel::VIBER, MessageAttemptState::FAILED),
            (MessageChannel::VIBER, MessageAttemptState::FAILED),
        ]
    );
    assert_eq!(server.requests().len(), 3);
    let scheduled: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.message
             WHERE tenant_id = $1 AND next_retry_at IS NOT NULL",
            &[&world.tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(scheduled, 0);
}

/// A message rendered from no stored template can't be sent again: its
/// transient failure is final, with the reason.
#[tokio::test]
async fn a_message_without_a_stored_template_is_not_retried() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7004).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![rate_limited()]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7004").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let mut delivery = notice(&world, "keycloak:notice-1", MessageChannel::VIBER);
    delivery.template_alias = None;
    let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(result.next_retry_at, None);
    // R2-09: the provider's reason stays, with why it isn't retried next
    // to it.
    assert_eq!(
        result.reason.as_deref(),
        Some(
            "rate limited by the provider (TOO_MANY_REQUESTS); \
             Can't be retried: no stored template"
        )
    );
    assert_eq!(
        result.recorded[0].reason.as_deref(),
        result.reason.as_deref()
    );
}

/// WM-B3: a replayed job sends, counts and logs nothing new.
#[tokio::test]
async fn a_replayed_job_records_nothing_new() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7005).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![accepted("ib-5")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7005").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let delivery = notice(&world, "send-template:s5:voter-1", MessageChannel::VIBER);

    let first = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
    assert!(first.newly_sent());
    assert_eq!(first.recorded.len(), 1);
    for _ in 0..2 {
        let replay = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
        assert_eq!(replay.message_id, first.message_id);
        assert_eq!(replay.state, MessageAttemptState::ACCEPTED);
        // Nothing to count and nothing to log.
        assert!(!replay.newly_sent());
        assert!(replay.recorded.is_empty());
    }
    assert_eq!(server.requests().len(), 1);
}

/// WM-B8: a failure the provider reports later is retried when it may
/// pass, and final when it can't.
#[tokio::test]
async fn late_failures_are_classified_by_the_provider_code() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7006).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![accepted("ib-6a"), accepted("ib-6b")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7006").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let send = bulk_send(&mut client, &world, reminder()).await;
    let (key_a, key_b) = (format!("{send}:a"), format!("{send}:b"));
    for key in [&key_a, &key_b] {
        let result = deliver(
            &mut client,
            &dispatcher,
            &notice(&world, key, MessageChannel::VIBER),
        )
        .await
        .unwrap();
        assert_eq!(result.state, MessageAttemptState::ACCEPTED);
    }
    let account = {
        let tx = client.transaction().await.unwrap();
        let account =
            windmill::postgres::messaging::get_messaging_account(&tx, &world.tenant, &viber)
                .await
                .unwrap()
                .unwrap();
        tx.commit().await.unwrap();
        account
    };
    let report = |id: &str, error: &str| {
        messaging::webhooks::WebhookEvent::Status(messaging::webhooks::StatusReport {
            provider_message_id: id.to_string(),
            state: MessageAttemptState::FAILED,
            error_code: Some(error.to_string()),
            billing: Some(json!({"price": {"pricePerMessage": 0.01, "currency": "EUR"}})),
            at: None,
        })
    };
    windmill::services::messaging::webhooks::apply_events(
        &mut client,
        &dispatcher,
        &account,
        vec![
            report("ib-6a", "EC_ABSENT_SUBSCRIBER"),
            report("ib-6b", "EC_UNKNOWN_SUBSCRIBER"),
        ],
    )
    .await
    .unwrap();
    let row = |key: String| {
        let client = &client;
        let tenant = world.tenant;
        async move {
            client
                .query_one(
                    "SELECT failure, next_retry_at IS NOT NULL, billable
                     FROM sequent_backend.message WHERE tenant_id = $1 AND logical_key = $2",
                    &[&tenant, &key],
                )
                .await
                .unwrap()
        }
    };
    let transient = row(key_a.clone()).await;
    assert_eq!(
        transient.get::<_, Option<String>>(0).as_deref(),
        Some("TRANSIENT")
    );
    assert!(transient.get::<_, bool>(1));
    assert_eq!(transient.get::<_, Option<bool>>(2), Some(true));
    let permanent = row(key_b.clone()).await;
    assert_eq!(
        permanent.get::<_, Option<String>>(0).as_deref(),
        Some("PERMANENT")
    );
    assert!(!permanent.get::<_, bool>(1));
}

/// A failed attempt whose same-channel retry is due now.
async fn due_retry(client: &Client, world: &World, key: &str, overdue: &str) {
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, template_alias,
                 masked_destination, destination_digest, logical_key, attempt, state,
                 failure, error, next_retry_at, created_at, failed_at)
            VALUES ($1, $2, 'VIBER', 'OUTBOUND', 'NOTICE', 'reminder', '+63 ••• ••• 567', 'd',
                    $3, 1, 'FAILED', 'TRANSIENT', 'rate limited by the provider',
                    now() - $4::text::interval, now() - $4::text::interval - interval '1 minute',
                    now() - $4::text::interval - interval '1 minute')
            "#,
            &[&world.tenant, &world.event, &key, &overdue],
        )
        .await
        .unwrap();
}

/// R2-04 (WM-B2 across page boundaries): a backlog of 500 unknown outcomes
/// the provider can't answer doesn't hold up due retries, a pass asks the
/// provider a bounded number of times, and the newer unknown outcome
/// behind the backlog is reached in a later pass.
#[tokio::test]
async fn a_backlog_the_provider_cannot_answer_starves_neither_retries_nor_newer_rows() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7007).await;
    let mut client = pool.get().await.unwrap();
    let mut replies = vec![accepted("ib-7-retry")];
    // Logs lag behind sends: an empty answer leaves the outcome unknown.
    replies.extend((0..500).map(|_| Reply::Json(200, json!({"results": []}))));
    // Other tests' passes may ask about the backlog too: spare answers for
    // the newer row.
    replies.extend((0..2000).map(|_| {
        Reply::Json(
            200,
            json!({"results": [{"messageId": "send-template:s7:new:1",
                                "status": {"groupName": "DELIVERED"}}]}),
        )
    }));
    let server = TestServer::start(replies).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7007").await;
    configure_viber(&mut client, &world, &viber).await;
    // 500 unknown outcomes of the reconcilable account, each a second apart
    // so the keyset pages fall inside the backlog, older than the new one.
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, account_id, channel, direction, purpose,
                 masked_destination, destination_digest, logical_key, attempt, state,
                 created_at, dispatched_at)
            SELECT $1, $2, $3, 'VIBER', 'OUTBOUND', 'NOTICE', '+63 ••• ••• 567', 'd',
                   'send-template:s7:backlog:' || n, 1, 'UNKNOWN',
                   now() - interval '2 hours' + n * interval '1 second',
                   now() - interval '2 hours' + n * interval '1 second'
            FROM generate_series(1, 500) AS n
            "#,
            &[&world.tenant, &world.event, &viber],
        )
        .await
        .unwrap();
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, account_id, channel, direction, purpose,
                 masked_destination, destination_digest, logical_key, attempt, state,
                 created_at, dispatched_at)
            VALUES ($1, $2, $3, 'VIBER', 'OUTBOUND', 'NOTICE', '+63 ••• ••• 567', 'd',
                    'send-template:s7:new', 1, 'UNKNOWN',
                    now() - interval '20 minutes', now() - interval '20 minutes')
            "#,
            &[&world.tenant, &world.event, &viber],
        )
        .await
        .unwrap();
    due_retry(&client, &world, "send-template:s7:voter-1", "1 second").await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let renderer = SameNotice(notice(
        &world,
        "send-template:s7:voter-1",
        MessageChannel::VIBER,
    ));

    let (summary, _) = sweep_messages(&mut client, &dispatcher, &renderer)
        .await
        .unwrap();
    // The retry went out first, in the same pass, on its channel.
    assert_eq!(
        attempts(&mut client, &world, "send-template:s7:voter-1").await,
        vec![
            (MessageChannel::VIBER, MessageAttemptState::FAILED),
            (MessageChannel::VIBER, MessageAttemptState::ACCEPTED),
        ]
    );
    // The pass asked the provider a bounded number of times (this test's
    // backlog or other tests' rows alike).
    assert!(
        summary.asked <= 200,
        "one pass asked the provider {} times",
        summary.asked
    );
    // Later passes go on past the rows already asked, across the pages.
    for _ in 0..5 {
        if attempts(&mut client, &world, "send-template:s7:new").await
            == vec![(MessageChannel::VIBER, MessageAttemptState::DELIVERED)]
        {
            break;
        }
        sweep_messages(&mut client, &dispatcher, &renderer)
            .await
            .unwrap();
    }
    assert_eq!(
        attempts(&mut client, &world, "send-template:s7:new").await,
        vec![(MessageChannel::VIBER, MessageAttemptState::DELIVERED)]
    );
}

/// R2-05: one row the sweep can't read is skipped; the rest of the pass,
/// such as a due retry, still runs.
#[tokio::test]
async fn one_unreadable_row_does_not_stop_the_sweep() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7008).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![accepted("ib-8")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w7008").await;
    configure_viber(&mut client, &world, &viber).await;
    // A channel this release doesn't know.
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, masked_destination,
                 destination_digest, logical_key, attempt, state, created_at)
            VALUES ($1, $2, 'FAX', 'OUTBOUND', 'NOTICE', '-', '-', 'send-template:s8:fax', 1,
                    'QUEUED', now() - interval '10 minutes')
            "#,
            &[&world.tenant, &world.event],
        )
        .await
        .unwrap();
    due_retry(&client, &world, "send-template:s8:voter-1", "1 second").await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let renderer = SameNotice(notice(
        &world,
        "send-template:s8:voter-1",
        MessageChannel::VIBER,
    ));

    sweep_messages(&mut client, &dispatcher, &renderer)
        .await
        .expect("the pass goes on past the unreadable row");
    assert_eq!(
        attempts(&mut client, &world, "send-template:s8:voter-1").await,
        vec![
            (MessageChannel::VIBER, MessageAttemptState::FAILED),
            (MessageChannel::VIBER, MessageAttemptState::ACCEPTED),
        ]
    );
}

/// R2-07(b): an attempt handed to the provider moments ago, after waiting
/// for its turn, is still in flight: the sweep leaves it alone.
#[tokio::test]
async fn an_attempt_in_flight_after_waiting_is_not_called_stale() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x7009).await;
    let mut client = pool.get().await.unwrap();
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, masked_destination,
                 destination_digest, logical_key, attempt, state, created_at, dispatched_at)
            VALUES ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', '-', '-', 'send-template:s9:voter-1', 1,
                    'QUEUED', now() - interval '10 minutes', now() - interval '5 seconds')
            "#,
            &[&world.tenant, &world.event],
        )
        .await
        .unwrap();
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    reconcile_messages(&mut client, &dispatcher).await.unwrap();
    assert_eq!(
        attempts(&mut client, &world, "send-template:s9:voter-1").await,
        vec![(MessageChannel::SMS, MessageAttemptState::QUEUED)]
    );
}

/// R2-10: a failure reported more than an hour after the attempt isn't
/// retried, and says why next to the provider's reason.
#[tokio::test]
async fn a_failure_reported_too_late_is_not_retried() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x700a).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![accepted("ib-10")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w700a").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let key = "send-template:s10:voter-1";
    let sent = deliver(
        &mut client,
        &dispatcher,
        &notice(&world, key, MessageChannel::VIBER),
    )
    .await
    .unwrap();
    assert_eq!(sent.state, MessageAttemptState::ACCEPTED);
    client
        .execute(
            "UPDATE sequent_backend.message SET created_at = now() - interval '2 hours'
             WHERE tenant_id = $1",
            &[&world.tenant],
        )
        .await
        .unwrap();
    let account = {
        let tx = client.transaction().await.unwrap();
        let account =
            windmill::postgres::messaging::get_messaging_account(&tx, &world.tenant, &viber)
                .await
                .unwrap()
                .unwrap();
        tx.commit().await.unwrap();
        account
    };
    windmill::services::messaging::webhooks::apply_events(
        &mut client,
        &dispatcher,
        &account,
        vec![messaging::webhooks::WebhookEvent::Status(
            messaging::webhooks::StatusReport {
                provider_message_id: "ib-10".to_string(),
                state: MessageAttemptState::FAILED,
                error_code: Some("EC_ABSENT_SUBSCRIBER".to_string()),
                billing: None,
                at: None,
            },
        )],
    )
    .await
    .unwrap();
    let row = client
        .query_one(
            "SELECT state, next_retry_at IS NULL, error FROM sequent_backend.message
             WHERE tenant_id = $1 AND logical_key = $2",
            &[&world.tenant, &key],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "FAILED");
    assert!(row.get::<_, bool>(1), "no retry is scheduled");
    let error: String = row.get(2);
    assert!(
        error.starts_with("provider code EC_ABSENT_SUBSCRIBER")
            && error.ends_with("Reported too late to retry"),
        "{error}"
    );
}

/// R2-15: a replayed job finds a due retry, but the voter changed channel
/// since: nothing goes out, on either channel, and the attempt says why.
#[tokio::test]
async fn a_replay_after_a_channel_change_sends_nothing() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x700b).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![rate_limited(), accepted("ib-11")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w700b").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let key = format!(
        "{}:voter-1",
        bulk_send(&mut client, &world, reminder()).await
    );
    let key = key.as_str();
    let first = deliver(
        &mut client,
        &dispatcher,
        &notice(&world, key, MessageChannel::VIBER),
    )
    .await
    .unwrap();
    assert_eq!(first.state, MessageAttemptState::FAILED);
    backoff_passes(&client, &world).await;
    // The voter moved to SMS, with another number, before the replay.
    let mut moved = notice(&world, key, MessageChannel::SMS);
    moved.recipient.channel = MessageChannel::SMS;
    moved.recipient.destination = "+639187654321".to_string();
    deliver(&mut client, &dispatcher, &moved).await.unwrap();
    assert_eq!(server.requests().len(), 1, "nothing more was sent");
    let row = client
        .query_one(
            "SELECT count(*), max(error), bool_and(next_retry_at IS NULL)
             FROM sequent_backend.message WHERE tenant_id = $1 AND logical_key = $2",
            &[&world.tenant, &key],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 1);
    assert!(row
        .get::<_, String>(1)
        .ends_with("The voter's channel changed: not retried"));
    assert!(row.get::<_, bool>(2));
}

/// R2-16: a retry that can't be prepared again for an hour stops waiting,
/// with a reason, rather than staying due forever.
#[tokio::test]
async fn a_retry_that_cannot_be_prepared_stops_after_an_hour() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x700c).await;
    let mut client = pool.get().await.unwrap();
    due_retry(&client, &world, "send-template:s12:voter-1", "2 hours").await;
    struct Unrenderable;
    #[async_trait]
    impl RetryRenderer for Unrenderable {
        async fn render(&self, _record: &MessageRecord) -> anyhow::Result<RetryContent> {
            anyhow::bail!("the template can't be read")
        }
    }
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    sweep_messages(&mut client, &dispatcher, &Unrenderable)
        .await
        .unwrap();
    let row = client
        .query_one(
            "SELECT next_retry_at IS NULL, error FROM sequent_backend.message
             WHERE tenant_id = $1",
            &[&world.tenant],
        )
        .await
        .unwrap();
    assert!(row.get::<_, bool>(0), "the retry stopped waiting");
    let error: String = row.get(1);
    assert!(
        error.starts_with("rate limited by the provider; "),
        "the failure stays visible: {error}"
    );
}

/// The voters and encrypted attribute names a retry sees, in memory.
struct Directory {
    voters: std::collections::HashMap<String, User>,
    secret_names: HashSet<String>,
}

#[async_trait]
impl RetryDirectory for Directory {
    async fn voter(
        &self,
        _hasura_transaction: &deadpool_postgres::Transaction<'_>,
        _tenant_id: &str,
        _election_event_id: Option<&str>,
        voter_id: &str,
    ) -> anyhow::Result<Option<User>> {
        Ok(self.voters.get(voter_id).cloned())
    }

    async fn secret_attribute_names(
        &self,
        _tenant_id: &str,
        _election_event_id: &str,
    ) -> anyhow::Result<HashSet<String>> {
        Ok(self.secret_names.clone())
    }
}

fn viber_voter(id: &str) -> User {
    User {
        id: Some(id.to_string()),
        first_name: Some("Ana".to_string()),
        attributes: Some(std::collections::HashMap::from([
            (
                "sequent.read-only.message-channel".to_string(),
                vec!["VIBER".to_string()],
            ),
            (
                "sequent.read-only.message-number".to_string(),
                vec!["+639171234567".to_string()],
            ),
            ("pin".to_string(), vec!["ciphertext".to_string()]),
        ])),
        ..User::default()
    }
}

/// A failed first attempt of `key` whose retry is due, as the ledger keeps
/// it.
async fn failed_record(client: &mut Client, world: &World, key: &str) -> MessageRecord {
    due_retry(client, world, key, "1 second").await;
    client
        .execute(
            "UPDATE sequent_backend.message SET voter_id = 'voter-1'
             WHERE tenant_id = $1 AND logical_key = $2",
            &[&world.tenant, &key],
        )
        .await
        .unwrap();
    let tx = client.transaction().await.unwrap();
    let record = list_attempts(&tx, &world.tenant, key)
        .await
        .unwrap()
        .remove(0);
    tx.commit().await.unwrap();
    record
}

fn refusal(content: RetryContent) -> Option<MessageReason> {
    match content {
        RetryContent::Refused(reason) => Some(reason),
        RetryContent::Message(_) => None,
    }
}

/// R2-02, R2-03, R2-21: the production renderer sends again what the
/// administrator sent (the stored send, edited text included), never the
/// template as stored now; it refuses sends with encrypted voter
/// attributes, sends whose payload is gone, and every origin it can't
/// render again, each with its reason.
#[tokio::test]
async fn retries_are_rendered_from_what_was_sent_by_origin() {
    std::env::set_var("VOTING_PORTAL_URL", "https://vote.example.org");
    let pool = schema::pool().await;
    let world = world(&pool, 0x700d).await;
    let mut client = pool.get().await.unwrap();
    let renderer = RetryByOrigin {
        pool: Arc::new(pool.clone()),
        directory: Directory {
            voters: std::collections::HashMap::from([(
                "voter-1".to_string(),
                viber_voter("voter-1"),
            )]),
            secret_names: HashSet::from(["pin".to_string()]),
        },
    };

    // The administrator corrected the stored text before sending.
    let edited = bulk_send(
        &mut client,
        &world,
        json!({"audience_selection": "ALL_USERS", "alias": "reminder",
               "viber": {"message": "Polls close at 7 pm, {{user.first_name}}."}}),
    )
    .await;
    let record = failed_record(&mut client, &world, &format!("{edited}:voter-1")).await;
    let RetryContent::Message(delivery) = renderer.render(&record).await.unwrap() else {
        panic!("the notice is rendered again");
    };
    assert_eq!(delivery.recipient.channel, MessageChannel::VIBER);
    assert_eq!(delivery.recipient.destination, "+639171234567");
    assert_eq!(
        delivery.content.as_ref().map(|c| c.text.as_str()),
        Some("Polls close at 7 pm, Ana.")
    );
    assert_eq!(delivery.logical_key, record.logical_key.clone().unwrap());

    // A send that used encrypted voter attributes is never sent again.
    let declared = bulk_send(
        &mut client,
        &world,
        json!({"audience_selection": "ALL_USERS", "secret_attribute_names": ["pin"],
               "viber": {"message": "Your PIN is {{user.pin}}"}}),
    )
    .await;
    let record = failed_record(&mut client, &world, &format!("{declared}:voter-1")).await;
    assert_eq!(
        refusal(renderer.render(&record).await.unwrap()),
        Some(MessageReason::PROTECTED_FIELDS)
    );
    // Nor one whose template uses one without declaring it.
    let undeclared = bulk_send(
        &mut client,
        &world,
        json!({"audience_selection": "ALL_USERS",
               "viber": {"message": "Your PIN is {{lookup user.attributes \"pin\"}}"}}),
    )
    .await;
    let record = failed_record(&mut client, &world, &format!("{undeclared}:voter-1")).await;
    assert_eq!(
        refusal(renderer.render(&record).await.unwrap()),
        Some(MessageReason::PROTECTED_FIELDS)
    );
    // The send's payload is gone, or never kept.
    let record = failed_record(
        &mut client,
        &world,
        &format!("send-template:{}:voter-1", Uuid::new_v4()),
    )
    .await;
    assert_eq!(
        refusal(renderer.render(&record).await.unwrap()),
        Some(MessageReason::NO_STORED_TEMPLATE)
    );
    // Keycloak's notices are never retried by the server.
    let record = failed_record(&mut client, &world, "keycloak:notice-13").await;
    assert_eq!(
        refusal(renderer.render(&record).await.unwrap()),
        Some(MessageReason::NO_STORED_TEMPLATE)
    );
    // A voter no longer in the register has no channel.
    let gone = bulk_send(&mut client, &world, reminder()).await;
    let mut record = failed_record(&mut client, &world, &format!("{gone}:voter-1")).await;
    record.voter_id = Some("voter-gone".to_string());
    assert_eq!(
        refusal(renderer.render(&record).await.unwrap()),
        Some(MessageReason::NO_CONTACT_CHANNEL)
    );
}

/// WM-B3 through what `send_template` counts and logs for one voter: the
/// first run counts the notice under its channel and logs its attempt; a
/// replayed job counts and logs nothing.
#[tokio::test]
async fn a_replayed_bulk_send_counts_and_logs_nothing_new() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x700e).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![accepted("ib-14")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w700e").await;
    configure_viber(&mut client, &world, &viber).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let key = format!(
        "{}:voter-1",
        bulk_send(&mut client, &world, reminder()).await
    );
    let delivery = notice(&world, &key, MessageChannel::VIBER);

    let first = send_voter_notice(&mut client, &dispatcher, &delivery)
        .await
        .unwrap();
    assert_eq!(first.counted, Some(MessageChannel::VIBER));
    assert_eq!(first.to_log.len(), 1);
    assert_eq!(first.to_log[0].outcome, MessageAttemptState::ACCEPTED);
    for _ in 0..2 {
        let replay = send_voter_notice(&mut client, &dispatcher, &delivery)
            .await
            .unwrap();
        assert_eq!(replay.counted, None);
        assert!(replay.to_log.is_empty());
    }
    assert_eq!(server.requests().len(), 1);
}

/// R2-08 and R2-11: an attempt whose log entry its sender never handed
/// over is returned by the sweep to be logged; a retry the sweep sends is
/// counted in the event's statistics.
#[tokio::test]
async fn the_sweep_logs_what_a_stopped_sender_left_and_counts_its_retries() {
    let pool = schema::pool().await;
    let world = world(&pool, 0x700f).await;
    let mut client = pool.get().await.unwrap();
    let server = TestServer::start(vec![accepted("ib-15")]).await;
    let viber = viber_account(&mut client, &world, &server.base_url, "w700f").await;
    configure_viber(&mut client, &world, &viber).await;
    client
        .execute(
            r#"
            INSERT INTO sequent_backend.message
                (tenant_id, election_event_id, channel, direction, purpose, masked_destination,
                 destination_digest, logical_key, attempt, state, created_at, updated_at)
            VALUES ($1, $2, 'SMS', 'OUTBOUND', 'NOTICE', '-', '-', 'keycloak:left', 1,
                    'ACCEPTED', now() - interval '10 minutes', now() - interval '10 minutes')
            "#,
            &[&world.tenant, &world.event],
        )
        .await
        .unwrap();
    due_retry(&client, &world, "send-template:s15:voter-1", "1 second").await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let renderer = SameNotice(notice(
        &world,
        "send-template:s15:voter-1",
        MessageChannel::VIBER,
    ));
    let (_, recorded) = sweep_messages(&mut client, &dispatcher, &renderer)
        .await
        .unwrap();
    let mine: Vec<_> = recorded
        .iter()
        .filter(|o| o.tenant_id == world.tenant)
        .map(|o| (o.kind, o.outcome))
        .collect();
    assert!(mine.contains(&(MessageKind::KEYCLOAK, MessageAttemptState::ACCEPTED)));
    assert!(mine.contains(&(MessageKind::BULK_NOTICE, MessageAttemptState::ACCEPTED)));
    let sent: Option<String> = client
        .query_one(
            "SELECT statistics ->> 'num_viber_sent' FROM sequent_backend.election_event
             WHERE id = $1",
            &[&world.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(sent.as_deref(), Some("1"));
}
