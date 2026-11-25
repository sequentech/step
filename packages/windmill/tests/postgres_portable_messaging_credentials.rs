// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Delivered AWS key pairs against PostgreSQL and the vault: server unwrap,
//! staging for the uploader only, install in the account's own transaction,
//! the installed-account checks, and discard. Synthetic values only; no
//! provider is ever contacted. Its own binary: the registry takes a NOWAIT
//! table lock that concurrent registry tests would contend for.

#[path = "support/schema.rs"]
mod schema;

use sequent_core::election_config::encrypted_delivery::encrypt_signed_delivery_with_credentials;
use sequent_core::election_config::messaging_credentials::{
    commitment, CredentialSidecar, SidecarEntry, SidecarFormat,
};
use sequent_core::election_config::portable_messaging::{parse, PortableMessagingAccounts};
use sequent_core::types::messaging::CredentialName;
use uuid::Uuid;
use windmill::postgres::messaging::list_messaging_accounts;
use windmill::services::documents::publish_unwrapped_pointer;
use windmill::services::import::configuration_package_credentials::{
    discard, load, stage_unwrapped, staged_secret_key, unwrap_delivery, STAGED_BY_OTHER,
};
use windmill::services::messaging::portable_registry;
use windmill::services::vault::read_secret;

// Synthetic: a fixed test vault key and format-valid, non-AWS values.
const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";
const KEY_ID: &str = "AKIASYNTHETICEXAMPLE";
const SECRET: &str = "SyntheticSecretValueForTestsOnly00000000";
const PASSWORD: &str = "synthetic transfer password";
const UPLOADER: &str = "synthetic-uploader";

fn entry() -> SidecarEntry {
    SidecarEntry {
        reference: "email".into(),
        nonce: "07".repeat(32),
        aws_access_key_id: KEY_ID.into(),
        aws_secret_access_key: SECRET.into(),
    }
}

fn definitions(tenant: &Uuid) -> PortableMessagingAccounts {
    let doc = serde_json::json!({"format":"sequent.tenant-messaging-accounts/1","accounts":[{
        "reference":"email","name":"Synthetic Email",
        "sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"},
        "credential_commitment": commitment(tenant, &entry()).unwrap(),
    }]});
    parse(&serde_json::to_vec(&doc).unwrap()).unwrap()
}

#[tokio::test]
async fn delivered_keys_install_once_for_the_uploader_and_keep_the_account_pristine() {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tenant = Uuid::from_u128(301);
    let document = Uuid::from_u128(302);
    let definitions = definitions(&tenant);

    // A staging row recorded for its uploader, as get_upload_url makes it.
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

    // Server unwrap of a V2 envelope, staged in the vault with its pointer.
    let sidecar = CredentialSidecar {
        format: SidecarFormat::V1,
        tenant_id: tenant.to_string(),
        accounts: vec![entry()],
    };
    let wire = encrypt_signed_delivery_with_credentials(
        b"PK\x03\x04synthetic-signed-zip",
        &sidecar.to_bytes().unwrap(),
        PASSWORD,
    )
    .unwrap();
    let unwrapped = unwrap_delivery(&tenant, &wire, PASSWORD).unwrap();
    let sha = unwrapped.package_sha256.clone();
    let now = chrono::Utc::now();
    let expires_at = stage_unwrapped(&tx, &tenant, &document, &unwrapped, now)
        .await
        .unwrap();
    publish_unwrapped_pointer(&tx, tenant, document, Uuid::new_v4(), expires_at)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Preview: only the uploader's task may use the staged pair.
    let tx = client.transaction().await.unwrap();
    let plan = portable_registry::plan(&tx, tenant, &sha, &definitions)
        .await
        .unwrap();
    for actor in [Some("another-admin"), None] {
        let refused = load(
            &tx,
            &tenant,
            &sha,
            Some(&document),
            &definitions,
            true,
            actor,
        )
        .await
        .err()
        .expect("another actor must be refused")
        .to_string();
        assert_eq!(refused, STAGED_BY_OTHER);
    }
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
    .expect("the uploader's staged pair");
    assert!(!format!("{staged:?}").contains(SECRET));

    // Apply: the account and its two vault credentials in one transaction.
    let installed = portable_registry::apply(&tx, tenant, &sha, &definitions, &plan, Some(&staged))
        .await
        .unwrap();
    discard(&tx, &tenant, &document).await.unwrap();
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let accounts = list_messaging_accounts(&tx, &tenant).await.unwrap();
    let [account] = accounts.as_slice() else {
        panic!("exactly one account")
    };
    assert_eq!(
        account.credentials.keys().copied().collect::<Vec<_>>(),
        [
            CredentialName::AWS_ACCESS_KEY_ID,
            CredentialName::AWS_SECRET_ACCESS_KEY
        ]
    );
    // record_credentials_replaced ran in the insert's transaction: now() is
    // that transaction's start time, so the account still reads as untouched.
    assert_eq!(account.created_at, account.updated_at);
    // The event import's later check accepts exactly this installed state.
    let observed = portable_registry::require_installed(&tx, tenant, &sha, &definitions, &plan)
        .await
        .unwrap();
    assert_eq!(observed, installed);
    // The staged copy is gone, the expiry cleared; nothing to consume again.
    assert!(read_secret(
        &tx,
        &tenant.to_string(),
        None,
        &staged_secret_key(&document)
    )
    .await
    .unwrap()
    .is_none());
    let expiry: Option<chrono::DateTime<chrono::Utc>> = tx
        .query_one(
            "SELECT credentials_expires_at FROM sequent_backend.configuration_package_staging
             WHERE document_id = $1",
            &[&document],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(expiry, None);
    // A resumed Apply (account exists, nothing needed) finds nothing to use.
    assert!(load(
        &tx,
        &tenant,
        &sha,
        Some(&document),
        &definitions,
        false,
        Some(UPLOADER)
    )
    .await
    .unwrap()
    .is_none());
}
