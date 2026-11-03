// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![recursion_limit = "256"]

//! The shared per-account token bucket against the real schema: concurrent
//! takers on separate connections, as harvest and windmill processes are,
//! never spend more than the bucket holds, and notices never take the
//! tokens kept for codes. Each test commits under a tenant of its own.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::{Client, Pool};
use messaging::providers::Endpoints;
use messaging::rate_limit::BucketShare;
use messaging::sender::FailureKind;
use messaging::test_server::{Reply, TestServer};
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, CreateMessengerLinkRequest, CredentialName,
    EventChannelConfig, EventMessagingConfig, MessageAttemptState, MessageChannel, MessageContent,
    MessagePurpose, OutOfWindowPolicy, ReadinessBlocker,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;
use windmill::postgres::messaging::{insert_message, list_attempts, MessageRecord, NewMessage};
use windmill::postgres::rate_bucket::{take, try_take, Take, TryTake, CAPACITY_BUSY};
use windmill::services::messaging::accounts::{not_ready_reason, replace_credentials};
use windmill::services::messaging::dispatch::{
    account_for, deliver, deliver_within, ConfiguredAccountMissing, Delivery, Dispatcher, Place,
    Recipient, SendPolicy, ShareRule,
};
use windmill::services::messaging::links::create_link;
use windmill::services::messaging::links::{send_bound_code, BoundCode};
use windmill::services::messaging::reconcile::{
    sweep_messages_with, RetryContent, RetryRenderer, SweepLimits,
};
use windmill::services::messaging::webhooks::{meta_callback, WebhookOutcome};

const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";

/// Separate pools: like separate processes, no connection is shared.
const WORKERS: usize = 4;
const TAKERS_PER_WORKER: usize = 8;

fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

fn limits(rate: u32, reserved: u32) -> AccountLimits {
    AccountLimits {
        messages_per_second: Some(rate),
        otp_reserved_per_second: Some(reserved),
        allowed_calling_codes: vec![],
        otp_concurrency: None,
    }
}

/// What a passing connection check stores for an account in production
/// with approved templates for codes and notices.
fn ready() -> AccountCheck {
    AccountCheck {
        connected: true,
        production_access: true,
        approved_templates: [
            (MessagePurpose::OTP, vec!["en".to_string()]),
            (MessagePurpose::NOTICE, vec!["en".to_string()]),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    }
}

/// What a check that couldn't connect stores: the account isn't ready for
/// anything.
fn not_connected() -> AccountCheck {
    AccountCheck {
        connected: false,
        ..ready()
    }
}

/// The reason of a message the account's readiness refused at send time.
const NOT_CONNECTED_REASON: &str = "Not sent: the account isn't ready (NOT_CONNECTED)";

/// A tenant and a ready Viber account with these limits.
async fn viber_account(
    pool: &Pool,
    seed: u32,
    limits: &AccountLimits,
    base_url: &str,
) -> (Uuid, Uuid) {
    viber_account_with(pool, seed, limits, base_url, ready()).await
}

/// A tenant and a Viber account with these limits and check result.
async fn viber_account_with(
    pool: &Pool,
    seed: u32,
    limits: &AccountLimits,
    base_url: &str,
    status: AccountCheck,
) -> (Uuid, Uuid) {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    let tenant = id(seed, 1);
    let mut client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
    let sender = AccountSender::VIBER_INFOBIP {
        base_url: base_url.to_string(),
        sender: "Elections".to_string(),
        approved_templates: BTreeMap::from([(
            MessagePurpose::NOTICE,
            BTreeMap::from([("en".to_string(), "tpl-notice".to_string())]),
        )]),
    };
    let account: Uuid = client
        .query_one(
            r#"
            INSERT INTO sequent_backend.messaging_account
                (tenant_id, channel, provider, name, sender, limits, status, webhook_key)
            VALUES ($1, 'VIBER', 'VIBER_INFOBIP', 'Viber account', $2, $3, $4, $5)
            RETURNING id
            "#,
            &[
                &tenant,
                &serde_json::to_value(sender).unwrap(),
                &serde_json::to_value(limits).unwrap(),
                &serde_json::to_value(status).unwrap(),
                &format!("{seed:064x}"),
            ],
        )
        .await
        .unwrap()
        .get(0);
    let tx = client.transaction().await.unwrap();
    replace_credentials(
        &tx,
        &tenant,
        &account,
        &[(CredentialName::API_KEY, "key-1".to_string())]
            .into_iter()
            .collect(),
        false,
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
    (tenant, account)
}

/// Sets the bucket to `tokens` with no refill for an hour: what concurrent
/// takers may take is then exactly what it holds.
async fn freeze(client: &Client, account: &Uuid, tokens: f64) {
    client
        .execute(
            "INSERT INTO sequent_backend.messaging_rate_bucket (account_id, tokens, refilled_at)
             VALUES ($1, $2, now() + interval '1 hour')
             ON CONFLICT (account_id) DO UPDATE
             SET tokens = EXCLUDED.tokens, refilled_at = EXCLUDED.refilled_at",
            &[account, &tokens],
        )
        .await
        .unwrap();
}

/// Sets the bucket to `tokens`, refilling from now on.
async fn thaw(client: &Client, account: &Uuid, tokens: f64) {
    client
        .execute(
            "UPDATE sequent_backend.messaging_rate_bucket
             SET tokens = $2, refilled_at = now() WHERE account_id = $1",
            &[account, &tokens],
        )
        .await
        .unwrap();
}

async fn tokens(client: &Client, account: &Uuid) -> f64 {
    client
        .query_one(
            "SELECT tokens FROM sequent_backend.messaging_rate_bucket WHERE account_id = $1",
            &[account],
        )
        .await
        .unwrap()
        .get(0)
}

/// Every taker of every worker tries until `until`; the tokens left after
/// each take, in the order they were taken.
async fn take_concurrently(
    account: Uuid,
    limits: AccountLimits,
    share: BucketShare,
    until: Instant,
) -> Vec<f64> {
    let taken = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for _ in 0..WORKERS {
        let pool = schema::pool().await;
        for _ in 0..TAKERS_PER_WORKER {
            let client = pool.get().await.unwrap();
            let limits = limits.clone();
            let taken = taken.clone();
            handles.push(tokio::spawn(async move {
                while Instant::now() < until {
                    match try_take(&client, &account, &limits, share).await.unwrap() {
                        TryTake::Taken { tokens_left } => taken.lock().unwrap().push(tokens_left),
                        TryTake::Wait(wait) => {
                            tokio::time::sleep(wait.min(Duration::from_millis(20))).await
                        }
                        TryTake::Unthrottled => panic!("the account is throttled"),
                    }
                }
            }));
        }
    }
    for handle in handles {
        handle.await.unwrap();
    }
    let taken = taken.lock().unwrap().clone();
    taken
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_takers_never_spend_more_than_the_bucket_holds() {
    let pool = schema::pool().await;
    let limits = limits(20, 5);
    let (_, account) =
        viber_account(&pool, 0x7b0c_0001, &limits, "https://unused.example.org").await;
    let client = pool.get().await.unwrap();

    // Codes may take every token, and not one more.
    freeze(&client, &account, 12.0).await;
    let until = Instant::now() + Duration::from_millis(600);
    let taken = take_concurrently(account, limits.clone(), BucketShare::WHOLE, until).await;
    assert_eq!(taken.len(), 12);
    assert!(tokens(&client, &account).await.abs() < 1e-9);

    // Notices stop at the reserve, however many try at once.
    freeze(&client, &account, 20.0).await;
    let until = Instant::now() + Duration::from_millis(600);
    let taken = take_concurrently(
        account,
        limits.clone(),
        BucketShare::ABOVE_OTP_RESERVE,
        until,
    )
    .await;
    assert_eq!(taken.len(), 15);
    assert!(taken.iter().all(|left| *left >= 5.0), "{taken:?}");
    assert!((tokens(&client, &account).await - 5.0).abs() < 1e-9);

    // The reserve is still there for codes.
    let until = Instant::now() + Duration::from_millis(600);
    let taken = take_concurrently(account, limits, BucketShare::WHOLE, until).await;
    assert_eq!(taken.len(), 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_saturating_bulk_send_keeps_to_the_rate_and_leaves_the_reserve_to_codes() {
    let pool = schema::pool().await;
    let limits = limits(50, 10);
    let (_, account) =
        viber_account(&pool, 0x7b0c_0002, &limits, "https://unused.example.org").await;
    let client = pool.get().await.unwrap();

    let started = Instant::now();
    let bulk = tokio::spawn(take_concurrently(
        account,
        limits.clone(),
        BucketShare::ABOVE_OTP_RESERVE,
        started + Duration::from_millis(1500),
    ));
    // While the bulk send saturates the account, codes get a token at
    // once, from the reserve.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut code_waits = Vec::new();
    for _ in 0..5 {
        let asked = Instant::now();
        let taken = take(
            &client,
            &account,
            &limits,
            BucketShare::WHOLE,
            Some(asked + Duration::from_millis(500)),
        )
        .await
        .unwrap();
        assert_eq!(taken, Take::Taken);
        code_waits.push(asked.elapsed());
    }
    let bulk = bulk.await.unwrap();
    let elapsed = started.elapsed().as_secs_f64();

    // Never more than a full bucket plus what the elapsed time refilled,
    // codes included.
    let spent = bulk.len() + code_waits.len();
    assert!(
        (spent as f64) <= 50.0 + 50.0 * elapsed + 1.0,
        "{spent} tokens in {elapsed:.2}s"
    );
    // The send was saturated: it took most of what the rate allowed.
    assert!(bulk.len() as f64 >= 50.0 * 1.0, "{} notices", bulk.len());
    // Notices never dipped into the reserve.
    assert!(bulk.iter().all(|left| *left >= 10.0 - 1e-9), "{bulk:?}");
    // Codes didn't queue behind the bulk send.
    assert!(
        code_waits
            .iter()
            .all(|wait| *wait < Duration::from_millis(200)),
        "{code_waits:?}"
    );
}

#[tokio::test]
async fn accounts_without_a_rate_are_never_throttled_and_get_no_bucket() {
    let pool = schema::pool().await;
    let unlimited = AccountLimits::default();
    let (_, account) =
        viber_account(&pool, 0x7b0c_0003, &unlimited, "https://unused.example.org").await;
    let client = pool.get().await.unwrap();
    for _ in 0..100 {
        assert_eq!(
            take(
                &client,
                &account,
                &unlimited,
                BucketShare::ABOVE_OTP_RESERVE,
                None
            )
            .await
            .unwrap(),
            Take::Unthrottled
        );
    }
    let buckets: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.messaging_rate_bucket WHERE account_id = $1",
            &[&account],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(buckets, 0);
}

#[tokio::test]
async fn a_new_bucket_starts_full_and_a_waiting_taker_stops_at_its_deadline() {
    let pool = schema::pool().await;
    let limits = limits(3, 1);
    let (_, account) =
        viber_account(&pool, 0x7b0c_0004, &limits, "https://unused.example.org").await;
    let client = pool.get().await.unwrap();
    for _ in 0..3 {
        assert!(matches!(
            try_take(&client, &account, &limits, BucketShare::WHOLE)
                .await
                .unwrap(),
            TryTake::Taken { .. }
        ));
    }
    freeze(&client, &account, 0.0).await;
    let asked = Instant::now();
    let taken = take(
        &client,
        &account,
        &limits,
        BucketShare::WHOLE,
        Some(asked + Duration::from_millis(150)),
    )
    .await
    .unwrap();
    assert_eq!(taken, Take::DeadlinePassed);
    assert!(asked.elapsed() >= Duration::from_millis(150));
    assert!(asked.elapsed() < Duration::from_millis(1000));
}

/// A notice of an application result through `account`: its origin can
/// render it again, so a transient failure is retried.
fn viber_notice(tenant: Uuid, account: Uuid, logical_key: &str) -> Delivery {
    Delivery {
        tenant_id: tenant,
        election_event_id: None,
        purpose: MessagePurpose::NOTICE,
        recipient: Recipient {
            voter_id: None,
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
        template_alias: Some("application-result".to_string()),
        logical_key: logical_key.to_string(),
        expires_at: None,
        account_id: Some(account),
        template_key: None,
        provider_template: None,
    }
}

fn accepted() -> Reply {
    Reply::Json(
        200,
        json!({"messages": [{"messageId": "m-1", "status": {"groupName": "PENDING"}}]}),
    )
}

async fn attempts_of(client: &mut Client, tenant: &Uuid, logical_key: &str) -> Vec<MessageRecord> {
    let tx = client.transaction().await.unwrap();
    let attempts = list_attempts(&tx, tenant, logical_key).await.unwrap();
    tx.commit().await.unwrap();
    attempts
}

/// R5-06: a notice still waiting for capacity at its deadline is never
/// handed to the provider, and its failure is transient: the retry chain
/// applies when its origin can render it again.
#[tokio::test]
async fn a_notice_past_its_capacity_deadline_is_failed_for_a_retry_and_never_sent() {
    let pool = schema::pool().await;
    let server = TestServer::start(vec![accepted()]).await;
    let limits = limits(2, 1);
    let (tenant, account) = viber_account(&pool, 0x7b0c_0005, &limits, &server.base_url).await;
    let mut client = pool.get().await.unwrap();
    freeze(&client, &account, 0.0).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let delivery = viber_notice(tenant, account, "application:capacity-busy-1");
    let result = deliver_within(
        &mut client,
        &dispatcher,
        &delivery,
        SendPolicy {
            capacity_deadline: Some(Instant::now() + Duration::from_millis(100)),
            ..SendPolicy::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(result.reason.as_deref(), Some(CAPACITY_BUSY));
    assert!(server.requests().is_empty());
    let attempts = attempts_of(&mut client, &tenant, "application:capacity-busy-1").await;
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].state, MessageAttemptState::FAILED);
    assert_eq!(attempts[0].failure, Some(FailureKind::TRANSIENT));
    assert!(attempts[0].dispatched_at.is_none());
    assert!(attempts[0].next_retry_at.is_some());

    // Once tokens are back, the same account sends at once.
    freeze(&client, &account, 2.0).await;
    let sent = deliver_within(
        &mut client,
        &dispatcher,
        &viber_notice(tenant, account, "application:capacity-busy-2"),
        SendPolicy::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        sent.state,
        MessageAttemptState::ACCEPTED,
        "{:?}",
        sent.reason
    );
    assert_eq!(server.requests().len(), 1);
}

/// R5-06: windmill takes its token before it writes the QUEUED row, so a
/// send waiting for capacity leaves nothing for the stale sweep to fail.
#[tokio::test]
async fn a_send_waiting_for_capacity_has_no_queued_row_yet() {
    let pool = schema::pool().await;
    let server = TestServer::start(vec![accepted()]).await;
    let limits = limits(2, 0);
    let (tenant, account) = viber_account(&pool, 0x7b0c_0008, &limits, &server.base_url).await;
    let client = pool.get().await.unwrap();
    freeze(&client, &account, 0.0).await;
    let sending = tokio::spawn({
        let pool = pool.clone();
        async move {
            let mut client = pool.get().await.unwrap();
            let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
            deliver(
                &mut client,
                &dispatcher,
                &viber_notice(tenant, account, "application:waiting-1"),
            )
            .await
            .unwrap()
        }
    });
    tokio::time::sleep(Duration::from_millis(400)).await;
    let rows: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.message WHERE tenant_id = $1",
            &[&tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(rows, 0, "a row was written before the token was taken");
    thaw(&client, &account, 1.0).await;
    let sent = tokio::time::timeout(Duration::from_secs(10), sending)
        .await
        .expect("the send went on once a token was there")
        .unwrap();
    assert_eq!(
        sent.state,
        MessageAttemptState::ACCEPTED,
        "{:?}",
        sent.reason
    );
    assert_eq!(server.requests().len(), 1);
}

/// R5-01: a notice windmill sends through an account that isn't ready is
/// FAILED with the readiness blocker's reason and never handed to the
/// provider. Its failure is transient: the normal retry rules apply once
/// the account is ready.
#[tokio::test]
async fn a_notice_through_an_account_that_isnt_ready_is_failed_and_never_sent() {
    let pool = schema::pool().await;
    let server = TestServer::start(vec![accepted()]).await;
    let (tenant, account) = viber_account_with(
        &pool,
        0x7b0c_0009,
        &AccountLimits::default(),
        &server.base_url,
        not_connected(),
    )
    .await;
    let mut client = pool.get().await.unwrap();
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let result = deliver(
        &mut client,
        &dispatcher,
        &viber_notice(tenant, account, "application:not-ready-1"),
    )
    .await
    .unwrap();
    assert_eq!(result.state, MessageAttemptState::FAILED);
    assert_eq!(result.reason.as_deref(), Some(NOT_CONNECTED_REASON));
    assert_eq!(
        not_ready_reason(ReadinessBlocker::NOT_CONNECTED),
        NOT_CONNECTED_REASON
    );
    assert!(server.requests().is_empty());
    let attempts = attempts_of(&mut client, &tenant, "application:not-ready-1").await;
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].failure, Some(FailureKind::TRANSIENT));
    assert!(attempts[0].dispatched_at.is_none());
    assert!(attempts[0].next_retry_at.is_some());
}

/// Renders the due retries of one tenant's message again; any other
/// tenant's are left due (other tests share the database).
struct RetryOf {
    tenant: Uuid,
    delivery: Delivery,
}

#[async_trait::async_trait]
impl RetryRenderer for RetryOf {
    async fn render(&self, record: &MessageRecord) -> anyhow::Result<RetryContent> {
        if record.tenant_id != self.tenant {
            anyhow::bail!("another test's retry");
        }
        Ok(RetryContent::Message(Box::new(self.delivery.clone())))
    }
}

/// A failed first attempt of `delivery` whose retry is due now.
async fn due_retry(client: &mut Client, delivery: &Delivery) -> MessageRecord {
    let tx = client.transaction().await.unwrap();
    let failed = insert_message(
        &tx,
        &NewMessage {
            tenant_id: delivery.tenant_id,
            election_event_id: None,
            voter_id: None,
            account_id: delivery.account_id,
            channel: Some(delivery.recipient.channel),
            purpose: Some(delivery.purpose),
            template_alias: delivery.template_alias.clone(),
            language: delivery.language.clone(),
            masked_destination: "+63917****567".to_string(),
            destination_digest: "-".to_string(),
            destination_country: None,
            logical_key: Some(delivery.logical_key.clone()),
            attempt: 1,
            state: MessageAttemptState::FAILED,
            provider_message_id: None,
            error: Some("provider unavailable".to_string()),
            failure: Some(FailureKind::TRANSIENT),
            billing_category: None,
        },
    )
    .await
    .unwrap()
    .unwrap();
    tx.execute(
        "UPDATE sequent_backend.message SET next_retry_at = now() - interval '1 minute'
         WHERE id = $1",
        &[&failed.id],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    failed
}

async fn sweep_retry(client: &mut Client, dispatcher: &Dispatcher, renderer: &RetryOf) {
    let limits = SweepLimits {
        retry_budget: Duration::from_millis(300),
        ..SweepLimits::default()
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        sweep_messages_with(client, dispatcher, renderer, &limits),
    )
    .await
    .expect("the sweep's retries end within their budget")
    .unwrap();
}

/// R5-01: a due retry through an account that isn't ready isn't sent: it
/// stays due, later, until the account is ready; then it is sent as usual.
#[tokio::test]
async fn a_due_retry_waits_while_its_account_isnt_ready() {
    let pool = schema::pool().await;
    let server = TestServer::start(vec![accepted()]).await;
    let (tenant, account) = viber_account_with(
        &pool,
        0x7b0c_000a,
        &AccountLimits::default(),
        &server.base_url,
        not_connected(),
    )
    .await;
    let mut client = pool.get().await.unwrap();
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let delivery = viber_notice(tenant, account, "application:not-ready-retry");
    let failed = due_retry(&mut client, &delivery).await;
    let renderer = RetryOf { tenant, delivery };

    sweep_retry(&mut client, &dispatcher, &renderer).await;
    assert!(server.requests().is_empty());
    let attempts = attempts_of(&mut client, &tenant, "application:not-ready-retry").await;
    assert_eq!(attempts.len(), 1, "{attempts:?}");
    assert!(
        attempts[0].next_retry_at > failed.next_retry_at.max(Some(chrono::Utc::now())),
        "{:?}",
        attempts[0].next_retry_at
    );
    assert_eq!(attempts[0].error, failed.error);

    // Ready again: the retry goes out at its next turn.
    client
        .execute(
            "UPDATE sequent_backend.messaging_account SET status = $2 WHERE id = $1",
            &[&account, &serde_json::to_value(ready()).unwrap()],
        )
        .await
        .unwrap();
    client
        .execute(
            "UPDATE sequent_backend.message SET next_retry_at = now() - interval '1 second'
             WHERE id = $1",
            &[&failed.id],
        )
        .await
        .unwrap();
    sweep_retry(&mut client, &dispatcher, &renderer).await;
    let attempts = attempts_of(&mut client, &tenant, "application:not-ready-retry").await;
    assert_eq!(attempts.len(), 2, "{attempts:?}");
    assert_eq!(attempts[1].state, MessageAttemptState::ACCEPTED);
    assert_eq!(server.requests().len(), 1);
}

/// R5-06: a due retry whose account has no capacity within the pass's
/// retry budget isn't failed: it stays due, later, and the pass goes on.
#[tokio::test]
async fn a_due_retry_without_capacity_stays_due_and_the_pass_goes_on() {
    let pool = schema::pool().await;
    let server = TestServer::start(vec![accepted()]).await;
    let limits = limits(2, 0);
    let (tenant, account) = viber_account(&pool, 0x7b0c_000b, &limits, &server.base_url).await;
    let mut client = pool.get().await.unwrap();
    freeze(&client, &account, 0.0).await;
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let delivery = viber_notice(tenant, account, "application:busy-retry");
    let failed = due_retry(&mut client, &delivery).await;
    let renderer = RetryOf { tenant, delivery };

    let started = Instant::now();
    sweep_retry(&mut client, &dispatcher, &renderer).await;
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(server.requests().is_empty());
    let attempts = attempts_of(&mut client, &tenant, "application:busy-retry").await;
    assert_eq!(attempts.len(), 1, "{attempts:?}");
    assert_eq!(attempts[0].state, MessageAttemptState::FAILED);
    assert_eq!(attempts[0].error, failed.error);
    assert!(
        attempts[0].next_retry_at > Some(chrono::Utc::now()),
        "{:?}",
        attempts[0].next_retry_at
    );
}

const APP_SECRET: &str = "synthetic-app-secret";
const PSID: &str = "6543210";
const WEBHOOKS_URL: &str = "https://callbacks.example.org";

/// A tenant, an event offering Messenger for codes through a ready
/// Messenger account with these limits, and a pending link for code
/// 482619. Returns the account, the link's reference and its id.
async fn messenger_link(
    pool: &Pool,
    seed: u32,
    limits: &AccountLimits,
    key: &str,
) -> (Uuid, Uuid, String) {
    messenger_link_with(pool, seed, limits, key, ready()).await
}

/// [`messenger_link`] through an account with this check result.
async fn messenger_link_with(
    pool: &Pool,
    seed: u32,
    limits: &AccountLimits,
    key: &str,
    status: AccountCheck,
) -> (Uuid, Uuid, String) {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    std::env::set_var("MESSAGING_WEBHOOKS_PUBLIC_URL", WEBHOOKS_URL);
    let (tenant, event) = (id(seed, 1), id(seed, 2));
    let mut client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &tenant],
        )
        .await
        .unwrap();
    let sender = AccountSender::MESSENGER_SEND_API {
        page_id: "page-1".to_string(),
        page_name: Some("Elections".to_string()),
        page_username: Some("elections".to_string()),
        api_version: "v23.0".to_string(),
        api_base_url: None,
    };
    let account: Uuid = client
        .query_one(
            r#"
            INSERT INTO sequent_backend.messaging_account
                (tenant_id, channel, provider, name, sender, limits, status, webhook_key)
            VALUES ($1, 'MESSENGER', 'MESSENGER_SEND_API', 'Page', $2, $3, $4, $5)
            RETURNING id
            "#,
            &[
                &tenant,
                &serde_json::to_value(sender).unwrap(),
                &serde_json::to_value(limits).unwrap(),
                &serde_json::to_value(status).unwrap(),
                &key,
            ],
        )
        .await
        .unwrap()
        .get(0);
    let tx = client.transaction().await.unwrap();
    replace_credentials(
        &tx,
        &tenant,
        &account,
        &[
            (CredentialName::ACCESS_TOKEN, "page-token".to_string()),
            (CredentialName::APP_SECRET, APP_SECRET.to_string()),
        ]
        .into_iter()
        .collect(),
        false,
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
    let config = serde_json::to_value(EventMessagingConfig {
        channels: vec![EventChannelConfig {
            channel: MessageChannel::MESSENGER,
            account_id: account.to_string(),
            purposes: vec![MessagePurpose::OTP],
            templates: vec![],
            out_of_window: OutOfWindowPolicy::DISABLED,
        }],
        ..Default::default()
    })
    .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.messaging_event_config
                (tenant_id, election_event_id, config)
             VALUES ($1, $2, $3)",
            &[&tenant, &event, &config],
        )
        .await
        .unwrap();
    let created = create_link(
        &mut client,
        &CreateMessengerLinkRequest {
            tenant_id: tenant.to_string(),
            election_event_id: Some(event.to_string()),
            auth_session: "session-digest".to_string(),
            challenge: "challenge-digest".to_string(),
            code: "482619".to_string(),
            language: Some("en".to_string()),
            content: MessageContent {
                subject: None,
                text: "Your verification code is 482619.".to_string(),
                html: None,
                template_parameters: vec![],
                code: None,
            },
            expires_at: (chrono::Utc::now() + chrono::Duration::minutes(30)).to_rfc3339(),
            expected_recipient: None,
            expected_page: None,
        },
        Some(WEBHOOKS_URL),
    )
    .await
    .unwrap()
    .unwrap();
    (tenant, account, created.reference)
}

/// Get Started from `PSID` with `reference`, signed by the app secret, as
/// harvest takes it: stored and the link bound; the codes it releases are
/// returned unsent.
async fn get_started(
    client: &mut Client,
    key: &str,
    reference: &str,
) -> (WebhookOutcome, Vec<BoundCode>) {
    use hmac::{Hmac, Mac};
    let body = json!({"object": "page", "entry": [{"id": "page-1", "messaging": [
        {"sender": {"id": PSID}, "recipient": {"id": "page-1"},
         "timestamp": chrono::Utc::now().timestamp_millis(),
         "postback": {"title": "Get Started", "payload": "GET_STARTED",
                      "referral": {"ref": reference, "source": "SHORTLINK", "type": "OPEN_THREAD"}}}
    ]}]})
    .to_string();
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(APP_SECRET.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
    meta_callback(client, key, Some(&signature), body.as_bytes())
        .await
        .unwrap()
}

async fn link_state(client: &Client, tenant: &Uuid) -> (String, Option<String>) {
    let row = client
        .query_one(
            "SELECT state, page_scoped_id FROM sequent_backend.messenger_link
             WHERE tenant_id = $1",
            &[tenant],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1))
}

async fn link_code_attempts(
    client: &mut Client,
    tenant: &Uuid,
) -> Vec<(MessageAttemptState, Option<String>)> {
    client
        .query(
            "SELECT state, error FROM sequent_backend.message
             WHERE tenant_id = $1 AND logical_key LIKE 'messenger-link:%'",
            &[tenant],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| {
            let state: String = row.get(0);
            (state.parse().unwrap(), row.get(1))
        })
        .collect()
}

/// R5-07: the callback stores the events and binds the link; the code is
/// sent after, with its own capacity: no place means FAILED with the
/// capacity reason, never sent, and the link stays bound without
/// CODE_SENT; with a place the code takes from the whole bucket.
#[tokio::test]
async fn the_code_a_messenger_referral_releases_is_sent_after_the_callback_with_its_own_capacity() {
    let pool = schema::pool().await;
    let server = TestServer::start(vec![Reply::Json(200, json!({"message_id": "m_code"}))]).await;
    let dispatcher = Dispatcher::new(Endpoints {
        meta_graph: Some(server.base_url.clone()),
        ..Endpoints::default()
    })
    .unwrap();

    let limits = limits(10, 9);
    let key = format!("{:064x}", 0x7b0c_0006u64);
    let (tenant, account, reference) = messenger_link(&pool, 0x7b0c_0006, &limits, &key).await;
    let mut client = pool.get().await.unwrap();
    let (outcome, codes) = get_started(&mut client, &key, &reference).await;
    assert_eq!(outcome, WebhookOutcome::Accepted);
    // Answered with nothing sent yet: the link is bound, no attempt exists.
    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].account.id, account);
    assert_eq!(codes[0].delivery.purpose, MessagePurpose::OTP);
    assert!(link_code_attempts(&mut client, &tenant).await.is_empty());
    assert_eq!(
        link_state(&client, &tenant).await,
        ("PENDING".to_string(), Some(PSID.to_string()))
    );
    // No place within the budget.
    let sent = send_bound_code(
        &mut client,
        &dispatcher,
        &codes[0],
        SendPolicy {
            place: Place::NOT_GRANTED,
            share: ShareRule::WHOLE_BUCKET,
            ..SendPolicy::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(sent.state, MessageAttemptState::FAILED);
    assert!(server.requests().is_empty());
    assert_eq!(
        link_code_attempts(&mut client, &tenant).await,
        vec![(MessageAttemptState::FAILED, Some(CAPACITY_BUSY.to_string()))]
    );
    assert_eq!(link_state(&client, &tenant).await.0, "PENDING");

    // With a place, the code takes from the whole bucket: below the
    // notices' reserve (9 of 10 tokens), it still goes at once.
    let key = format!("{:064x}", 0x7b0c_0007u64);
    let (tenant, account, reference) = messenger_link(&pool, 0x7b0c_0007, &limits, &key).await;
    freeze(&client, &account, 1.0).await;
    let (_, codes) = get_started(&mut client, &key, &reference).await;
    send_bound_code(
        &mut client,
        &dispatcher,
        &codes[0],
        SendPolicy {
            place: Place::HELD,
            capacity_deadline: Some(Instant::now() + Duration::from_secs(5)),
            ..SendPolicy::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(server.requests().len(), 1);
    assert_eq!(
        link_code_attempts(&mut client, &tenant).await,
        vec![(MessageAttemptState::ACCEPTED, None)]
    );
    assert_eq!(link_state(&client, &tenant).await.0, "CODE_SENT");
    assert!(tokens(&client, &account).await.abs() < 1e-9);
}

/// R5-01: the code a Get Started releases through a Messenger account that
/// isn't ready is FAILED with the readiness reason and never sent.
#[tokio::test]
async fn the_code_a_referral_releases_through_an_account_that_isnt_ready_is_never_sent() {
    let pool = schema::pool().await;
    let server = TestServer::start(vec![Reply::Json(200, json!({"message_id": "m_code"}))]).await;
    let dispatcher = Dispatcher::new(Endpoints {
        meta_graph: Some(server.base_url.clone()),
        ..Endpoints::default()
    })
    .unwrap();
    let key = format!("{:064x}", 0x7b0c_000cu64);
    let (tenant, _, reference) = messenger_link_with(
        &pool,
        0x7b0c_000c,
        &AccountLimits::default(),
        &key,
        not_connected(),
    )
    .await;
    let mut client = pool.get().await.unwrap();
    let (outcome, codes) = get_started(&mut client, &key, &reference).await;
    assert_eq!(outcome, WebhookOutcome::Accepted);
    let sent = send_bound_code(&mut client, &dispatcher, &codes[0], SendPolicy::default())
        .await
        .unwrap();
    assert_eq!(sent.reason.as_deref(), Some(NOT_CONNECTED_REASON));
    assert!(server.requests().is_empty());
    assert_eq!(
        link_code_attempts(&mut client, &tenant).await,
        vec![(
            MessageAttemptState::FAILED,
            Some(NOT_CONNECTED_REASON.to_string())
        )]
    );
}

/// R5-05: an account the event's configuration names, deleted since, sends
/// nothing: one FAILED row with "No sending account for the channel",
/// never the deployment's own email or SMS sender. Nor does a test
/// message through a deleted account.
#[tokio::test]
async fn a_deleted_configured_account_never_falls_back_to_the_deployment_sender() {
    let pool = schema::pool().await;
    let (tenant, event) = (id(0x7b0c_000d, 1), id(0x7b0c_000d, 2));
    let mut client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &tenant],
        )
        .await
        .unwrap();
    let deleted = id(0x7b0c_000d, 3);
    let config = serde_json::to_value(EventMessagingConfig {
        channels: vec![EventChannelConfig {
            channel: MessageChannel::EMAIL,
            account_id: deleted.to_string(),
            purposes: vec![MessagePurpose::NOTICE],
            templates: vec![],
            out_of_window: OutOfWindowPolicy::DISABLED,
        }],
        ..Default::default()
    })
    .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.messaging_event_config
                (tenant_id, election_event_id, config)
             VALUES ($1, $2, $3)",
            &[&tenant, &event, &config],
        )
        .await
        .unwrap();
    let dispatcher = Dispatcher::new(Endpoints::default()).unwrap();
    let email = |logical_key: &str, account_id: Option<Uuid>| Delivery {
        tenant_id: tenant,
        election_event_id: account_id.is_none().then_some(event),
        purpose: MessagePurpose::NOTICE,
        recipient: Recipient {
            voter_id: None,
            channel: MessageChannel::EMAIL,
            destination: "voter@example.org".to_string(),
            election_ids: vec![],
            messenger_page: None,
        },
        content: Some(MessageContent {
            subject: Some("Voting".to_string()),
            text: "Voting opens tomorrow.".to_string(),
            html: None,
            template_parameters: vec![],
            code: None,
        }),
        language: Some("en".to_string()),
        template_alias: None,
        logical_key: logical_key.to_string(),
        expires_at: None,
        account_id,
        template_key: None,
        provider_template: None,
    };
    // The send's configuration already leaves out a channel whose account
    // is gone ("Channel not offered…"); a test message names the account
    // itself. Neither goes through the deployment's sender.
    for (delivery, reason) in [
        (
            email("configured-gone", None),
            "Channel not offered for notices",
        ),
        (
            email("account-test:gone", Some(deleted)),
            "No sending account for the channel",
        ),
    ] {
        let result = deliver(&mut client, &dispatcher, &delivery).await.unwrap();
        assert_eq!(result.state, MessageAttemptState::FAILED, "{delivery:?}");
        assert_eq!(result.reason.as_deref(), Some(reason), "{delivery:?}");
        let attempts = attempts_of(&mut client, &tenant, &delivery.logical_key).await;
        assert_eq!(attempts.len(), 1);
        assert!(attempts[0].dispatched_at.is_none());
        assert_eq!(attempts[0].account_id, None);
    }
    // Asked with the configuration as stored, the channel's account is
    // refused, never "no account" (which means the deployment's sender).
    let stored: EventMessagingConfig = serde_json::from_value(config).unwrap();
    let tx = client.transaction().await.unwrap();
    let missing = account_for(&tx, &tenant, Some(&stored), MessageChannel::EMAIL)
        .await
        .expect_err("a configured account that is gone is refused");
    tx.commit().await.unwrap();
    assert_eq!(
        missing.downcast_ref::<ConfiguredAccountMissing>(),
        Some(&ConfiguredAccountMissing {
            channel: MessageChannel::EMAIL,
            account_id: deleted.to_string(),
        })
    );
    assert_eq!(missing.to_string(), "No sending account for the channel");
}
