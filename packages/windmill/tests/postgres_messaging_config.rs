// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What protects the messaging configuration, against the real schema: the
//! event configuration lives in its own table, written only after
//! validation (BC-F10); account addresses take their credentials with them
//! only when they are entered again (BC-F2); and removed providers and
//! readiness policies are never written again.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use sequent_core::types::messaging::{
    AccountLimits, AccountSender, CredentialName, EventMessagingConfig, MessageChannel,
    ProviderApproval, ReadinessPolicy,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use windmill::postgres::messaging::{
    get_messaging_event_config, insert_messaging_account, update_messaging_account,
    update_messaging_account_reentering, AccountSettings, ENDPOINT_CREDENTIALS_REASON,
    ENDPOINT_HTTPS_REASON,
};
use windmill::services::messaging::config::{
    get_event_messaging_config, save_event_messaging_config_as, SaveOutcome,
};

fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

struct World {
    tenant: Uuid,
    event: Uuid,
}

async fn world(tx: &Transaction<'_>, seed: u32) -> World {
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

fn viber(base_url: &str) -> AccountSettings {
    AccountSettings {
        channel: MessageChannel::VIBER,
        name: "Viber".to_string(),
        sender: AccountSender::VIBER_INFOBIP {
            base_url: base_url.to_string(),
            sender: "Elections".to_string(),
            approved_templates: BTreeMap::new(),
        },
        limits: AccountLimits::default(),
        provider_approval: ProviderApproval::PENDING,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    }
}

fn sms() -> AccountSettings {
    AccountSettings {
        channel: MessageChannel::SMS,
        name: "Texts".to_string(),
        sender: AccountSender::AWS_SNS {
            sender_id: Some("Elections".to_string()),
            origination_number: None,
            region: Some("eu-west-1".to_string()),
        },
        limits: AccountLimits::default(),
        provider_approval: ProviderApproval::PENDING,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    }
}

#[tokio::test]
async fn the_event_configuration_is_read_from_and_saved_to_its_table_only() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xb001).await;
    // A first-release annotation, written around the validation: never read.
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET annotations = jsonb_build_object('messaging:config', $2::text)
         WHERE id = $1",
        &[
            &world.event,
            &json!({"version": 1, "channels": [{"channel": "SMS", "account_id": "x"}]}).to_string(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        get_event_messaging_config(&tx, &world.tenant, &world.event)
            .await
            .unwrap(),
        None
    );

    let config = EventMessagingConfig::default();
    for _ in 0..2 {
        assert!(matches!(
            save_event_messaging_config_as(
                &tx,
                &world.tenant,
                &world.event,
                &config,
                Some("admin-1")
            )
            .await
            .unwrap(),
            SaveOutcome::Saved(_)
        ));
    }
    let stored = get_messaging_event_config(&tx, &world.tenant, &world.event)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.version, 2, "each save is counted");
    assert_eq!(stored.updated_by.as_deref(), Some("admin-1"));
    assert_eq!(
        get_event_messaging_config(&tx, &world.tenant, &world.event)
            .await
            .unwrap(),
        Some(config.clone())
    );
    // The annotation was left as it was.
    let annotation: Option<String> = tx
        .query_one(
            "SELECT annotations ->> 'messaging:config' FROM sequent_backend.election_event
             WHERE id = $1",
            &[&world.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(annotation.unwrap().contains("\"x\""));

    // An invalid configuration writes nothing; another tenant's event is
    // unknown.
    let invalid: EventMessagingConfig = serde_json::from_value(json!({
        "version": 1, "channels": [{"channel": "SMS", "account_id": "missing"}],
    }))
    .unwrap();
    assert!(matches!(
        save_event_messaging_config_as(&tx, &world.tenant, &world.event, &invalid, None)
            .await
            .unwrap(),
        SaveOutcome::Invalid(_)
    ));
    assert_eq!(
        get_messaging_event_config(&tx, &world.tenant, &world.event)
            .await
            .unwrap()
            .unwrap()
            .version,
        2
    );
    assert_eq!(
        save_event_messaging_config_as(&tx, &Uuid::new_v4(), &world.event, &config, None)
            .await
            .unwrap(),
        SaveOutcome::UnknownEvent
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn sends_skip_a_channel_whose_account_was_deleted() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xb002).await;
    let account = insert_messaging_account(&tx, &world.tenant, &sms(), "b002-key")
        .await
        .unwrap();
    // As stored by a save that was valid then.
    tx.execute(
        "INSERT INTO sequent_backend.messaging_event_config
             (tenant_id, election_event_id, config)
         VALUES ($1, $2, $3)",
        &[
            &world.tenant,
            &world.event,
            &json!({"version": 1, "channels": [
                {"channel": "SMS", "account_id": account.id.to_string(), "purposes": ["NOTICE"]},
            ]}),
        ],
    )
    .await
    .unwrap();
    let sendable = get_event_messaging_config(&tx, &world.tenant, &world.event)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sendable.channels.len(), 1);
    tx.execute(
        "DELETE FROM sequent_backend.messaging_account WHERE id = $1",
        &[&account.id],
    )
    .await
    .unwrap();
    let sendable = get_event_messaging_config(&tx, &world.tenant, &world.event)
        .await
        .unwrap()
        .unwrap();
    assert!(sendable.channels.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_new_address_needs_the_credentials_again_and_https() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xb003).await;
    let account = insert_messaging_account(
        &tx,
        &world.tenant,
        &viber("https://a.api.infobip.com"),
        "b003-key",
    )
    .await
    .unwrap();
    let moved = viber("https://attacker.example.org");
    assert_eq!(
        update_messaging_account(&tx, &world.tenant, &account.id, &moved)
            .await
            .unwrap_err()
            .to_string(),
        ENDPOINT_CREDENTIALS_REASON
    );
    // Renaming keeps the address, and needs nothing.
    let mut renamed = viber("https://a.api.infobip.com/");
    renamed.name = "Viber (Manila)".to_string();
    assert!(
        update_messaging_account(&tx, &world.tenant, &account.id, &renamed)
            .await
            .unwrap()
            .is_some()
    );
    // With the API key entered again in the same change.
    let updated = update_messaging_account_reentering(
        &tx,
        &world.tenant,
        &account.id,
        &moved,
        &BTreeSet::from([CredentialName::API_KEY]),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.sender, moved.sender);

    for settings in [viber("http://a.api.infobip.com"), viber("ftp://x")] {
        assert_eq!(
            insert_messaging_account(&tx, &world.tenant, &settings, "b003-other")
                .await
                .unwrap_err()
                .to_string(),
            ENDPOINT_HTTPS_REASON
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn removed_providers_and_policies_are_never_written_again() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0xb004).await;
    for sender in [
        json!({"provider": "CONSOLE"}),
        json!({"provider": "HTTP_API", "send": {"url": "https://gateway.example.org/send"}}),
    ] {
        let settings = AccountSettings {
            sender: serde_json::from_value(sender).unwrap(),
            ..sms()
        };
        assert_eq!(
            insert_messaging_account(&tx, &world.tenant, &settings, "b004-removed")
                .await
                .unwrap_err()
                .to_string(),
            "Unsupported provider: replace this account"
        );
    }
    let confirmed = AccountSettings {
        readiness: ReadinessPolicy::ADMIN_CONFIRMED,
        ..sms()
    };
    let account = insert_messaging_account(&tx, &world.tenant, &confirmed, "b004-key")
        .await
        .unwrap();
    assert_eq!(account.readiness, ReadinessPolicy::PROVIDER_CHECK);
    let account = update_messaging_account(&tx, &world.tenant, &account.id, &confirmed)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.readiness, ReadinessPolicy::PROVIDER_CHECK);

    // A stored account of a removed provider still loads.
    tx.execute(
        "UPDATE sequent_backend.messaging_account
         SET provider = 'CONSOLE', sender = '{\"provider\": \"CONSOLE\"}',
             readiness = 'ADMIN_CONFIRMED'
         WHERE id = $1",
        &[&account.id],
    )
    .await
    .unwrap();
    let listed = windmill::postgres::messaging::list_messaging_accounts(&tx, &world.tenant)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].readiness, ReadinessPolicy::ADMIN_CONFIRMED);
    tx.rollback().await.unwrap();
}
