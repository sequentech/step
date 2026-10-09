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
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, EventChannelConfig, EventMessagingConfig,
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose, OutOfWindowPolicy,
};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::messaging::{list_attempts, MessageRecord};
use windmill::services::messaging::accounts::replace_credentials;
use windmill::services::messaging::dispatch::{deliver, Delivery, Dispatcher, Recipient};
use windmill::services::messaging::reconcile::{
    reconcile_messages, sweep_messages, RetryContent, RetryRenderer,
};

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
    attempts.into_iter().map(|a| (a.channel, a.state)).collect()
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
    let delivery = notice(&world, "send-template:s1:voter-1", MessageChannel::VIBER);

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
    assert_eq!(retried.channel, MessageChannel::VIBER);
    assert_eq!(
        attempts(&mut client, &world, "send-template:s1:voter-1").await,
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
    let server = TestServer::start(vec![Reply::Json(
        200,
        json!({"results": [{"messageId": "send-template:s2:new:1",
                            "status": {"groupName": "DELIVERED"}}]}),
    )])
    .await;
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

    reconcile_messages(&mut client, &dispatcher).await.unwrap();

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
    let delivery = notice(&world, "send-template:s3:voter-1", MessageChannel::VIBER);
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
    assert_eq!(mine[0].channel, MessageChannel::VIBER);
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
        attempts(&mut client, &world, "send-template:s3:voter-1").await,
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
    assert_eq!(
        result.reason.as_deref(),
        Some("Can't be retried: no stored template")
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
    for key in ["send-template:s6:a", "send-template:s6:b"] {
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
    let row = |key: &'static str| {
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
    let transient = row("send-template:s6:a").await;
    assert_eq!(
        transient.get::<_, Option<String>>(0).as_deref(),
        Some("TRANSIENT")
    );
    assert!(transient.get::<_, bool>(1));
    assert_eq!(transient.get::<_, Option<bool>>(2), Some(true));
    let permanent = row("send-template:s6:b").await;
    assert_eq!(
        permanent.get::<_, Option<String>>(0).as_deref(),
        Some("PERMANENT")
    );
    assert!(!permanent.get::<_, bool>(1));
}
