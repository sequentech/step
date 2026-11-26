// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The deployment's operational signing policy as Windmill reads it: the
//! default is legacy P12; a promoted tenant, or a promoted deployment,
//! requires hardware; an unreadable policy is unavailable, never legacy; and
//! the start-up assertion refuses the superseded environment settings, a
//! profile that disagrees with the database, and an HSM-required deployment.
//!
//! Promotion is one-way, so every promotion here happens in a transaction
//! that is rolled back. The start-up assertion reads process-global
//! environment variables: each case runs in a child process of this test
//! binary, which connects to the parent's database.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Runtime;
use std::ffi::OsString;
use tokio_postgres::NoTls;
use uuid::Uuid;
use windmill::services::database::PgConfig;
use windmill::services::signing::operational_policy::{
    approval_signing_mode, assert_startup_policy, direct_hsm_approval_build, require_legacy_event,
    require_legacy_signing, signing_mode, signing_mode_for_event, LegacySigningBlocked,
    OperationalSigningMode,
};

const PROFILE: &str = "OPERATIONAL_SIGNING_INSTALLATION_PROFILE";
const SUPERSEDED_MODE: &str = "OPERATIONAL_SIGNING_MODE";
const SUPERSEDED_TENANTS: &str = "OPERATIONAL_SIGNING_HSM_REQUIRED_TENANTS";

/// Set only in a child: the connection settings of the parent's database.
const CHILD_DATABASE: &str = "WINDMILL_TEST_OPERATIONAL_POLICY_DATABASE";
/// Set only in a child: `ok` or `unavailable`.
const CHILD_EXPECTS: &str = "WINDMILL_TEST_OPERATIONAL_POLICY_EXPECTS";
/// What a child prints once it ran the assertion.
const CHILD_RAN: &str = "operational-policy-child-ran";

#[tokio::test]
async fn the_default_policy_is_legacy_p12() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = Uuid::new_v4();
    assert_eq!(
        signing_mode(&tx, tenant).await,
        Ok(OperationalSigningMode::LegacyP12)
    );
    assert_eq!(
        require_legacy_signing(&tx, tenant).await,
        if direct_hsm_approval_build() {
            Err(LegacySigningBlocked::HsmRequired)
        } else {
            Ok(())
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn approval_does_not_need_a_tenant_or_event_switch() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = (Uuid::new_v4(), Uuid::new_v4());
    let expected = if direct_hsm_approval_build() {
        OperationalSigningMode::HsmRequired
    } else {
        OperationalSigningMode::LegacyP12
    };
    assert_eq!(approval_signing_mode(&tx, tenant).await, Ok(expected));
    assert_eq!(
        signing_mode_for_event(&tx, tenant, event).await,
        Ok(expected)
    );
    assert_eq!(
        require_legacy_event(&tx, tenant, event).await,
        if direct_hsm_approval_build() {
            Err(LegacySigningBlocked::HsmRequired)
        } else {
            Ok(())
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_promoted_tenant_requires_hardware_and_other_tenants_stay_legacy() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (promoted, other) = (Uuid::new_v4(), Uuid::new_v4());
    for tenant in [promoted, other] {
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
    }
    // The promotion takes the policy's exclusive lock before this
    // transaction reads the policy, so two promoting tests can't deadlock.
    tx.execute(
        "INSERT INTO sequent_backend.operational_signing_required_tenant (tenant_id) VALUES ($1)",
        &[&promoted],
    )
    .await
    .unwrap();
    assert_eq!(
        signing_mode(&tx, promoted).await,
        Ok(OperationalSigningMode::HsmRequired)
    );
    assert_eq!(
        require_legacy_signing(&tx, promoted).await,
        Err(LegacySigningBlocked::HsmRequired)
    );
    assert_eq!(
        signing_mode(&tx, other).await,
        Ok(OperationalSigningMode::LegacyP12)
    );
    assert_eq!(
        require_legacy_signing(&tx, other).await,
        if direct_hsm_approval_build() {
            Err(LegacySigningBlocked::HsmRequired)
        } else {
            Ok(())
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_promoted_deployment_requires_hardware_everywhere_and_does_not_start() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let updated = tx
        .execute(
            "UPDATE sequent_backend.operational_signing_policy SET mode = 'hsm-required'
             WHERE singleton",
            &[],
        )
        .await
        .unwrap();
    assert_eq!(updated, 1);
    let tenant = Uuid::new_v4();
    assert_eq!(
        signing_mode(&tx, tenant).await,
        Ok(OperationalSigningMode::HsmRequired)
    );
    assert_eq!(
        require_legacy_signing(&tx, tenant).await,
        Err(LegacySigningBlocked::HsmRequired)
    );
    // Whatever this process's environment asserts, a deployment whose
    // database requires hardware doesn't report a healthy start.
    assert_eq!(
        assert_startup_policy(&tx).await,
        Err(LegacySigningBlocked::Unavailable)
    );
    tx.rollback().await.unwrap();

    // Nothing of it was kept.
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        signing_mode(&tx, tenant).await,
        Ok(OperationalSigningMode::LegacyP12)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_policy_that_cant_be_read_is_unavailable() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // A failed statement aborts the transaction: every later read fails.
    assert!(tx.batch_execute("SELECT 1 / 0").await.is_err());
    let tenant = Uuid::new_v4();
    assert_eq!(
        signing_mode(&tx, tenant).await,
        Err(LegacySigningBlocked::Unavailable)
    );
    assert_eq!(
        require_legacy_signing(&tx, tenant).await,
        Err(LegacySigningBlocked::Unavailable)
    );
    assert_eq!(
        assert_startup_policy(&tx).await,
        Err(LegacySigningBlocked::Unavailable)
    );
    tx.rollback().await.unwrap();
}

#[test]
fn a_blocked_legacy_signature_says_why() {
    assert_eq!(
        LegacySigningBlocked::HsmRequired.to_string(),
        "This tenant requires a server-managed signing device."
    );
    assert_eq!(
        LegacySigningBlocked::Unavailable.to_string(),
        "The official signing mode is unavailable."
    );
    let error: Box<dyn std::error::Error> = Box::new(LegacySigningBlocked::Unavailable);
    assert!(error.source().is_none());
    assert_ne!(
        OperationalSigningMode::LegacyP12,
        OperationalSigningMode::HsmRequired
    );
}

/// The child of [`the_startup_assertion_follows_the_environment_and_the_database`]:
/// runs the assertion with the environment its parent gave it. Does nothing
/// in an ordinary run.
#[tokio::test]
async fn startup_policy_child() {
    let Ok(settings) = std::env::var(CHILD_DATABASE) else {
        return;
    };
    let expected = match std::env::var(CHILD_EXPECTS).as_deref() {
        Ok("ok") => Ok(()),
        Ok("unavailable") => Err(LegacySigningBlocked::Unavailable),
        other => panic!("unknown expectation {other:?}"),
    };
    let config: deadpool_postgres::Config =
        serde_json::from_str(&settings).expect("the parent's database settings");
    let pool = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("child database pool");
    let client = pool.get().await.expect("child database connection");
    assert_eq!(assert_startup_policy(&client).await, expected);
    println!("{CHILD_RAN}");
}

/// Runs [`startup_policy_child`] in a process with `environment` on top of
/// this process's own, without the three policy variables.
async fn child(settings: &str, environment: &[(&str, OsString)], expects: &str) {
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .kill_on_drop(true)
        .args(["--exact", "startup_policy_child", "--nocapture"])
        .env_remove(PROFILE)
        .env_remove(SUPERSEDED_MODE)
        .env_remove(SUPERSEDED_TENANTS)
        .env(CHILD_DATABASE, settings)
        .env(CHILD_EXPECTS, expects);
    for (key, value) in environment {
        command.env(key, value);
    }
    let output = command.output().await.expect("run the child test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains(CHILD_RAN),
        "{environment:?} expecting {expects}:\n{stdout}\n{stderr}"
    );
}

#[tokio::test]
async fn the_startup_assertion_follows_the_environment_and_the_database() {
    let pool = schema::pool().await;
    let database: String = pool
        .get()
        .await
        .unwrap()
        .query_one("SELECT current_database()", &[])
        .await
        .unwrap()
        .get(0);
    let mut config = PgConfig::from_env().unwrap().hasura_db;
    config.dbname = Some(database);
    let settings = serde_json::to_string(&config).unwrap();
    let text = |value: &str| OsString::from(value);

    // The committed deployment policy of the fixture is legacy-p12.
    // The dedicated build has no configurable signing profile. Without
    // one, the workers can start in setup-only mode.
    child(&settings, &[], "ok").await;
    child(
        &settings,
        &[(PROFILE, text("legacy-p12"))],
        if direct_hsm_approval_build() {
            "unavailable"
        } else {
            "ok"
        },
    )
    .await;
    // A profile the database doesn't have, or that is no profile.
    child(&settings, &[(PROFILE, text("hsm-required"))], "unavailable").await;
    child(&settings, &[(PROFILE, text("softhsm"))], "unavailable").await;
    child(&settings, &[(PROFILE, text(""))], "unavailable").await;
    // The superseded settings are refused, not ignored, whatever they say.
    child(
        &settings,
        &[(SUPERSEDED_MODE, text("legacy-p12"))],
        "unavailable",
    )
    .await;
    child(&settings, &[(SUPERSEDED_TENANTS, text(""))], "unavailable").await;
    child(
        &settings,
        &[
            (PROFILE, text("legacy-p12")),
            (SUPERSEDED_TENANTS, text(&Uuid::new_v4().to_string())),
        ],
        "unavailable",
    )
    .await;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        child(
            &settings,
            &[(PROFILE, OsString::from_vec(vec![0xff, 0xfe]))],
            "unavailable",
        )
        .await;
    }
}
