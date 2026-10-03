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
    MessageChannel, MessageDirection, MessagePurpose, MessengerLinkState, ProviderApproval,
    ReadinessPolicy,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::messaging::{
    expire_messenger_links, find_message_by_provider_id, get_messaging_account,
    get_messaging_account_by_webhook_key, get_messenger_link_by_reference,
    get_pending_messenger_link, insert_message, insert_messaging_account, insert_messenger_link,
    last_inbound_at, list_attempts, list_messaging_accounts, list_unresolved_messages,
    record_credentials_replaced, transition_message, update_messaging_account,
    update_messaging_account_status, update_messenger_link, AccountSettings, NewMessage,
    NewMessengerLink, StateChange,
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
        channel: MessageChannel::SMS,
        direction: MessageDirection::OUTBOUND,
        purpose: Some(MessagePurpose::NOTICE),
        template_alias: Some("reminder".to_string()),
        language: Some("en".to_string()),
        masked_destination: "+63******4567".to_string(),
        destination_digest: "digest-1".to_string(),
        destination_country: Some("63".to_string()),
        logical_key: Some(logical_key.to_string()),
        attempt,
        state: MessageAttemptState::QUEUED,
        provider_message_id: None,
        error: None,
        failure: None,
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
async fn unresolved_attempts_include_unknown_and_stale_queued_ones() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x574c).await;
    let queued = insert_message(&tx, &attempt(&world, "send-3:a", 1))
        .await
        .unwrap()
        .unwrap();
    let unknown = insert_message(&tx, &attempt(&world, "send-3:b", 1))
        .await
        .unwrap()
        .unwrap();
    transition_message(
        &tx,
        &unknown.id,
        MessageAttemptState::UNKNOWN,
        &StateChange::default(),
    )
    .await
    .unwrap()
    .unwrap();
    let accepted = insert_message(&tx, &attempt(&world, "send-3:c", 1))
        .await
        .unwrap()
        .unwrap();
    transition_message(
        &tx,
        &accepted.id,
        MessageAttemptState::ACCEPTED,
        &StateChange::default(),
    )
    .await
    .unwrap()
    .unwrap();

    let ids = |records: Vec<windmill::postgres::messaging::MessageRecord>| -> Vec<Uuid> {
        records
            .into_iter()
            .filter(|r| r.tenant_id == world.tenant)
            .map(|r| r.id)
            .collect()
    };
    let now_only_unknown = list_unresolved_messages(&tx, Utc::now() - Duration::hours(1), 100)
        .await
        .unwrap();
    assert_eq!(ids(now_only_unknown), vec![unknown.id]);
    let mut both = ids(
        list_unresolved_messages(&tx, Utc::now() + Duration::hours(1), 100)
            .await
            .unwrap(),
    );
    both.sort();
    let mut expected = vec![queued.id, unknown.id];
    expected.sort();
    assert_eq!(both, expected);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn inbound_messages_open_the_conversation_window() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x574d).await;
    let account = insert_messaging_account(&tx, &world.tenant, &sms_settings("SMS", false), "k")
        .await
        .unwrap();
    assert_eq!(
        last_inbound_at(&tx, &account.id, "digest-1").await.unwrap(),
        None
    );
    let mut inbound = attempt(&world, "unused", 1);
    inbound.account_id = Some(account.id);
    inbound.direction = MessageDirection::INBOUND;
    inbound.logical_key = None;
    inbound.purpose = None;
    inbound.state = MessageAttemptState::DELIVERED;
    insert_message(&tx, &inbound).await.unwrap().unwrap();
    assert!(last_inbound_at(&tx, &account.id, "digest-1")
        .await
        .unwrap()
        .is_some());
    tx.rollback().await.unwrap();
}

fn link(world: &World, account_id: Uuid, session: &str, reference: &str) -> NewMessengerLink {
    NewMessengerLink {
        tenant_id: world.tenant,
        election_event_id: Some(world.event),
        account_id,
        reference_digest: reference.to_string(),
        link_word_digest: format!("word-{reference}"),
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

    assert!(
        get_pending_messenger_link(&tx, &account.id, Some("ref-1"), None)
            .await
            .unwrap()
            .is_none()
    );
    let by_word = get_pending_messenger_link(&tx, &account.id, None, Some("word-ref-2"))
        .await
        .unwrap()
        .unwrap();
    update_messenger_link(
        &tx,
        &by_word.id,
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
    update_messenger_link(&tx, &by_word.id, MessengerLinkState::CONFIRMED, None, None)
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
