// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging persistence against the real schema: sent-message statistics,
//! sending accounts, the message ledger and Messenger links.

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, CredentialName, MessageAttemptState,
    MessageChannel, MessagePurpose, MessengerLinkState, ProviderApproval, ReadinessPolicy,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::messaging::{
    count_event_messages, expire_messenger_links, find_message_by_provider_id,
    get_bulk_send_payload, get_messaging_account, get_messaging_account_by_webhook_key,
    get_messenger_link_by_reference, get_pending_messenger_link, insert_message,
    insert_messaging_account, insert_messenger_link, last_get_started_at, latest_attempt_ids,
    list_attempts, list_due_retries, list_messaging_accounts, list_reconcilable_unknown,
    list_stale_queued, list_unlogged, mark_dispatched, mark_logged, record_bulk_send,
    record_credentials_replaced, record_messenger_contact, transition_message,
    update_messaging_account, update_messaging_account_status, update_messenger_link,
    AccountSettings, ChannelFilter, LedgerCount, NewMessage, NewMessengerLink, StateChange,
};
use windmill::services::election_event_statistics::update_election_event_statistics;
use windmill::services::election_statistics::update_election_statistics;

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

struct World {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
}

async fn world(tx: &Transaction<'_>, seed: u32) -> World {
    let world = World {
        tenant: id(seed, 1),
        event: id(seed, 2),
        election: id(seed, 3),
    };
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
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&world.election, &world.tenant, &world.event],
    )
    .await
    .unwrap();
    world
}

fn counters(pairs: &[(&str, i64)]) -> BTreeMap<String, i64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[tokio::test]
async fn statistics_add_per_channel_counters_and_keep_other_keys() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5747).await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET statistics = '{\"num_emails_sent\": 4, \"num_users\": 9}'::jsonb
         WHERE id = $1",
        &[&world.event],
    )
    .await
    .unwrap();

    let tenant = world.tenant.to_string();
    let event = world.event.to_string();
    update_election_event_statistics(
        &tx,
        &tenant,
        &event,
        &counters(&[("num_emails_sent", 2), ("num_whatsapp_sent", 3)]),
    )
    .await
    .unwrap();
    update_election_event_statistics(&tx, &tenant, &event, &counters(&[("num_whatsapp_sent", 1)]))
        .await
        .unwrap();
    update_election_statistics(
        &tx,
        &tenant,
        &event,
        &world.election.to_string(),
        &counters(&[("num_viber_sent", 5)]),
    )
    .await
    .unwrap();

    let event_statistics: Value = tx
        .query_one(
            "SELECT statistics FROM sequent_backend.election_event WHERE id = $1",
            &[&world.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        event_statistics,
        json!({"num_emails_sent": 6, "num_users": 9, "num_whatsapp_sent": 4})
    );
    let election_statistics: Value = tx
        .query_one(
            "SELECT statistics FROM sequent_backend.election WHERE id = $1",
            &[&world.election],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(election_statistics["num_viber_sent"], json!(5));
    tx.rollback().await.unwrap();
}

fn sms_settings(name: &str, is_default: bool) -> AccountSettings {
    AccountSettings {
        channel: MessageChannel::SMS,
        name: name.to_string(),
        sender: AccountSender::AWS_SNS {
            sender_id: Some("COMELEC".to_string()),
            origination_number: None,
            region: Some("ap-southeast-1".to_string()),
        },
        limits: AccountLimits::default(),
        provider_approval: ProviderApproval::PENDING,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default,
    }
}

#[tokio::test]
async fn accounts_keep_one_default_per_channel_and_never_store_credential_values() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5748).await;

    let first = insert_messaging_account(&tx, &world.tenant, &sms_settings("First", true), "key-a")
        .await
        .unwrap();
    let second =
        insert_messaging_account(&tx, &world.tenant, &sms_settings("Second", true), "key-b")
            .await
            .unwrap();
    let accounts = list_messaging_accounts(&tx, &world.tenant).await.unwrap();
    assert_eq!(accounts.len(), 2);
    let defaults: Vec<&str> = accounts
        .iter()
        .filter(|a| a.is_default)
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(defaults, vec!["Second"]);

    let at = Utc::now();
    record_credentials_replaced(
        &tx,
        &world.tenant,
        &first.id,
        &[
            CredentialName::AWS_ACCESS_KEY_ID,
            CredentialName::AWS_SECRET_ACCESS_KEY,
        ],
        at,
    )
    .await
    .unwrap();
    let stored: Value = tx
        .query_one(
            "SELECT credentials FROM sequent_backend.messaging_account WHERE id = $1",
            &[&first.id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        stored,
        json!({
            "AWS_ACCESS_KEY_ID": {"replaced_at": at.to_rfc3339()},
            "AWS_SECRET_ACCESS_KEY": {"replaced_at": at.to_rfc3339()},
        })
    );

    let check = AccountCheck {
        connected: true,
        production_access: false,
        reason: Some("the account is in the SMS sandbox".to_string()),
        ..Default::default()
    };
    update_messaging_account_status(&tx, &world.tenant, &second.id, &check)
        .await
        .unwrap();
    let by_key = get_messaging_account_by_webhook_key(&tx, "key-b")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_key.status, check);
    assert!(get_messaging_account(&tx, &id(0x9999, 1), &second.id)
        .await
        .unwrap()
        .is_none());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_account_cannot_switch_provider_or_channel() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5749).await;
    let account = insert_messaging_account(&tx, &world.tenant, &sms_settings("SMS", false), "k")
        .await
        .unwrap();
    let mut settings = sms_settings("SMS", false);
    settings.channel = MessageChannel::EMAIL;
    settings.sender = AccountSender::AWS_SES {
        from_address: "noreply@example.org".to_string(),
        from_name: None,
        region: None,
        notification_topic_arn: None,
    };
    assert!(
        update_messaging_account(&tx, &world.tenant, &account.id, &settings)
            .await
            .is_err()
    );
    let mut mismatch = sms_settings("Mismatch", false);
    mismatch.channel = MessageChannel::WHATSAPP;
    assert!(
        insert_messaging_account(&tx, &world.tenant, &mismatch, "k2")
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
}

fn attempt(world: &World, logical_key: &str, attempt: i32) -> NewMessage {
    NewMessage {
        tenant_id: world.tenant,
        election_event_id: Some(world.event),
        voter_id: Some("voter-1".to_string()),
        account_id: None,
        channel: Some(MessageChannel::SMS),
        purpose: Some(MessagePurpose::NOTICE),
        template_alias: Some("reminder".to_string()),
        language: Some("en".to_string()),
        masked_destination: "+63 \u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022} 567".to_string(),
        destination_digest: "digest-1".to_string(),
        destination_country: Some("PH".to_string()),
        logical_key: Some(logical_key.to_string()),
        attempt,
        state: MessageAttemptState::QUEUED,
        provider_message_id: None,
        error: None,
        failure: None,
        billing_category: None,
    }
}

#[tokio::test]
async fn a_replayed_attempt_is_not_inserted_twice() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x574a).await;
    assert!(insert_message(&tx, &attempt(&world, "send-1:voter-1", 1))
        .await
        .unwrap()
        .is_some());
    assert!(insert_message(&tx, &attempt(&world, "send-1:voter-1", 1))
        .await
        .unwrap()
        .is_none());
    assert!(insert_message(&tx, &attempt(&world, "send-1:voter-1", 2))
        .await
        .unwrap()
        .is_some());
    let attempts = list_attempts(&tx, &world.tenant, "send-1:voter-1")
        .await
        .unwrap();
    assert_eq!(
        attempts.iter().map(|a| a.attempt).collect::<Vec<_>>(),
        vec![1, 2]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn late_reports_cannot_regress_an_attempt() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x574b).await;
    let account = insert_messaging_account(&tx, &world.tenant, &sms_settings("SMS", false), "k")
        .await
        .unwrap();
    let mut new = attempt(&world, "send-2:voter-1", 1);
    new.account_id = Some(account.id);
    let message = insert_message(&tx, &new).await.unwrap().unwrap();

    let accepted = transition_message(
        &tx,
        &message.id,
        MessageAttemptState::ACCEPTED,
        &StateChange {
            provider_message_id: Some("sns-1".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(accepted.provider_message_id.as_deref(), Some("sns-1"));
    let found = find_message_by_provider_id(&tx, &account.id, "sns-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, message.id);

    transition_message(
        &tx,
        &message.id,
        MessageAttemptState::DELIVERED,
        &StateChange::default(),
    )
    .await
    .unwrap()
    .unwrap();
    for late in [
        MessageAttemptState::FAILED,
        MessageAttemptState::ACCEPTED,
        MessageAttemptState::UNKNOWN,
    ] {
        assert!(
            transition_message(&tx, &message.id, late, &StateChange::default())
                .await
                .unwrap()
                .is_none()
        );
    }
    let state: String = tx
        .query_one(
            "SELECT state FROM sequent_backend.message WHERE id = $1",
            &[&message.id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(state, "DELIVERED");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_fast_report_delivers_a_queued_attempt_and_keeps_the_provider_id() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5750).await;
    let mut new = attempt(&world, "send-9:voter-1", 1);
    new.provider_message_id = Some("provider-1".to_string());
    let message = insert_message(&tx, &new).await.unwrap().unwrap();
    // The report arrives before the send's answer was recorded.
    let delivered = transition_message(
        &tx,
        &message.id,
        MessageAttemptState::DELIVERED,
        &StateChange {
            provider_message_id: Some("send-9:voter-1:1".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .expect("QUEUED → DELIVERED");
    assert_eq!(delivered.state, MessageAttemptState::DELIVERED);
    // Step's own reference never replaces the provider's ID.
    assert_eq!(delivered.provider_message_id.as_deref(), Some("provider-1"));
    // The late answer to the send doesn't regress it.
    assert!(transition_message(
        &tx,
        &message.id,
        MessageAttemptState::ACCEPTED,
        &StateChange::default()
    )
    .await
    .unwrap()
    .is_none());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_sweep_lists_page_by_keyset_and_skip_what_they_cannot_act_on() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x574c).await;
    let viber = insert_messaging_account(
        &tx,
        &world.tenant,
        &AccountSettings {
            channel: MessageChannel::VIBER,
            name: "Viber".to_string(),
            sender: AccountSender::VIBER_INFOBIP {
                base_url: "https://api.example.org".to_string(),
                sender: "Elections".to_string(),
                approved_templates: BTreeMap::new(),
            },
            limits: AccountLimits::default(),
            provider_approval: ProviderApproval::PENDING,
            readiness: ReadinessPolicy::PROVIDER_CHECK,
            is_default: false,
        },
        "k-viber",
    )
    .await
    .unwrap();
    let mine = |records: Vec<windmill::postgres::messaging::MessageRecord>| -> Vec<Uuid> {
        records
            .into_iter()
            .filter(|r| r.tenant_id == world.tenant)
            .map(|r| r.id)
            .collect()
    };
    let mut queued = vec![];
    for n in 0..3 {
        queued.push(
            insert_message(&tx, &attempt(&world, &format!("send-3:q{n}"), 1))
                .await
                .unwrap()
                .unwrap()
                .id,
        );
    }
    // Two pages of one cover all three, in order.
    let later = Utc::now() + Duration::hours(1);
    let mut seen = vec![];
    let mut after = None;
    loop {
        let page = list_stale_queued(&tx, later, later, after, 1)
            .await
            .unwrap();
        let Some(last) = page.last else { break };
        after = Some(last);
        seen.extend(mine(page.records));
    }
    queued.sort();
    seen.sort();
    assert_eq!(seen, queued);
    assert!(mine(
        list_stale_queued(
            &tx,
            Utc::now() - Duration::hours(1),
            Utc::now() - Duration::hours(1),
            None,
            10
        )
        .await
        .unwrap()
        .records
    )
    .is_empty());

    // Unknown outcomes: only through accounts whose provider can tell, not
    // older than the unresolved limit, and not asked about recently.
    let mut reconcilable = attempt(&world, "send-3:viber", 1);
    reconcilable.account_id = Some(viber.id);
    reconcilable.channel = Some(MessageChannel::VIBER);
    let reconcilable = insert_message(&tx, &reconcilable).await.unwrap().unwrap();
    let environment = insert_message(&tx, &attempt(&world, "send-3:env", 1))
        .await
        .unwrap()
        .unwrap();
    for id in [reconcilable.id, environment.id] {
        transition_message(
            &tx,
            &id,
            MessageAttemptState::UNKNOWN,
            &StateChange::default(),
        )
        .await
        .unwrap()
        .unwrap();
    }
    let providers = vec!["VIBER_INFOBIP".to_string()];
    let unknown = |newer_than, asked_before| {
        let providers = providers.clone();
        let tx = &tx;
        async move {
            list_reconcilable_unknown(tx, &providers, newer_than, asked_before, None, 10)
                .await
                .unwrap()
                .records
        }
    };
    let day_ago = Utc::now() - Duration::hours(24);
    assert_eq!(
        mine(unknown(day_ago, Utc::now()).await),
        vec![reconcilable.id]
    );
    assert!(mine(unknown(Utc::now() + Duration::minutes(1), Utc::now()).await).is_empty());
    tx.execute(
        "UPDATE sequent_backend.message SET last_reconciled_at = now() WHERE id = $1",
        &[&reconcilable.id],
    )
    .await
    .unwrap();
    assert!(mine(unknown(day_ago, Utc::now() - Duration::minutes(5)).await).is_empty());

    // Due retries.
    let failed = insert_message(&tx, &attempt(&world, "send-3:retry", 1))
        .await
        .unwrap()
        .unwrap();
    transition_message(
        &tx,
        &failed.id,
        MessageAttemptState::FAILED,
        &StateChange {
            next_retry_at: Some(Utc::now() + Duration::minutes(1)),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(mine(
        list_due_retries(&tx, Utc::now(), None, 10)
            .await
            .unwrap()
            .records
    )
    .is_empty());
    assert_eq!(
        mine(
            list_due_retries(&tx, Utc::now() + Duration::minutes(2), None, 10)
                .await
                .unwrap()
                .records
        ),
        vec![failed.id]
    );
    // The next attempt is the retry: the earlier one stops waiting.
    insert_message(&tx, &attempt(&world, "send-3:retry", 2))
        .await
        .unwrap()
        .unwrap();
    assert!(mine(
        list_due_retries(&tx, Utc::now() + Duration::minutes(2), None, 10)
            .await
            .unwrap()
            .records
    )
    .is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn only_a_queued_attempt_not_yet_dispatched_is_dispatched() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5751).await;
    let message = insert_message(&tx, &attempt(&world, "send-5:voter-1", 1))
        .await
        .unwrap()
        .unwrap();
    assert!(mark_dispatched(&tx, &message.id).await.unwrap());
    assert!(!mark_dispatched(&tx, &message.id).await.unwrap());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn ledger_counts_are_per_message_and_flag_unresolved_ones() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5752).await;
    // One message failed once, then was accepted: counted once, accepted.
    let first = insert_message(&tx, &attempt(&world, "send-6:a", 1))
        .await
        .unwrap()
        .unwrap();
    transition_message(
        &tx,
        &first.id,
        MessageAttemptState::FAILED,
        &StateChange::default(),
    )
    .await
    .unwrap();
    let second = insert_message(&tx, &attempt(&world, "send-6:a", 2))
        .await
        .unwrap()
        .unwrap();
    transition_message(
        &tx,
        &second.id,
        MessageAttemptState::ACCEPTED,
        &StateChange::default(),
    )
    .await
    .unwrap();
    // Two unknown outcomes, one older than a day.
    for key in ["send-6:b", "send-6:c"] {
        let message = insert_message(&tx, &attempt(&world, key, 1))
            .await
            .unwrap()
            .unwrap();
        transition_message(
            &tx,
            &message.id,
            MessageAttemptState::UNKNOWN,
            &StateChange::default(),
        )
        .await
        .unwrap();
    }
    tx.execute(
        "UPDATE sequent_backend.message SET created_at = now() - interval '25 hours'
         WHERE tenant_id = $1 AND logical_key = 'send-6:c'",
        &[&world.tenant],
    )
    .await
    .unwrap();
    // A voter with no contact channel: a row with no channel, never
    // counted as an SMS (or email) failure.
    let mut no_channel = attempt(&world, "send-6:d", 1);
    no_channel.channel = None;
    no_channel.state = MessageAttemptState::FAILED;
    no_channel.error = Some("No contact channel".to_string());
    let no_channel = insert_message(&tx, &no_channel).await.unwrap().unwrap();
    assert_eq!(no_channel.channel, None);
    let unresolved_before = Utc::now() - Duration::hours(24);
    let counts = count_event_messages(&tx, &world.tenant, &world.event, None, unresolved_before)
        .await
        .unwrap();
    let count = |outcome: &str| LedgerCount {
        channel: Some(MessageChannel::SMS),
        outcome: outcome.to_string(),
        messages: 1,
    };
    assert_eq!(
        counts,
        vec![
            count("ACCEPTED"),
            count("UNKNOWN"),
            count("UNRESOLVED"),
            LedgerCount {
                channel: None,
                outcome: "FAILED".to_string(),
                messages: 1
            },
        ]
    );

    // R4-03: Logs filter on the same messages as the counts, by the latest
    // attempt's current state: the message accepted on attempt 2 is not a
    // failure, and its failed attempt 1 isn't listed.
    let ids = |channel, outcome: Option<&'static str>| {
        let tx = &tx;
        let world = &world;
        async move {
            latest_attempt_ids(
                tx,
                &world.tenant,
                &world.event,
                channel,
                outcome,
                unresolved_before,
            )
            .await
            .unwrap()
        }
    };
    assert_eq!(
        ids(
            ChannelFilter::Channel(MessageChannel::SMS),
            Some("ACCEPTED")
        )
        .await,
        vec![second.id]
    );
    assert!(
        ids(ChannelFilter::Channel(MessageChannel::SMS), Some("FAILED"))
            .await
            .is_empty()
    );
    assert_eq!(
        ids(ChannelFilter::NoChannel, Some("failed")).await,
        vec![no_channel.id]
    );
    assert_eq!(ids(ChannelFilter::Any, Some("UNRESOLVED")).await.len(), 1);
    assert_eq!(ids(ChannelFilter::Any, None).await.len(), 4);

    // R4-07: the latest attempt of each message, for the drawer's last
    // failure: attempt 1 of "send-6:a" is superseded.
    let latest: Vec<Uuid> = tx
        .query(
            "SELECT id FROM sequent_backend.message_latest_attempt
             WHERE tenant_id = $1 AND voter_id = 'voter-1' AND state = 'FAILED'",
            &[&world.tenant],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(latest, vec![no_channel.id]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn only_get_started_opens_the_messenger_window() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x574d).await;
    let account = insert_messaging_account(&tx, &world.tenant, &sms_settings("SMS", false), "k")
        .await
        .unwrap();
    assert_eq!(
        last_get_started_at(&tx, &account.id, "digest-1")
            .await
            .unwrap(),
        None
    );
    let earlier = Utc::now() - Duration::hours(2);
    let later = Utc::now() - Duration::minutes(1);
    record_messenger_contact(&tx, &world.tenant, &account.id, "digest-1", later)
        .await
        .unwrap();
    // A replayed, older event doesn't move the window back.
    record_messenger_contact(&tx, &world.tenant, &account.id, "digest-1", earlier)
        .await
        .unwrap();
    let at = last_get_started_at(&tx, &account.id, "digest-1")
        .await
        .unwrap()
        .unwrap();
    assert!((at - later).num_milliseconds().abs() < 1);
    // Incoming messages are no longer kept: the migration deleted them.
    let inbound: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.message WHERE direction = 'INBOUND'",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(inbound, 0);
    tx.rollback().await.unwrap();
}

fn link(world: &World, account_id: Uuid, session: &str, reference: &str) -> NewMessengerLink {
    NewMessengerLink {
        tenant_id: world.tenant,
        election_event_id: Some(world.event),
        account_id,
        reference_digest: reference.to_string(),
        auth_session_digest: session.to_string(),
        challenge_digest: "challenge-1".to_string(),
        encrypted_payload: vec![1, 2, 3],
        language: Some("en".to_string()),
        expires_at: Utc::now() + Duration::minutes(5),
    }
}

#[tokio::test]
async fn a_new_link_replaces_the_session_links_and_deletes_their_codes() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x574e).await;
    let account = insert_messaging_account(&tx, &world.tenant, &sms_settings("SMS", false), "k")
        .await
        .unwrap();
    let first = insert_messenger_link(&tx, &link(&world, account.id, "session-1", "ref-1"))
        .await
        .unwrap();
    insert_messenger_link(&tx, &link(&world, account.id, "session-2", "ref-other"))
        .await
        .unwrap();
    insert_messenger_link(&tx, &link(&world, account.id, "session-1", "ref-2"))
        .await
        .unwrap();

    let replaced = get_messenger_link_by_reference(&tx, &world.tenant, "ref-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replaced.id, first.id);
    assert_eq!(replaced.state, MessengerLinkState::REPLACED);
    assert_eq!(replaced.encrypted_payload, None);
    let other = get_messenger_link_by_reference(&tx, &world.tenant, "ref-other")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(other.state, MessengerLinkState::PENDING);

    assert!(get_pending_messenger_link(&tx, &account.id, "ref-1")
        .await
        .unwrap()
        .is_none());
    let pending = get_pending_messenger_link(&tx, &account.id, "ref-2")
        .await
        .unwrap()
        .unwrap();
    update_messenger_link(
        &tx,
        &pending.id,
        MessengerLinkState::CODE_SENT,
        Some("psid-1"),
        None,
    )
    .await
    .unwrap();
    let sent = get_messenger_link_by_reference(&tx, &world.tenant, "ref-2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sent.page_scoped_id.as_deref(), Some("psid-1"));
    assert_eq!(sent.encrypted_payload, None);
    update_messenger_link(&tx, &pending.id, MessengerLinkState::CONFIRMED, None, None)
        .await
        .unwrap();
    let confirmed = get_messenger_link_by_reference(&tx, &world.tenant, "ref-2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(confirmed.encrypted_payload, None);

    assert_eq!(
        expire_messenger_links(&tx, Utc::now() + Duration::minutes(10))
            .await
            .unwrap(),
        1
    );
    tx.rollback().await.unwrap();
}

/// R2-08: an attempt whose log entry was never handed over is listed for
/// the sweep once the sender's grace has passed, and no more once marked.
#[tokio::test]
async fn attempts_left_unlogged_are_listed_until_logged() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5760).await;
    let mut queued = attempt(&world, "send-7:queued", 1);
    queued.logical_key = Some("send-7:queued".to_string());
    insert_message(&tx, &queued).await.unwrap().unwrap();
    let mut failed = attempt(&world, "send-7:failed", 1);
    failed.state = MessageAttemptState::FAILED;
    let failed = insert_message(&tx, &failed).await.unwrap().unwrap();
    assert_eq!(failed.logged_at, None);
    let mine = |page: windmill::postgres::messaging::LedgerPage| -> Vec<Uuid> {
        page.records
            .into_iter()
            .filter(|r| r.tenant_id == world.tenant)
            .map(|r| r.id)
            .collect()
    };
    // Still within the sender's grace: not listed.
    assert!(mine(
        list_unlogged(&tx, Utc::now() - Duration::minutes(5), None, 100)
            .await
            .unwrap()
    )
    .is_empty());
    // An outcome, unlogged, past the grace: listed; a queued one isn't.
    assert_eq!(
        mine(
            list_unlogged(&tx, Utc::now() + Duration::minutes(1), None, 100)
                .await
                .unwrap()
        ),
        vec![failed.id]
    );
    mark_logged(&tx, &[failed.id]).await.unwrap();
    assert!(mine(
        list_unlogged(&tx, Utc::now() + Duration::minutes(1), None, 100)
            .await
            .unwrap()
    )
    .is_empty());
    tx.rollback().await.unwrap();
}

/// R2-02: what an administrator sent is kept as a stopped, archived
/// scheduled event whose ID is the send's ID.
#[tokio::test]
async fn a_bulk_send_keeps_what_was_sent() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x5761).await;
    let payload = json!({"audience_selection": "ALL_USERS",
                         "sms": {"message": "Polls close at 7 pm, not 6 pm."}});
    let send_id = record_bulk_send(
        &tx,
        &world.tenant,
        Some(&world.event),
        Some("admin-1"),
        &payload,
    )
    .await
    .unwrap();
    let stored = get_bulk_send_payload(&tx, &world.tenant, &send_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored["sms"]["message"], "Polls close at 7 pm, not 6 pm.");
    assert_eq!(stored["send_id"], send_id.to_string());
    let row = tx
        .query_one(
            "SELECT stopped_at IS NOT NULL AND archived_at IS NOT NULL, event_processor
             FROM sequent_backend.scheduled_event WHERE id = $1",
            &[&send_id],
        )
        .await
        .unwrap();
    assert!(row.get::<_, bool>(0), "never picked up by the scheduler");
    assert_eq!(row.get::<_, String>(1), "SEND_TEMPLATE");
    // Another tenant can't read it.
    assert_eq!(
        get_bulk_send_payload(&tx, &Uuid::new_v4(), &send_id)
            .await
            .unwrap(),
        None
    );
    tx.rollback().await.unwrap();
}
