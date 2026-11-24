// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Portable package account registration and event bindings against PostgreSQL.
//! This test uses only its own synthetic tenants. It never opens a provider.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use sequent_core::election_config::portable_messaging::PortableMessagingAccounts;
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, EventMessagingConfig, MessageChannel, ProviderApproval,
    ReadinessPolicy,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use uuid::Uuid;
use windmill::postgres::messaging::{
    get_messaging_event_config, insert_messaging_account, list_messaging_accounts, AccountSettings,
};
use windmill::services::messaging::{
    config::{
        import_event_messaging_config, preflight_import_event_messaging_config, ImportOutcome,
        MessagingConfigPermission,
    },
    portable_registry,
};

const SHA: &str = "0000000000000000000000000000000000000000000000000000000000000078";

fn definitions() -> PortableMessagingAccounts {
    let bytes = serde_json::to_vec(&json!({
        "format": "sequent.tenant-messaging-accounts/1",
        "accounts": [
            {"reference":"whatsapp","name":"Demo WhatsApp","sender":{
                "provider":"WHATSAPP_CLOUD_API","business_account_id":"10001",
                "phone_number_id":"10002","display_phone_number":"+12025550100","api_version":"v23.0"
            }},
            {"reference":"viber","name":"Demo Viber","sender":{
                "provider":"VIBER_INFOBIP","base_url":"https://api.infobip.com","sender":"Demo Viber"
            }},
            {"reference":"messenger","name":"Demo Messenger","sender":{
                "provider":"MESSENGER_SEND_API","page_id":"10003","api_version":"v23.0"
            }},
            {"reference":"sms","name":"Demo SMS","sender":{
                "provider":"AWS_SNS","sender_id":"DemoPoll","region":"us-east-1"
            }},
            {"reference":"email","name":"Demo Email","sender":{
                "provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"
            }}
        ]
    })).unwrap();
    PortableMessagingAccounts::parse(&bytes).unwrap()
}

fn config() -> Value {
    json!({
        "version": 1,
        "channels": [
            {"channel":"WHATSAPP","account_id":"package:whatsapp","purposes":[]},
            {"channel":"VIBER","account_id":"package:viber","purposes":[]},
            {"channel":"MESSENGER","account_id":"package:messenger","purposes":[]},
            {"channel":"SMS","account_id":"package:sms","purposes":[]},
            {"channel":"EMAIL","account_id":"package:email","purposes":[]}
        ]
    })
}

async fn seed(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
}

// One dedicated schema-test binary/transaction stream: this intentionally
// exercises the registry's NOWAIT table lock without unrelated test writers.
#[tokio::test]
async fn portable_registry_preview_apply_and_rebind_never_send_or_modify_existing_accounts() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tenant = Uuid::from_u128(101);
    let second = Uuid::from_u128(102);
    let event = Uuid::from_u128(201);
    let second_event = Uuid::from_u128(202);
    let definitions = definitions();
    let tx = client.transaction().await.unwrap();
    seed(&tx, tenant, event).await;
    seed(&tx, second, second_event).await;
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let preview = portable_registry::plan(&tx, tenant, SHA, &definitions)
        .await
        .unwrap();
    let other_preview = portable_registry::plan(&tx, second, SHA, &definitions)
        .await
        .unwrap();
    assert_eq!(preview.bindings.len(), 5);
    assert!(preview.bindings.iter().all(|(reference, binding)| binding
        .existing_fingerprint
        .is_none()
        && binding.account_id != other_preview.bindings[reference].account_id));
    let projected = portable_registry::account_summaries(&tx, tenant, &definitions, &preview)
        .await
        .unwrap();
    assert_eq!(projected.len(), 5);
    assert!(projected
        .iter()
        .all(|entry| entry.check == AccountCheck::default()));
    assert!(list_messaging_accounts(&tx, &tenant)
        .await
        .unwrap()
        .is_empty());
    assert!(list_messaging_accounts(&tx, &second)
        .await
        .unwrap()
        .is_empty());
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let installed = portable_registry::apply(&tx, tenant, SHA, &definitions, &preview)
        .await
        .unwrap();
    assert!(installed.bindings.iter().all(|(reference, binding)| binding
        .existing_fingerprint
        .is_some()
        && binding.account_id == preview.bindings[reference].account_id));
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let observed = portable_registry::require_installed(&tx, tenant, SHA, &definitions, &preview)
        .await
        .unwrap();
    assert_eq!(observed, installed);
    let bound = observed.bind(&config()).unwrap();
    let approved = MessagingConfigPermission::PERMITTED;
    let denied = MessagingConfigPermission::NOT_PERMITTED;
    let mut would_send = bound.clone();
    would_send["channels"][4]["purposes"] = json!(["OTP", "NOTICE"]);
    assert!(matches!(
        import_event_messaging_config(&tx, &tenant, &event, &bound, &Default::default(), denied)
            .await
            .unwrap(),
        ImportOutcome::NotImported(_)
    ));
    assert!(matches!(
        import_event_messaging_config(
            &tx,
            &tenant,
            &event,
            &would_send,
            &Default::default(),
            approved
        )
        .await
        .unwrap(),
        ImportOutcome::NotImported(_)
    ));
    assert!(get_messaging_event_config(&tx, &tenant, &event)
        .await
        .unwrap()
        .is_none());
    preflight_import_event_messaging_config(
        &tx,
        &tenant,
        &[],
        &bound,
        &Default::default(),
        approved,
    )
    .await
    .unwrap();
    let imported =
        import_event_messaging_config(&tx, &tenant, &event, &bound, &Default::default(), approved)
            .await
            .unwrap();
    let ImportOutcome::Imported(public) = imported else {
        panic!("unchecked account selections should import without any sendable purposes");
    };
    assert!(
        public.channels.is_empty(),
        "never publish an unchecked path to voters"
    );
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let persisted = get_messaging_event_config(&tx, &tenant, &event)
        .await
        .unwrap()
        .unwrap();
    let persisted: EventMessagingConfig = serde_json::from_value(persisted.config).unwrap();
    let before = list_messaging_accounts(&tx, &tenant).await.unwrap();
    let expected_names = definitions
        .accounts
        .iter()
        .map(|account| account.name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        before
            .iter()
            .map(|account| account.name.as_str())
            .collect::<BTreeSet<_>>(),
        expected_names
    );
    assert_eq!(persisted.channels.len(), 5);
    assert!(persisted.channels.iter().all(|channel| {
        channel.purposes.is_empty()
            && before.iter().any(|account| {
                account.id.to_string() == channel.account_id
                    && account.channel == channel.channel
                    && account.credentials.is_empty()
                    && !account.is_default
                    && account.status == AccountCheck::default()
            })
    }));
    // A repeated apply after a fresh preview reuses all five actual rows.
    let repeated = portable_registry::plan(&tx, tenant, SHA, &definitions)
        .await
        .unwrap();
    assert_eq!(repeated, installed);
    assert_eq!(
        portable_registry::apply(&tx, tenant, SHA, &definitions, &repeated)
            .await
            .unwrap(),
        repeated
    );
    assert_eq!(list_messaging_accounts(&tx, &tenant).await.unwrap(), before);
    assert!(list_messaging_accounts(&tx, &second)
        .await
        .unwrap()
        .is_empty());
    tx.commit().await.unwrap();

    // An ordinary admin creates one exact recipient-side donor after preview.
    // Applying the stale preview may neither rebind to it nor add other rows.
    let tx = client.transaction().await.unwrap();
    let email = definitions.accounts.last().unwrap();
    let donor = insert_messaging_account(
        &tx,
        &second,
        &AccountSettings {
            channel: MessageChannel::EMAIL,
            name: email.name.to_uppercase(),
            sender: email.account_sender(),
            limits: AccountLimits::default(),
            readiness: ReadinessPolicy::PROVIDER_CHECK,
            provider_approval: ProviderApproval::PENDING,
            is_default: true,
        },
        "synthetic-portable-registry-donor-key",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    assert!(
        portable_registry::apply(&tx, second, SHA, &definitions, &other_preview)
            .await
            .unwrap_err()
            .to_string()
            .contains("preview the configuration package again")
    );
    assert_eq!(
        list_messaging_accounts(&tx, &second).await.unwrap(),
        vec![donor.clone()]
    );
    tx.rollback().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let fresh = portable_registry::plan(&tx, second, SHA, &definitions)
        .await
        .unwrap();
    assert_eq!(fresh.bindings["package:email"].account_id, donor.id);
    portable_registry::apply(&tx, second, SHA, &definitions, &fresh)
        .await
        .unwrap();
    let second_accounts = list_messaging_accounts(&tx, &second).await.unwrap();
    assert_eq!(second_accounts.len(), 5);
    assert_eq!(
        second_accounts
            .iter()
            .find(|account| account.id == donor.id),
        Some(&donor)
    );
    assert!(second_accounts
        .iter()
        .all(|account| before.iter().all(|first| first.id != account.id)));
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.messaging_account SET name = 'demo email', updated_at = updated_at + interval '1 second' WHERE id = $1",
        &[&donor.id],
    ).await.unwrap();
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    assert!(
        portable_registry::require_installed(&tx, second, SHA, &definitions, &fresh)
            .await
            .unwrap_err()
            .to_string()
            .contains("preview the configuration package again")
    );
    assert_eq!(
        list_messaging_accounts(&tx, &second).await.unwrap().len(),
        5
    );
    assert!(get_messaging_event_config(&tx, &second, &second_event)
        .await
        .unwrap()
        .is_none());
    for actual_tenant in [tenant, second] {
        let counts = tx
            .query_one(
                "SELECT
                (SELECT count(*) FROM sequent_backend.message WHERE tenant_id = $1),
                (SELECT count(*) FROM sequent_backend.secret WHERE tenant_id = $1)",
                &[&actual_tenant],
            )
            .await
            .unwrap();
        assert_eq!((counts.get::<_, i64>(0), counts.get::<_, i64>(1)), (0, 0));
    }
    tx.rollback().await.unwrap();
}
