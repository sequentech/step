// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A tenant reset returns the tenant's Postgres rows to the baseline taken
//! before its first configuration package, and touches no other tenant.
//!
//! Needs the PostgreSQL server `HASURA_DB__*` names; every test rolls back.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::tenant::allow_configuration_signing_change;
use windmill::postgres::tenant_reset::{
    allow_tenant_reset, capture_rows, capture_tenant_row, claim_reset, get_baseline,
    purge_tenant_rows, restore_rows, restore_tenant_row, save_baseline, set_lifecycle_state,
    tenant_tables, Baseline, LifecycleState, TableRole, NOT_RESTORED_TABLES, PRESERVED_TABLES,
    TENANT_LOG_TABLES,
};

const TRUST: &str =
    r#"{"policy": "required", "package_roots": "", "staff_roots": "", "required_approvals": 2}"#;

async fn tenant(tx: &Transaction<'_>, slug: &str) -> Uuid {
    let id = Uuid::new_v4();
    allow_configuration_signing_change(tx, &id.to_string())
        .await
        .unwrap();
    let settings: Value = json!({
        "theme": "dark",
        "configuration_signing": serde_json::from_str::<Value>(TRUST).unwrap(),
    });
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug, annotations, settings) \
         VALUES ($1, $2, $3, $4)",
        &[
            &id,
            &format!("{slug}-{id}"),
            &json!({"css": "before"}),
            &settings,
        ],
    )
    .await
    .unwrap();
    tx.execute(
        "SELECT set_config('sequent.configuration_signing_tenant', '', true)",
        &[],
    )
    .await
    .unwrap();
    id
}

/// What an installer seeds before any package: a trustee, a secret and a
/// tenant-level document.
async fn seed(tx: &Transaction<'_>, tenant: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.trustee (tenant_id, name, public_key) VALUES ($1, 'seeded', 'pk-seeded')",
        &[&tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.secret (tenant_id, key, value) VALUES ($1, $2, '\\x0102')",
        &[&tenant, &format!("seeded-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.document (tenant_id, name) VALUES ($1, 'logo.png')",
        &[&tenant],
    )
    .await
    .unwrap();
}

/// A staged upload: its document, staging row and messaging credentials.
async fn stage(tx: &Transaction<'_>, tenant: Uuid) {
    let document: Uuid = tx
        .query_one(
            "INSERT INTO sequent_backend.document (tenant_id, name) VALUES ($1, 'delivery.zip') RETURNING id",
            &[&tenant],
        )
        .await
        .unwrap()
        .get(0);
    tx.execute(
        "INSERT INTO sequent_backend.configuration_package_staging (tenant_id, document_id) VALUES ($1, $2)",
        &[&tenant, &document],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.secret (tenant_id, key, value) VALUES ($1, $2, '\\x03')",
        &[
            &tenant,
            &format!("configuration-package-staging-{document}-messaging-credentials"),
        ],
    )
    .await
    .unwrap();
}

/// What an Apply and later use leave: an event, package history, a roster,
/// tasks, more trustees, entries for the tenant and the event log.
async fn import(tx: &Transaction<'_>, tenant: Uuid) -> Uuid {
    let event = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol) VALUES ($1, $2, 'RSA')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    // Its guard lets a sealing row go only with its event.
    tx.execute(
        "INSERT INTO sequent_backend.cast_vote_sealing_required (tenant_id, election_event_id) VALUES ($1, $2)",
        &[&tenant, &event],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.configuration_package (tenant_id, election_event_id, external_id, \
         revision, manifest_sha256, manifest, signer_subject, signer_serial, signer_fingerprint, approvers) \
         VALUES ($1, $2, 'pkg', 1, repeat('a', 64), '{}', 's', '1', 'f', '[]')",
        &[&tenant, &event],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.managed_webauthn_roster (tenant_id, roster_json, sha256, generation, activated_by) \
         VALUES ($1, '{}', repeat('b', 64), 1, 'admin')",
        &[&tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tasks_execution (tenant_id, name, type, execution_status, executed_by_user) \
         VALUES ($1, 'apply', 'IMPORT_CONFIGURATION_PACKAGE', 'SUCCESS', 'admin')",
        &[&tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.trustee (tenant_id, name, public_key) VALUES ($1, 'imported', 'pk-imported')",
        &[&tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "UPDATE sequent_backend.trustee SET public_key = 'pk-changed' WHERE tenant_id = $1 AND name = 'seeded'",
        &[&tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.document (tenant_id, election_event_id, name) VALUES ($1, $2, 'ballot.pdf')",
        &[&tenant, &event],
    )
    .await
    .unwrap();
    for event_id in [None, Some(event)] {
        tx.execute(
            "INSERT INTO sequent_backend.audit_log_outbox (tenant_id, election_event_id, statement_kind, \
             event_type, log_type, description, details) VALUES ($1, $2, 'AdminChange', 'USER', 'INFO', 'd', '{}')",
            &[&tenant, &event_id],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "UPDATE sequent_backend.tenant SET annotations = '{\"css\": \"after\"}', \
         settings = settings || '{\"theme\": \"light\", \"imported\": true}' WHERE id = $1",
        &[&tenant],
    )
    .await
    .unwrap();
    event
}

/// Every row the tenant has, per table, as sorted JSON (for comparisons).
async fn snapshot(tx: &Transaction<'_>, tenant: Uuid) -> BTreeMap<String, Vec<String>> {
    let mut all = BTreeMap::new();
    for table in tenant_tables(tx).await.unwrap() {
        let sql = format!(
            "SELECT to_jsonb(r)::text FROM sequent_backend.\"{}\" r WHERE tenant_id = $1",
            table.name
        );
        let mut rows: Vec<String> = tx
            .query(sql.as_str(), &[&tenant])
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect();
        rows.sort();
        if !rows.is_empty() {
            all.insert(table.name, rows);
        }
    }
    all
}

async fn take_baseline(tx: &Transaction<'_>, tenant: Uuid) {
    let id = tenant.to_string();
    let baseline = Baseline {
        tenant_row: capture_tenant_row(tx, &id).await.unwrap(),
        rows: capture_rows(tx, &id).await.unwrap(),
        realm: json!({}),
    };
    assert!(save_baseline(tx, &id, &baseline, None).await.unwrap());
    assert!(
        !save_baseline(tx, &id, &Baseline::default(), None)
            .await
            .unwrap(),
        "a second baseline replaced the first"
    );
}

async fn reset(tx: &Transaction<'_>, tenant: Uuid, keep_task: Option<Uuid>) {
    let id = tenant.to_string();
    claim_reset(tx, &id).await.unwrap();
    allow_tenant_reset(tx, &id).await.unwrap();
    let baseline = get_baseline(tx, &id).await.unwrap().unwrap();
    purge_tenant_rows(tx, &id, keep_task).await.unwrap();
    restore_rows(tx, &id, &baseline.rows).await.unwrap();
    restore_tenant_row(tx, &id, &baseline.tenant_row)
        .await
        .unwrap();
    set_lifecycle_state(tx, &id, LifecycleState::Active)
        .await
        .unwrap();
    tx.execute("SELECT set_config('sequent.tenant_reset', '', true)", &[])
        .await
        .unwrap();
}

async fn count(tx: &Transaction<'_>, sql: &str, tenant: Uuid) -> i64 {
    tx.query_one(sql, &[&tenant]).await.unwrap().get(0)
}

#[tokio::test]
async fn the_lists_name_tenant_tables() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tables = tenant_tables(&tx).await.unwrap();
    for (name, _) in PRESERVED_TABLES
        .iter()
        .chain(TENANT_LOG_TABLES)
        .chain(NOT_RESTORED_TABLES)
    {
        assert!(
            tables.iter().any(|t| t.name == *name),
            "{name} is listed but has no tenant_id column"
        );
    }
    // Parents come before the tables that reference them.
    let position = |name: &str| tables.iter().position(|t| t.name == name).unwrap();
    assert!(position("election_event") < position("election"));
    assert!(position("document") < position("configuration_package_staging"));
    assert_eq!(
        tables
            .iter()
            .find(|t| t.name == "managed_webauthn_roster")
            .unwrap()
            .role,
        TableRole::NotRestored
    );
}

#[tokio::test]
async fn a_reset_returns_to_the_baseline() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    let target = tenant(&tx, "target").await;
    let other = tenant(&tx, "other").await;
    seed(&tx, target).await;
    seed(&tx, other).await;
    // The baseline is taken at the first upload, after its staging rows
    // exist in the retry case; they must not become part of it.
    stage(&tx, target).await;
    let before_stage_rows = {
        let mut rows = snapshot(&tx, target).await;
        rows.remove("configuration_package_staging");
        rows
    };
    take_baseline(&tx, target).await;

    import(&tx, target).await;
    import(&tx, other).await;
    let other_before = snapshot(&tx, other).await;

    // Package trust changes after the baseline: the reset keeps the new one.
    allow_configuration_signing_change(&tx, &target.to_string())
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.tenant SET settings = jsonb_set(settings, \
         '{configuration_signing,required_approvals}', '3') WHERE id = $1",
        &[&target],
    )
    .await
    .unwrap();
    tx.execute(
        "SELECT set_config('sequent.configuration_signing_tenant', '', true)",
        &[],
    )
    .await
    .unwrap();

    // The reset's own task row survives.
    let task: Uuid = tx
        .query_one(
            "INSERT INTO sequent_backend.tasks_execution (tenant_id, name, type, execution_status, executed_by_user) \
             VALUES ($1, 'reset', 'RESET_TENANT', 'IN_PROGRESS', 'admin') RETURNING id",
            &[&target],
        )
        .await
        .unwrap()
        .get(0);

    reset(&tx, target, Some(task)).await;

    let after = snapshot(&tx, target).await;
    let restored: BTreeMap<_, _> = after
        .iter()
        .filter(|(table, _)| {
            !matches!(
                table.as_str(),
                "tasks_execution" | "audit_log_outbox" | "tenant_reset_baseline"
            )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let expected: BTreeMap<_, _> = before_stage_rows
        .into_iter()
        .map(|(table, rows)| {
            let rows = rows
                .into_iter()
                .filter(|row| {
                    !row.contains("delivery.zip") && !row.contains("configuration-package-staging-")
                })
                .collect::<Vec<_>>();
            (table, rows)
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect();
    assert_eq!(restored, expected, "rows differ from the baseline");

    assert_eq!(
        count(
            &tx,
            "SELECT count(*) FROM sequent_backend.tasks_execution WHERE tenant_id = $1",
            target
        )
        .await,
        1
    );
    // The tenant log keeps its tenant-level entry and loses the event's.
    assert_eq!(
        count(
            &tx,
            "SELECT count(*) FROM sequent_backend.audit_log_outbox WHERE tenant_id = $1 AND election_event_id IS NULL",
            target
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &tx,
            "SELECT count(*) FROM sequent_backend.audit_log_outbox WHERE tenant_id = $1 AND election_event_id IS NOT NULL",
            target
        )
        .await,
        0
    );

    let row = tx
        .query_one(
            "SELECT annotations, settings, EXISTS (SELECT 1 FROM sequent_backend.tenant_reset_state \
             WHERE tenant_id = $1) FROM sequent_backend.tenant WHERE id = $1",
            &[&target],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, Value>(0), json!({"css": "before"}));
    let settings: Value = row.get(1);
    assert_eq!(settings["theme"], "dark");
    assert!(settings.get("imported").is_none());
    assert_eq!(settings["configuration_signing"]["required_approvals"], 3);
    assert!(!row.get::<_, bool>(2), "the tenant is still resetting");
    assert_eq!(
        count(
            &tx,
            "SELECT count(*) FROM sequent_backend.cast_vote_sealing_required WHERE tenant_id = $1",
            target
        )
        .await,
        0
    );

    assert_eq!(
        snapshot(&tx, other).await,
        other_before,
        "another tenant changed"
    );

    // A second reset from the same baseline gives the same rows.
    reset(&tx, target, Some(task)).await;
    assert_eq!(snapshot(&tx, target).await, after);
}

#[tokio::test]
async fn a_resetting_tenant_takes_no_new_rows_but_its_reset() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let target = tenant(&tx, "target").await;
    let other = tenant(&tx, "other").await;
    set_lifecycle_state(&tx, &target.to_string(), LifecycleState::Resetting)
        .await
        .unwrap();

    let insert = "INSERT INTO sequent_backend.trustee (tenant_id, name) VALUES ($1, 'late')";
    tx.execute("SAVEPOINT s", &[]).await.unwrap();
    let refused = tx.execute(insert, &[&target]).await;
    tx.execute("ROLLBACK TO SAVEPOINT s", &[]).await.unwrap();
    let message = refused
        .expect_err("an insert into a resetting tenant was allowed")
        .as_db_error()
        .map(|e| e.message().to_string())
        .unwrap_or_default();
    assert!(message.contains("is being reset"), "{message}");

    // A new reset task is let in: running the reset again finishes it.
    tx.execute(
        "INSERT INTO sequent_backend.tasks_execution (tenant_id, name, type, execution_status, executed_by_user) \
         VALUES ($1, 'reset', 'RESET_TENANT', 'IN_PROGRESS', 'admin')",
        &[&target],
    )
    .await
    .unwrap();
    tx.execute("SAVEPOINT s", &[]).await.unwrap();
    assert!(tx
        .execute(
            "INSERT INTO sequent_backend.tasks_execution (tenant_id, name, type, execution_status, executed_by_user) \
             VALUES ($1, 'export', 'EXPORT_TENANT_CONFIG', 'IN_PROGRESS', 'admin')",
            &[&target],
        )
        .await
        .is_err());
    tx.execute("ROLLBACK TO SAVEPOINT s", &[]).await.unwrap();

    // Allowing another tenant's reset doesn't open this one.
    allow_tenant_reset(&tx, &other.to_string()).await.unwrap();
    tx.execute("SAVEPOINT s", &[]).await.unwrap();
    assert!(tx.execute(insert, &[&target]).await.is_err());
    tx.execute("ROLLBACK TO SAVEPOINT s", &[]).await.unwrap();
    tx.execute(insert, &[&other]).await.unwrap();

    allow_tenant_reset(&tx, &target.to_string()).await.unwrap();
    tx.execute(insert, &[&target]).await.unwrap();
}

#[tokio::test]
async fn kept_rows_go_only_in_their_tenants_reset() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let target = tenant(&tx, "target").await;
    let other = tenant(&tx, "other").await;
    let event = import(&tx, target).await;
    // monitoring_event refuses deletes by design.
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id) VALUES ($1, $2)",
        &[&target, &event],
    )
    .await
    .unwrap();

    for allowed in [None, Some(other)] {
        if let Some(tenant) = allowed {
            allow_tenant_reset(&tx, &tenant.to_string()).await.unwrap();
        }
        tx.execute("SAVEPOINT s", &[]).await.unwrap();
        let purged = purge_tenant_rows(&tx, &target.to_string(), None).await;
        tx.execute("ROLLBACK TO SAVEPOINT s", &[]).await.unwrap();
        assert!(
            purged.is_err(),
            "kept rows deleted with permission {allowed:?}"
        );
    }

    allow_tenant_reset(&tx, &target.to_string()).await.unwrap();
    let counts = purge_tenant_rows(&tx, &target.to_string(), None)
        .await
        .unwrap();
    assert_eq!(counts.get("monitoring_event"), Some(&1));
}
