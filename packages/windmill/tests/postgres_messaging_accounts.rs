// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending accounts against the real schema and a loopback provider:
//! candidate credentials, identity changes, deletion while in use and
//! WhatsApp number-change signals (plan §2.7). Each test works in a
//! transaction it rolls back.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use messaging::destination::Destination;
use messaging::providers::Endpoints;
use messaging::test_server::{Reply, TestServer};
use messaging::webhooks::NumberChange as NumberChangeSignal;
use sequent_core::types::messaging::{
    purpose_readiness, AccountCheck, AccountSender, CredentialName, MessageChannel, MessagePurpose,
    NumberChange, ProviderApproval, ReadinessBlocker, ReadinessPolicy,
};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::messaging::{
    get_messaging_account, insert_messaging_account, update_messaging_account,
    update_messaging_account_status, AccountSettings,
};
use windmill::services::messaging::accounts::{
    check_account, delete_account, flag_number_change, replace_checked_credentials,
    replace_credentials, runtime_account, send_readiness, CredentialsOutcome, DeleteOutcome,
};
use windmill::services::messaging::keys::{tenant_key, TenantKey};

const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";

fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

struct World {
    tenant: Uuid,
    event: Uuid,
}

async fn world(tx: &Transaction<'_>, seed: u32) -> World {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    let world = World {
        tenant: id(seed, 1),
        event: id(seed, 2),
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
    world
}

fn whatsapp(phone_number_id: &str) -> AccountSettings {
    AccountSettings {
        channel: MessageChannel::WHATSAPP,
        name: "WhatsApp".to_string(),
        sender: AccountSender::WHATSAPP_CLOUD_API {
            business_account_id: "waba-1".to_string(),
            phone_number_id: phone_number_id.to_string(),
            display_phone_number: "+63 2 8123 4567".to_string(),
            display_name: None,
            api_version: "v23.0".to_string(),
            api_base_url: None,
        },
        limits: Default::default(),
        provider_approval: ProviderApproval::CONFIRMED,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    }
}

/// The deployment address sends the adapter to the loopback provider.
fn endpoints(server: &TestServer) -> Endpoints {
    Endpoints {
        meta_graph: Some(server.base_url.clone()),
        ..Default::default()
    }
}

async fn stored_token(tx: &Transaction<'_>, world: &World, account: &Uuid) -> String {
    let account = get_messaging_account(tx, &world.tenant, account)
        .await
        .unwrap()
        .unwrap();
    runtime_account(tx, &account)
        .await
        .unwrap()
        .credential(CredentialName::ACCESS_TOKEN)
        .unwrap()
        .to_string()
}

/// BC-F11: a replacement is checked before it is saved. A candidate that
/// doesn't connect is refused with the check's reason and leaves the stored
/// credential, and its record, as they were; one that connects replaces it
/// and its check becomes the account's status.
#[tokio::test]
async fn a_bad_candidate_credential_is_refused_and_the_old_one_kept() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc001).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c001-key")
        .await
        .unwrap();
    let token = |value: &str| BTreeMap::from([(CredentialName::ACCESS_TOKEN, value.to_string())]);
    replace_credentials(&tx, &world.tenant, &account.id, &token("old-token"), false)
        .await
        .unwrap();
    let before = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();

    let refusing = TestServer::start(vec![Reply::Json(401, json!({"error": {"code": 190}}))]).await;
    let outcome = replace_checked_credentials(
        &tx,
        &world.tenant,
        &account.id,
        &token("bad-token"),
        false,
        &endpoints(&refusing),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        outcome,
        CredentialsOutcome::Refused(
            "The provider answered with status 401; \
             The new credentials didn't connect: the previous ones are kept"
                .to_string()
        )
    );
    // The check used the candidate, and nothing was written.
    assert_eq!(
        refusing.requests()[0].header("authorization"),
        Some("Bearer bad-token")
    );
    assert_eq!(stored_token(&tx, &world, &account.id).await, "old-token");
    let after = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.credentials, before.credentials);
    assert_eq!(after.status, before.status);

    // A candidate that connects, with a token Meta says can send (R5-11),
    // replaces it; the next read uses it.
    let accepting = TestServer::start(vec![
        Reply::Json(200, json!({"id": "pn-1"})),
        Reply::Json(
            200,
            json!({"data": {"is_valid": true, "expires_at": 0,
                            "scopes": ["whatsapp_business_messaging"]}}),
        ),
    ])
    .await;
    let outcome = replace_checked_credentials(
        &tx,
        &world.tenant,
        &account.id,
        &token("new-token"),
        false,
        &endpoints(&accepting),
    )
    .await
    .unwrap()
    .unwrap();
    let CredentialsOutcome::Replaced(replaced, Some(check)) = outcome else {
        panic!("replaced after a check: {outcome:?}");
    };
    assert_eq!(replaced.replaced, vec![CredentialName::ACCESS_TOKEN]);
    assert!(check.connected);
    assert_eq!(stored_token(&tx, &world, &account.id).await, "new-token");
    let after = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert!(after.status.connected);
    tx.rollback().await.unwrap();
}

/// AP-B3, R5-09: a new WhatsApp number or business account makes the
/// account unchecked: its last check names the old identity (a check
/// stored before checks named theirs gets the old one), so nothing is
/// ready, at config save or at send time, until it is checked as it is
/// now. The readiness column keeps the values every binary reads, and the
/// number-change signals stay. Other edits keep the check as it is.
#[tokio::test]
async fn an_identity_change_makes_the_account_unchecked_until_the_check_runs() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc002).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c002-key")
        .await
        .unwrap();
    replace_credentials(
        &tx,
        &world.tenant,
        &account.id,
        &BTreeMap::from([(CredentialName::ACCESS_TOKEN, "token".to_string())]),
        false,
    )
    .await
    .unwrap();
    let mut ready = ready_check();
    let signal = NumberChange {
        masked_old_number: "+63 ••• ••• 567".to_string(),
        reported_at: "2026-10-09T00:00:00Z".to_string(),
        ..Default::default()
    };
    ready.details.number_changes = vec![signal.clone()];
    update_messaging_account_status(&tx, &world.tenant, &account.id, &ready)
        .await
        .unwrap();
    let readiness = |account: &windmill::postgres::messaging::MessagingAccount| {
        purpose_readiness(
            &account.sender.capabilities(account.channel).unwrap(),
            account.provider_approval,
            Some(&account.sender.identity()),
            &account.status,
            MessagePurpose::OTP,
            Some("en"),
        )
    };

    // Renaming isn't an identity change.
    let renamed = update_messaging_account(
        &tx,
        &world.tenant,
        &account.id,
        &AccountSettings {
            name: "Renamed".to_string(),
            ..whatsapp("pn-1")
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(renamed.status, ready);
    assert!(readiness(&renamed).is_ready());
    assert_eq!(send_readiness(&renamed, MessagePurpose::OTP), Ok(()));

    let moved = update_messaging_account(&tx, &world.tenant, &account.id, &whatsapp("pn-2"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.readiness, ReadinessPolicy::PROVIDER_CHECK);
    let stored_readiness: String = tx
        .query_one(
            "SELECT readiness FROM sequent_backend.messaging_account WHERE id = $1",
            &[&account.id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(stored_readiness, "PROVIDER_CHECK");
    assert_eq!(
        moved.status.details.identity,
        Some(whatsapp("pn-1").sender.identity())
    );
    assert_eq!(moved.status.details.number_changes, vec![signal.clone()]);
    assert_eq!(
        readiness(&moved).blockers,
        vec![ReadinessBlocker::UNCHECKED]
    );
    assert_eq!(
        send_readiness(&moved, MessagePurpose::OTP),
        Err(ReadinessBlocker::UNCHECKED)
    );

    // Saving again before the check keeps it unchecked.
    let saved = update_messaging_account(&tx, &world.tenant, &account.id, &whatsapp("pn-2"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        send_readiness(&saved, MessagePurpose::OTP),
        Err(ReadinessBlocker::UNCHECKED)
    );

    let server = TestServer::start(vec![Reply::Json(200, json!({"id": "pn-2"}))]).await;
    let checked = check_account(&tx, &world.tenant, &account.id, &endpoints(&server))
        .await
        .unwrap()
        .unwrap();
    assert!(checked.stored && checked.check.connected);
    assert_eq!(
        server.requests()[0].path.split('?').next(),
        Some("/v23.0/pn-2")
    );
    let checked = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        checked.status.details.identity,
        Some(whatsapp("pn-2").sender.identity())
    );
    assert_eq!(checked.status.details.number_changes, vec![signal]);
    assert!(!readiness(&checked)
        .blockers
        .contains(&ReadinessBlocker::UNCHECKED));
    tx.rollback().await.unwrap();
}

/// An account an event's messaging settings use isn't deleted.
#[tokio::test]
async fn an_account_in_use_is_not_deleted() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc003).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c003-key")
        .await
        .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.messaging_event_config (tenant_id, election_event_id, config)
         VALUES ($1, $2, $3)",
        &[
            &world.tenant,
            &world.event,
            &json!({"version": 1, "channels": [{"channel": "WHATSAPP",
                "account_id": account.id.to_string(), "purposes": ["OTP"], "templates": []}],
                "election_channels": {}}),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        delete_account(&tx, &world.tenant, &account.id)
            .await
            .unwrap(),
        DeleteOutcome::InUse(vec![world.event])
    );
    assert!(get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .is_some());
    tx.execute(
        "UPDATE sequent_backend.messaging_event_config
         SET config = '{\"version\": 1, \"channels\": [], \"election_channels\": {}}'
         WHERE election_event_id = $1",
        &[&world.event],
    )
    .await
    .unwrap();
    assert_eq!(
        delete_account(&tx, &world.tenant, &account.id)
            .await
            .unwrap(),
        DeleteOutcome::Deleted
    );
    assert_eq!(
        delete_account(&tx, &world.tenant, &account.id)
            .await
            .unwrap(),
        DeleteOutcome::NotFound
    );
    tx.rollback().await.unwrap();
}

/// BC-F19: a number change flags the voters the account sent to at the
/// old number, found by the keyed digest; only masked numbers are kept,
/// and the readiness and check are untouched.
#[tokio::test]
async fn a_number_change_flags_the_voters_at_the_old_number() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc004).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c004-key")
        .await
        .unwrap();
    let ready = AccountCheck {
        connected: true,
        production_access: true,
        ..Default::default()
    };
    update_messaging_account_status(&tx, &world.tenant, &account.id, &ready)
        .await
        .unwrap();
    let key = tenant_key(&tx, &world.tenant, TenantKey::Destination)
        .await
        .unwrap();
    let old = Destination::parse(MessageChannel::WHATSAPP, "+639171234567").unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.message
             (tenant_id, election_event_id, voter_id, account_id, channel, direction,
              masked_destination, destination_digest, state)
         VALUES ($1, $2, 'voter-1', $3, 'WHATSAPP', 'OUTBOUND', $4, $5, 'DELIVERED')",
        &[
            &world.tenant,
            &world.event,
            &account.id,
            &old.masked(),
            &old.digest(&key).unwrap(),
        ],
    )
    .await
    .unwrap();
    let account = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    let change = flag_number_change(
        &tx,
        &account,
        &NumberChangeSignal {
            old_number: "639171234567".to_string(),
            new_number: Some("639177654321".to_string()),
            at: None,
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(change.voter_ids, vec!["voter-1".to_string()]);
    assert_eq!(change.masked_old_number, old.masked());
    assert!(!change.masked_old_number.contains("1234567"));
    assert!(change
        .masked_new_number
        .as_deref()
        .is_some_and(|new| new.ends_with("321") && !new.contains("7654321")));
    let flagged = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(flagged.status.details.number_changes, vec![change]);
    assert!(flagged.status.connected);
    assert_eq!(flagged.readiness, ReadinessPolicy::PROVIDER_CHECK);
    tx.rollback().await.unwrap();
}

/// Regression (R7-04): a callback whose storage failed is answered 503 and
/// the provider sends it again, so the same number change arrives twice.
/// It is flagged once.
#[tokio::test]
async fn a_number_change_sent_again_is_flagged_once() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc005).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c005-key")
        .await
        .unwrap();
    let signal = NumberChangeSignal {
        old_number: "639171234567".to_string(),
        new_number: Some("639177654321".to_string()),
        at: Some(chrono::Utc::now()),
    };
    let account = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    let first = flag_number_change(&tx, &account, &signal).await.unwrap();
    assert!(first.is_some());
    let again = flag_number_change(&tx, &account, &signal).await.unwrap();
    assert_eq!(again, None, "the retry adds nothing");
    let flagged = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(flagged.status.details.number_changes, vec![first.unwrap()]);
    tx.rollback().await.unwrap();
}

fn ready_check() -> AccountCheck {
    AccountCheck {
        connected: true,
        production_access: true,
        approved_templates: BTreeMap::from([(MessagePurpose::OTP, vec!["en".to_string()])]),
        checked_at: Some("2026-10-10T00:00:00Z".to_string()),
        ..Default::default()
    }
}

/// R5-03 regression: a check that read the account before an identity
/// change describes the old number. Stored after the change, it made the
/// new number ready on the old one's check. It is dropped instead.
#[tokio::test]
async fn a_check_that_started_before_an_identity_change_is_not_stored() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc005).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c005-key")
        .await
        .unwrap();
    // The check reads the account as it is: number pn-1.
    let read_at_start = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    // Meanwhile an administrator moves it to pn-2.
    let moved = update_messaging_account(&tx, &world.tenant, &account.id, &whatsapp("pn-2"))
        .await
        .unwrap()
        .unwrap();
    // The check of pn-1 finishes.
    let _ =
        windmill::services::messaging::accounts::store_check(&tx, &read_at_start, ready_check())
            .await;
    let after = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after.status, moved.status,
        "a check of pn-1 was stored for pn-2"
    );
    tx.rollback().await.unwrap();
}

/// R5-03/R5-12 regression: a number-change signal stored while a check
/// ran was overwritten by the check's copy of the signals it read at its
/// start. The row's signals are kept.
#[tokio::test]
async fn a_check_keeps_the_number_changes_flagged_while_it_ran() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc006).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c006-key")
        .await
        .unwrap();
    let read_at_start = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    let change = flag_number_change(
        &tx,
        &read_at_start,
        &NumberChangeSignal {
            old_number: "639171234567".to_string(),
            new_number: None,
            at: None,
        },
    )
    .await
    .unwrap()
    .unwrap();
    let _ =
        windmill::services::messaging::accounts::store_check(&tx, &read_at_start, ready_check())
            .await;
    let after = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.status.details.number_changes, vec![change]);
    tx.rollback().await.unwrap();
}

/// R5-12 regression: two number-change signals for one account processed
/// at the same time each read the stored signals and wrote their own list:
/// one was lost. Both are kept.
#[tokio::test]
async fn number_changes_flagged_at_the_same_time_are_both_kept() {
    let pool = schema::pool().await;
    let mut setup = pool.get().await.unwrap();
    let tx = setup.transaction().await.unwrap();
    let world = world(&tx, 0xc007).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c007-key")
        .await
        .unwrap();
    // The tenant's destination key, created once.
    tenant_key(&tx, &world.tenant, TenantKey::Destination)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let signal = |number: &str| NumberChangeSignal {
        old_number: number.to_string(),
        new_number: None,
        at: None,
    };

    let mut first_client = pool.get().await.unwrap();
    let first = first_client.transaction().await.unwrap();
    flag_number_change(&first, &account, &signal("639171111111"))
        .await
        .unwrap()
        .unwrap();
    // The second signal is processed while the first isn't committed.
    let second = {
        let pool = pool.clone();
        let account = account.clone();
        tokio::spawn(async move {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            flag_number_change(&tx, &account, &signal("639172222222"))
                .await
                .unwrap()
                .unwrap();
            tx.commit().await.unwrap();
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    first.commit().await.unwrap();
    second.await.unwrap();

    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let after = get_messaging_account(&tx, &world.tenant, &account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after.status.details.number_changes.len(),
        2,
        "{:?}",
        after.status
    );
    tx.rollback().await.unwrap();
}

/// R5-11 regression: a replacement token that reads the number but lacks
/// the permission to send replaced a working one. Meta candidates must
/// pass `debug_token`: valid, and with the sending permissions.
#[tokio::test]
async fn a_candidate_token_that_cannot_send_is_refused() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc008).await;
    let account = insert_messaging_account(&tx, &world.tenant, &whatsapp("pn-1"), "c008-key")
        .await
        .unwrap();
    let token = |value: &str| BTreeMap::from([(CredentialName::ACCESS_TOKEN, value.to_string())]);
    replace_credentials(&tx, &world.tenant, &account.id, &token("old-token"), false)
        .await
        .unwrap();
    for debug_token in [
        // Reads, but can't send.
        json!({"data": {"is_valid": true, "expires_at": 0,
                        "scopes": ["whatsapp_business_management"]}}),
        // Expired or revoked.
        json!({"data": {"is_valid": false, "expires_at": 0, "scopes": []}}),
    ] {
        let server = TestServer::start(vec![
            Reply::Json(200, json!({"id": "pn-1"})),
            Reply::Json(200, debug_token),
        ])
        .await;
        let outcome = replace_checked_credentials(
            &tx,
            &world.tenant,
            &account.id,
            &token("reading-token"),
            false,
            &endpoints(&server),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            matches!(outcome, CredentialsOutcome::Refused(_)),
            "{outcome:?}"
        );
        assert_eq!(stored_token(&tx, &world, &account.id).await, "old-token");
    }
    tx.rollback().await.unwrap();
}

/// R5-05 regression: a delete that ran while a configuration save naming
/// the account was in flight saw no reference and deleted the account, so
/// the saved configuration named a deleted account. The delete now waits
/// for the save and is refused.
#[tokio::test]
async fn a_delete_during_a_config_save_that_uses_the_account_is_refused() {
    let pool = schema::pool().await;
    let mut setup = pool.get().await.unwrap();
    let tx = setup.transaction().await.unwrap();
    let world = world(&tx, 0xc009).await;
    let sms = AccountSettings {
        channel: MessageChannel::SMS,
        name: "SMS".to_string(),
        sender: AccountSender::AWS_SNS {
            sender_id: Some("Elections".to_string()),
            origination_number: None,
            region: None,
        },
        limits: Default::default(),
        provider_approval: ProviderApproval::CONFIRMED,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    };
    let account = insert_messaging_account(&tx, &world.tenant, &sms, "c009-key")
        .await
        .unwrap();
    update_messaging_account_status(&tx, &world.tenant, &account.id, &ready_check())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let config: sequent_core::types::messaging::EventMessagingConfig =
        serde_json::from_value(json!({"version": 1, "channels": [{"channel": "SMS",
            "account_id": account.id.to_string(), "purposes": ["NOTICE"], "templates": []}],
            "election_channels": {}}))
        .unwrap();

    let mut saving_client = pool.get().await.unwrap();
    let saving = saving_client.transaction().await.unwrap();
    let saved = windmill::services::messaging::config::save_event_messaging_config_with(
        &saving,
        &world.tenant,
        &world.event,
        &config,
        None,
        None,
    )
    .await
    .unwrap();
    assert!(
        matches!(
            saved,
            windmill::services::messaging::config::SaveOutcome::Saved(_)
        ),
        "{saved:?}"
    );
    let deleting = {
        let pool = pool.clone();
        let tenant = world.tenant;
        let id = account.id;
        tokio::spawn(async move {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let outcome = delete_account(&tx, &tenant, &id).await.unwrap();
            tx.commit().await.unwrap();
            outcome
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    saving.commit().await.unwrap();
    assert_eq!(
        deleting.await.unwrap(),
        DeleteOutcome::InUse(vec![world.event])
    );
}

/// R5-10: after an account's public sender changes, the events whose
/// settings send through it get a projection naming the new sender, which
/// harvest publishes to their realms. Before, it was published only when
/// the event's settings were saved again.
#[tokio::test]
async fn a_sender_change_gives_the_events_using_the_account_their_new_projection() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc00a).await;
    let sns = |sender_id: &str| AccountSettings {
        channel: MessageChannel::SMS,
        name: "SMS".to_string(),
        sender: AccountSender::AWS_SNS {
            sender_id: Some(sender_id.to_string()),
            origination_number: None,
            region: None,
        },
        limits: Default::default(),
        provider_approval: ProviderApproval::CONFIRMED,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    };
    let account = insert_messaging_account(&tx, &world.tenant, &sns("Elections"), "c00a-key")
        .await
        .unwrap();
    update_messaging_account_status(&tx, &world.tenant, &account.id, &ready_check())
        .await
        .unwrap();
    let config: sequent_core::types::messaging::EventMessagingConfig =
        serde_json::from_value(json!({"version": 1, "channels": [{"channel": "SMS",
            "account_id": account.id.to_string(), "purposes": ["NOTICE"], "templates": []}],
            "election_channels": {}}))
        .unwrap();
    let saved = windmill::services::messaging::config::save_event_messaging_config_with(
        &tx,
        &world.tenant,
        &world.event,
        &config,
        None,
        None,
    )
    .await
    .unwrap();
    assert!(
        matches!(
            saved,
            windmill::services::messaging::config::SaveOutcome::Saved(_)
        ),
        "{saved:?}"
    );

    update_messaging_account(&tx, &world.tenant, &account.id, &sns("Votes"))
        .await
        .unwrap()
        .unwrap();
    let projections = windmill::services::messaging::config::projections_using_account(
        &tx,
        &world.tenant,
        &account.id,
    )
    .await
    .unwrap();
    assert_eq!(projections.len(), 1);
    assert_eq!(projections[0].0, world.event);
    assert_eq!(
        projections[0].1.channels[0].sender_label.as_deref(),
        Some("Votes")
    );
    tx.rollback().await.unwrap();
}

/// BC-F13 (T2 gap): the voters a Page change affects are those connected
/// to the account's current Page in the realms of the events whose
/// settings send through the account. Voters on another Page, and another
/// tenant's voters on the same Page, are not counted; a voter is counted
/// once. Keeping the Page, or a non-Messenger account, affects nobody.
#[tokio::test]
async fn a_page_change_counts_the_voters_connected_to_the_old_page() {
    use sequent_core::services::keycloak::get_event_realm;
    use sequent_core::types::keycloak::MESSENGER_PAGE_ATTR_NAME;
    use windmill::services::messaging::accounts::messenger_page_change_impact;

    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xc00b).await;
    let page = |page_id: &str| AccountSettings {
        channel: MessageChannel::MESSENGER,
        name: "Page".to_string(),
        sender: AccountSender::MESSENGER_SEND_API {
            page_id: page_id.to_string(),
            page_name: Some("Elections".to_string()),
            page_username: None,
            api_version: "v23.0".to_string(),
            api_base_url: None,
        },
        limits: Default::default(),
        provider_approval: ProviderApproval::CONFIRMED,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    };
    let account = insert_messaging_account(&tx, &world.tenant, &page("page-1"), "c00b-key")
        .await
        .unwrap();
    windmill::postgres::messaging::upsert_messaging_event_config(
        &tx,
        &world.tenant,
        &world.event,
        &json!({"version": 1, "channels": [{"channel": "MESSENGER",
            "account_id": account.id.to_string(), "purposes": ["OTP"], "templates": []}],
            "election_channels": {}}),
        None,
    )
    .await
    .unwrap();

    // Keycloak's tables, in a schema of this transaction.
    tx.batch_execute(
        "CREATE SCHEMA keycloak_c00b;
         SET LOCAL search_path TO keycloak_c00b, public;
         CREATE TABLE realm (id varchar(36) PRIMARY KEY, name varchar(255) UNIQUE);
         CREATE TABLE user_entity (id varchar(36) PRIMARY KEY, realm_id varchar(255) NOT NULL);
         CREATE TABLE user_attribute (
             id serial PRIMARY KEY, name varchar(255) NOT NULL, value varchar(255),
             user_id varchar(36) NOT NULL);",
    )
    .await
    .unwrap();
    let event_realm = get_event_realm(&world.tenant.to_string(), &world.event.to_string());
    let other_tenant_realm =
        get_event_realm(&id(0xc00b, 9).to_string(), &id(0xc00b, 10).to_string());
    for (realm_id, name) in [("r1", &event_realm), ("r2", &other_tenant_realm)] {
        tx.execute(
            "INSERT INTO realm (id, name) VALUES ($1, $2)",
            &[&realm_id, name],
        )
        .await
        .unwrap();
    }
    for (user, realm_id, pages) in [
        ("on-page-1", "r1", vec!["page-1"]),
        // Counted once, whatever its attribute rows.
        ("twice-on-page-1", "r1", vec!["page-1", "page-1"]),
        ("on-page-2", "r1", vec!["page-2"]),
        ("not-connected", "r1", vec![]),
        ("other-tenant", "r2", vec!["page-1"]),
    ] {
        tx.execute(
            "INSERT INTO user_entity (id, realm_id) VALUES ($1, $2)",
            &[&user, &realm_id],
        )
        .await
        .unwrap();
        for page_id in pages {
            tx.execute(
                "INSERT INTO user_attribute (name, value, user_id) VALUES ($1, $2, $3)",
                &[&MESSENGER_PAGE_ATTR_NAME, &page_id, &user],
            )
            .await
            .unwrap();
        }
    }

    let impact = |new_page: Option<&'static str>| {
        messenger_page_change_impact(&tx, &tx, &world.tenant, &account.id, new_page)
    };
    let moved = impact(Some("page-3")).await.unwrap().unwrap();
    assert_eq!(moved.affected_voters, 2);
    assert_eq!(moved.election_event_ids, vec![world.event]);
    let kept = impact(Some(" page-1 ")).await.unwrap().unwrap();
    assert_eq!(kept.affected_voters, 0);
    assert_eq!(kept.election_event_ids, vec![world.event]);
    assert!(
        messenger_page_change_impact(&tx, &tx, &world.tenant, &Uuid::new_v4(), Some("page-3"))
            .await
            .unwrap()
            .is_none()
    );
    let sms = insert_messaging_account(
        &tx,
        &world.tenant,
        &AccountSettings {
            channel: MessageChannel::SMS,
            name: "SMS".to_string(),
            sender: AccountSender::AWS_SNS {
                sender_id: Some("Elections".to_string()),
                origination_number: None,
                region: None,
            },
            ..page("page-1")
        },
        "c00b-sms",
    )
    .await
    .unwrap();
    let not_messenger =
        messenger_page_change_impact(&tx, &tx, &world.tenant, &sms.id, Some("page-3"))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(not_messenger.affected_voters, 0);
    tx.rollback().await.unwrap();
}
