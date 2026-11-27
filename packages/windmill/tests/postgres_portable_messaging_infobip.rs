// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Infobip accounts (Viber, WhatsApp) delivered with a package, against
//! PostgreSQL, the vault and a loopback provider: their API keys arrive in
//! the encrypted package, are installed with the accounts, and Apply checks
//! the accounts it wrote before the event import reads them. An account
//! delivered without its key is still created, and waits for it. Synthetic
//! values only. Its own binary: the registry takes a NOWAIT table lock that
//! concurrent registry tests would contend for.

#[path = "support/schema.rs"]
mod schema;

use messaging::providers::Endpoints;
use messaging::test_server::{Reply, ServerOptions, TestServer};
use sequent_core::election_config::encrypted_delivery::encrypt_signed_delivery_with_credentials;
use sequent_core::election_config::messaging_credentials::{
    commitment, CredentialSidecar, SidecarEntry, SidecarFormat,
};
use sequent_core::election_config::portable_messaging::{parse, PortableMessagingAccounts};
use sequent_core::types::messaging::{AccountSender, CredentialName, MessagePurpose};
use serde_json::json;
use uuid::Uuid;
use windmill::postgres::messaging::list_messaging_accounts;
use windmill::services::documents::publish_unwrapped_pointer;
use windmill::services::import::configuration_package_credentials::{
    discard, load, stage_unwrapped, unwrap_delivery,
};
use windmill::services::messaging::keys::credential_secret_key;
use windmill::services::messaging::portable_registry;
use windmill::services::vault::read_secret;

// Synthetic: a fixed test vault key and format-valid, non-Infobip keys.
const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";
const VIBER_KEY: &str = "synthetic-viber-api-key-0123456789abcdef";
const WHATSAPP_KEY: &str = "synthetic-whatsapp-api-key-0123456789abcdef";
const PASSWORD: &str = "synthetic transfer password";
const UPLOADER: &str = "synthetic-uploader";

fn key(reference: &str, api_key: &str) -> SidecarEntry {
    SidecarEntry {
        reference: reference.into(),
        nonce: "07".repeat(32),
        aws_access_key_id: String::new(),
        aws_secret_access_key: String::new(),
        infobip_api_key: Some(api_key.into()),
    }
}

fn definitions(tenant: &Uuid, base_url: &str) -> PortableMessagingAccounts {
    let doc = json!({"format":"sequent.tenant-messaging-accounts/1","accounts":[
        {"reference":"viber","name":"Synthetic Viber",
         "sender":{"provider":"VIBER_INFOBIP","base_url":base_url,"sender":"Synthetic",
                   "content_mode":"FREE_TEXT"},
         "credential_commitment": commitment(tenant, &key("viber", VIBER_KEY)).unwrap()},
        {"reference":"whatsapp","name":"Synthetic WhatsApp",
         "sender":{"provider":"WHATSAPP_INFOBIP","base_url":base_url,"sender_number":"447700900123"},
         "credential_commitment": commitment(tenant, &key("whatsapp", WHATSAPP_KEY)).unwrap()},
        // Delivered without its key: created, and left for the operator.
        {"reference":"whatsapp-later","name":"Synthetic WhatsApp later",
         "sender":{"provider":"WHATSAPP_INFOBIP","base_url":base_url,"sender_number":"447700900124"}}
    ]});
    parse(&serde_json::to_vec(&doc).unwrap()).unwrap()
}

/// The number's templates, as Infobip lists them: one approved per purpose.
fn templates() -> Reply {
    Reply::Json(
        200,
        json!({"templates":[
            {"name":"otp_code","language":"en","status":"APPROVED","category":"AUTHENTICATION"},
            {"name":"election_notice","language":"en","status":"APPROVED","category":"UTILITY"}
        ]}),
    )
}

#[tokio::test]
async fn delivered_infobip_keys_are_installed_and_checked_at_apply() {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    // Deterministic RFC 4122 version-4 IDs: the vault accepts only v4 tenants.
    let tenant = Uuid::parse_str("00000000-0000-4000-8000-00000000022d").unwrap();
    let document = Uuid::parse_str("00000000-0000-4000-8000-00000000022e").unwrap();
    // The declared address must be Infobip's; the loopback answers in its
    // place through the deployment's Infobip endpoint override.
    let definitions = definitions(&tenant, "https://synthetic.api.infobip.com");

    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.document (id, tenant_id, name) VALUES ($1, $2, 'delivery.zip')",
        &[&document, &tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.configuration_package_staging (tenant_id, document_id, uploaded_by)
         VALUES ($1, $2, $3)",
        &[&tenant, &document, &UPLOADER],
    )
    .await
    .unwrap();
    let sidecar = CredentialSidecar {
        format: SidecarFormat::V1,
        tenant_id: tenant.to_string(),
        accounts: vec![key("viber", VIBER_KEY), key("whatsapp", WHATSAPP_KEY)],
    };
    let wire = encrypt_signed_delivery_with_credentials(
        b"PK\x03\x04synthetic-signed-zip",
        &sidecar.to_bytes().unwrap(),
        PASSWORD,
    )
    .unwrap();
    let unwrapped = unwrap_delivery(&tenant, &wire, PASSWORD).unwrap();
    let sha = unwrapped.package_sha256.clone();
    let expires_at = stage_unwrapped(&tx, &tenant, &document, &unwrapped, chrono::Utc::now())
        .await
        .unwrap();
    publish_unwrapped_pointer(&tx, tenant, document, Uuid::new_v4(), expires_at)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let server = TestServer::start_with(
        vec![],
        ServerOptions {
            then: Some(templates()),
            ..Default::default()
        },
    )
    .await;
    let endpoints = Endpoints {
        meta_graph: None,
        infobip: Some(server.base_url.clone()),
        aws: None,
    };

    // Apply, as the accounts section does it: install, discard, check.
    let tx = client.transaction().await.unwrap();
    let plan = portable_registry::plan(&tx, tenant, &sha, &definitions)
        .await
        .unwrap();
    let staged = load(
        &tx,
        &tenant,
        &sha,
        Some(&document),
        &definitions,
        true,
        Some(UPLOADER),
    )
    .await
    .unwrap()
    .expect("the uploader's staged keys");
    portable_registry::apply(&tx, tenant, &sha, &definitions, &plan, Some(&staged))
        .await
        .unwrap();
    discard(&tx, &tenant, &document).await.unwrap();
    let installed =
        portable_registry::check_written(&tx, tenant, &sha, &definitions, &plan, &endpoints)
            .await
            .unwrap();
    tx.commit().await.unwrap();

    // Each delivered key went to its own account, with its own header.
    let requests = server.requests();
    for (sender, key) in [
        ("/viber/1/senders/Synthetic/templates", VIBER_KEY),
        ("/whatsapp/2/senders/447700900123/templates", WHATSAPP_KEY),
    ] {
        let request = requests
            .iter()
            .find(|request| request.path == sender)
            .unwrap_or_else(|| panic!("{sender} was checked: {requests:?}"));
        assert_eq!(
            request.header("authorization"),
            Some(format!("App {key}").as_str())
        );
    }
    // The account without its key was never sent to the provider.
    assert!(!requests
        .iter()
        .any(|request| request.path.contains("447700900124")));

    let tx = client.transaction().await.unwrap();
    let accounts = list_messaging_accounts(&tx, &tenant).await.unwrap();
    assert_eq!(accounts.len(), 3);
    for account in &accounts {
        let (expected_key, connected) = match &account.sender {
            AccountSender::VIBER_INFOBIP { .. } => (Some(VIBER_KEY), true),
            AccountSender::WHATSAPP_INFOBIP { sender_number, .. }
                if sender_number == "447700900123" =>
            {
                (Some(WHATSAPP_KEY), true)
            }
            AccountSender::WHATSAPP_INFOBIP { .. } => (None, false),
            other => panic!("unexpected account {other:?}"),
        };
        let stored = read_secret(
            &tx,
            &tenant.to_string(),
            None,
            &credential_secret_key(&account.id, CredentialName::API_KEY),
        )
        .await
        .unwrap();
        assert_eq!(stored.as_deref(), expected_key, "{}", account.name);
        assert_eq!(
            account.credentials.contains_key(&CredentialName::API_KEY),
            expected_key.is_some()
        );
        assert_eq!(account.status.connected, connected, "{:?}", account.status);
        // Checked, so the daily pass leaves it until tomorrow.
        assert!(account.status.checked_at.is_some(), "{}", account.name);
        assert_eq!(
            account.status.details.identity,
            Some(account.sender.identity())
        );
    }
    let whatsapp = accounts
        .iter()
        .find(|account| {
            matches!(&account.sender, AccountSender::WHATSAPP_INFOBIP { sender_number, .. }
                if sender_number == "447700900123")
        })
        .unwrap();
    assert_eq!(
        whatsapp.status.approved_templates[&MessagePurpose::OTP],
        ["en"]
    );
    assert_eq!(
        whatsapp.status.approved_templates[&MessagePurpose::NOTICE],
        ["en"]
    );
    // The event import's later check accepts exactly the checked state.
    let observed =
        portable_registry::require_installed(&tx, tenant, &sha, &definitions, &installed)
            .await
            .unwrap();
    assert_eq!(observed, installed);
    tx.rollback().await.unwrap();
}
