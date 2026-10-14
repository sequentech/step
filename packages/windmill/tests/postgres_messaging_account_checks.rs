// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The daily account check's pass against the real schema and a loopback
//! provider (plan §2.7, R5-14). The pass reads every tenant's accounts, so
//! this binary holds this one test, its data is committed under its own
//! tenant, and every provider address answers the loopback; the assertions
//! concern only this tenant's accounts.

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::Pool;
use messaging::providers::Endpoints;
use messaging::test_server::{Reply, ServerOptions, TestServer};
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, MessageChannel, ProviderApproval, ReadinessPolicy,
};
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::messaging::{
    get_messaging_account, insert_messaging_account, update_messaging_account_status,
    AccountSettings, MessagingAccount,
};
use windmill::services::messaging::accounts::replace_credentials;
use windmill::services::messaging::checks::{check_due_accounts, due_accounts};
use windmill::services::messaging::dispatch::Dispatcher;

const MASTER_SECRET: &str = "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";
const SEED: u32 = 0xc0d0;

fn id(n: u32) -> Uuid {
    Uuid::parse_str(&format!("{SEED:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

/// SNS's answer to the account check: out of the SMS sandbox.
fn in_production() -> Reply {
    Reply::Raw(
        200,
        "text/xml",
        r#"<GetSMSSandboxAccountStatusResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/"><GetSMSSandboxAccountStatusResult><IsInSandbox>false</IsInSandbox></GetSMSSandboxAccountStatusResult><ResponseMetadata><RequestId>r</RequestId></ResponseMetadata></GetSMSSandboxAccountStatusResponse>"#.to_string(),
    )
}

fn sns(sender_id: &str) -> AccountSettings {
    AccountSettings {
        channel: MessageChannel::SMS,
        name: sender_id.to_string(),
        sender: AccountSender::AWS_SNS {
            sender_id: Some(sender_id.to_string()),
            origination_number: None,
            region: Some("us-east-1".to_string()),
        },
        limits: Default::default(),
        provider_approval: ProviderApproval::CONFIRMED,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    }
}

/// Removes what an earlier run committed.
async fn clean(pool: &Pool, tenant: &Uuid) {
    let client = pool.get().await.unwrap();
    for statement in [
        "DELETE FROM sequent_backend.secret WHERE tenant_id = $1",
        "DELETE FROM sequent_backend.messaging_account WHERE tenant_id = $1",
        "DELETE FROM sequent_backend.tenant WHERE id = $1",
    ] {
        client.execute(statement, &[tenant]).await.unwrap();
    }
}

async fn account(pool: &Pool, tenant: &Uuid, id: &Uuid) -> MessagingAccount {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let account = get_messaging_account(&tx, tenant, id)
        .await
        .unwrap()
        .unwrap();
    tx.rollback().await.unwrap();
    account
}

/// R5-14 and plan §2.7: one pass checks the accounts that are due and
/// stores each result with the identity it checked; an account checked
/// recently isn't called; one whose check can't run (its credential
/// doesn't decrypt) records the attempt and backs off instead of coming
/// first in every pass.
#[tokio::test]
async fn the_daily_pass_checks_due_accounts_and_backs_off_from_failing_ones() {
    std::env::set_var("MASTER_SECRET", MASTER_SECRET);
    let pool = schema::pool().await;
    let tenant = id(1);
    clean(&pool, &tenant).await;

    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    let never_checked = insert_messaging_account(&tx, &tenant, &sns("Due"), "c0d0-due")
        .await
        .unwrap();
    let recent = insert_messaging_account(&tx, &tenant, &sns("Recent"), "c0d0-recent")
        .await
        .unwrap();
    let mut checked_now = AccountCheck {
        connected: true,
        production_access: true,
        checked_at: Some(Utc::now().to_rfc3339()),
        ..Default::default()
    };
    checked_now.details.identity = Some(recent.sender.identity());
    update_messaging_account_status(&tx, &tenant, &recent.id, &checked_now)
        .await
        .unwrap();
    let failing = insert_messaging_account(&tx, &tenant, &sns("Failing"), "c0d0-failing")
        .await
        .unwrap();
    // SNS signs its requests: each account has its keys.
    for account in [&never_checked, &failing] {
        replace_credentials(
            &tx,
            &tenant,
            &account.id,
            &BTreeMap::from([
                (CredentialName::AWS_ACCESS_KEY_ID, "AKIDEXAMPLE".to_string()),
                (CredentialName::AWS_SECRET_ACCESS_KEY, "secret".to_string()),
            ]),
            false,
        )
        .await
        .unwrap();
    }
    // A secret that no longer decrypts: the check can't run.
    tx.execute(
        "UPDATE sequent_backend.secret SET value = '\\x00'::bytea
         WHERE tenant_id = $1 AND key LIKE $2",
        &[&tenant, &format!("messaging-account-{}-%", failing.id)],
    )
    .await
    .unwrap();
    // A status that can't be read at all: logged and still due, so the
    // row is never skipped unseen.
    let unreadable = insert_messaging_account(&tx, &tenant, &sns("Unreadable"), "c0d0-bad")
        .await
        .unwrap();
    tx.execute(
        r#"UPDATE sequent_backend.messaging_account
           SET status = '{"connected": "not a flag"}'::jsonb WHERE id = $1"#,
        &[&unreadable.id],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Every provider address is the loopback, which answers every request.
    let server = TestServer::start_with(
        vec![],
        ServerOptions {
            then: Some(in_production()),
            ..Default::default()
        },
    )
    .await;
    let dispatcher = Dispatcher::new(Endpoints {
        meta_graph: Some(server.base_url.clone()),
        infobip: Some(server.base_url.clone()),
        aws: Some(server.base_url.clone()),
    })
    .unwrap();
    let started = Utc::now();
    check_due_accounts(&mut client, &dispatcher).await.unwrap();

    let due = account(&pool, &tenant, &never_checked.id).await;
    assert!(due.status.connected, "{:?}", due.status);
    assert!(due.status.production_access);
    assert_eq!(
        due.status.details.identity,
        Some(due.sender.identity()),
        "the check names what it checked"
    );
    assert!(due.status.details.started_at.is_some());
    assert_eq!(
        account(&pool, &tenant, &recent.id).await.status,
        checked_now
    );
    let failed = account(&pool, &tenant, &failing.id).await;
    assert_eq!(failed.status.checked_at, None, "nothing was stored");
    let attempted = failed
        .status
        .details
        .attempted_at
        .as_deref()
        .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
        .expect("the attempt is recorded");
    assert!(attempted >= started - Duration::seconds(1));

    // The next pass leaves the checked ones alone; the failing account comes
    // back once its back-off is over, the others only when a day has
    // passed. The unreadable one stays due on every pass.
    let mine = |now| {
        let pool = pool.clone();
        async move {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let mut due: Vec<Uuid> = due_accounts(&tx, now)
                .await
                .unwrap()
                .into_iter()
                .filter(|account| account.tenant_id == tenant)
                .map(|account| account.id)
                .collect();
            due.sort();
            tx.rollback().await.unwrap();
            due
        }
    };
    assert_eq!(mine(Utc::now()).await, vec![unreadable.id]);
    let mut back = vec![failing.id, unreadable.id];
    back.sort();
    assert_eq!(mine(Utc::now() + Duration::hours(4)).await, back);

    clean(&pool, &tenant).await;
}
