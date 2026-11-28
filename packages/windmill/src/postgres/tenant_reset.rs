// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Postgres side of a tenant reset: the baseline taken before the first
//! configuration package, and the purge that returns the tenant to it.
//!
//! Every `sequent_backend` table with a `tenant_id` column is found in the
//! catalog, so a table added later is purged and restored without a change
//! here. Only the tables listed below are treated differently, each with its
//! reason; the `postgres_tenant_reset` test checks the lists against the
//! schema.

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tracing::instrument;
use uuid::Uuid;

/// Tables a reset leaves untouched: deployment configuration written by the
/// installer or the operator, never by an import.
pub const PRESERVED_TABLES: &[(&str, &str)] = &[
    (
        "operational_signing_required_tenant",
        "deployment signing policy, set by the operator",
    ),
    (
        "operational_setup_installation",
        "operator-provisioned setup installation",
    ),
    (
        "operational_signing_official_enrollment",
        "operator-provisioned official enrollment",
    ),
    (
        "operational_initialize_local_profile",
        "operator-provisioned local initialize profile; a reset refuses while it names an event",
    ),
    (
        "operational_signing_demo_scope",
        "operator-provisioned demo installation; a reset refuses while it names an event",
    ),
    ("tenant_reset_baseline", "the state a reset returns to"),
];

/// The tenant's own log is kept: only its election-event rows go, with the
/// events. The reset adds its own entry to it.
pub const TENANT_LOG_TABLES: &[(&str, &str)] = &[
    ("audit_log_outbox", "pending entries of the kept tenant log"),
    (
        "electoral_log_checkpoint",
        "checkpoints of the kept tenant log",
    ),
    (
        "electoral_log_outbox",
        "pending electoral-log entries; event databases are dropped",
    ),
];

/// Tables emptied for the tenant and never restored, even when the baseline
/// has rows for them: they are what makes a package or a roster count as
/// already imported.
pub const NOT_RESTORED_TABLES: &[(&str, &str)] = &[
    (
        "managed_webauthn_roster",
        "the signer roster is activated again after a reset",
    ),
    (
        "configuration_package",
        "package history: the same package is imported again",
    ),
    (
        "configuration_package_staging",
        "staged uploads of the packages being reset",
    ),
    (
        "configuration_revocation_list",
        "revocation lists arrive with the packages",
    ),
    (
        "tasks_execution",
        "earlier Apply tasks are matched to a re-import; the reset keeps its own task",
    ),
];

/// Documents and secrets created by staging a package are not part of the
/// tenant before its first import, even when the baseline is taken after
/// the upload.
const STAGED_SECRET_PREFIX: &str = "configuration-package-staging-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableRole {
    Preserved,
    TenantLog,
    NotRestored,
    Restored,
}

pub fn table_role(name: &str) -> TableRole {
    let listed = |list: &[(&str, &str)]| list.iter().any(|(table, _)| *table == name);
    if listed(PRESERVED_TABLES) {
        TableRole::Preserved
    } else if listed(TENANT_LOG_TABLES) {
        TableRole::TenantLog
    } else if listed(NOT_RESTORED_TABLES) {
        TableRole::NotRestored
    } else {
        TableRole::Restored
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantTable {
    pub name: String,
    pub has_event: bool,
    pub role: TableRole,
}

/// Every `sequent_backend` table with a `tenant_id` column, except `tenant`,
/// parents before the tables that reference them.
#[instrument(skip(tx), err)]
pub async fn tenant_tables(tx: &Transaction<'_>) -> Result<Vec<TenantTable>> {
    let rows = tx
        .query(
            r#"
            SELECT c.relname::text AS name,
                   EXISTS (
                       SELECT 1 FROM pg_attribute e
                       WHERE e.attrelid = c.oid AND e.attname = 'election_event_id'
                           AND NOT e.attisdropped
                   ) AS has_event
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            JOIN pg_attribute a ON a.attrelid = c.oid AND a.attname = 'tenant_id'
                AND NOT a.attisdropped
            WHERE n.nspname = 'sequent_backend' AND c.relkind IN ('r', 'p')
                AND c.relname <> 'tenant'
            ORDER BY c.relname
            "#,
            &[],
        )
        .await
        .context("Error listing tenant tables")?;
    let tables: Vec<TenantTable> = rows
        .iter()
        .map(|row| {
            let name: String = row.get("name");
            TenantTable {
                role: table_role(&name),
                has_event: row.get("has_event"),
                name,
            }
        })
        .collect();

    let edges = tx
        .query(
            r#"
            SELECT child.relname::text AS child, parent.relname::text AS parent
            FROM pg_constraint k
            JOIN pg_class child ON child.oid = k.conrelid
            JOIN pg_class parent ON parent.oid = k.confrelid
            JOIN pg_namespace n ON n.oid = child.relnamespace
            WHERE k.contype = 'f' AND n.nspname = 'sequent_backend'
                AND parent.relnamespace = child.relnamespace
            "#,
            &[],
        )
        .await
        .context("Error listing foreign keys")?
        .iter()
        .map(|row| {
            (
                row.get::<_, String>("child"),
                row.get::<_, String>("parent"),
            )
        })
        .collect::<Vec<_>>();

    Ok(parents_first(tables, &edges))
}

/// Orders `tables` so that a table comes after every table it references
/// (Kahn's algorithm, ties by name). Tables in a reference cycle keep their
/// name order at the end; a cycle's rows are deleted in one statement per
/// table, and constraints are checked at the end of each statement.
pub fn parents_first(tables: Vec<TenantTable>, edges: &[(String, String)]) -> Vec<TenantTable> {
    let names: BTreeSet<String> = tables.iter().map(|t| t.name.clone()).collect();
    let mut parents: BTreeMap<String, BTreeSet<String>> =
        names.iter().map(|n| (n.clone(), BTreeSet::new())).collect();
    for (child, parent) in edges {
        if child != parent && names.contains(child) && names.contains(parent) {
            parents.get_mut(child).unwrap().insert(parent.clone());
        }
    }
    let mut by_name: HashMap<String, TenantTable> =
        tables.into_iter().map(|t| (t.name.clone(), t)).collect();
    let mut ordered = Vec::with_capacity(by_name.len());
    loop {
        let ready: Vec<String> = parents
            .iter()
            .filter(|(_, p)| p.is_empty())
            .map(|(n, _)| n.clone())
            .collect();
        if ready.is_empty() {
            break;
        }
        for name in ready {
            parents.remove(&name);
            for p in parents.values_mut() {
                p.remove(&name);
            }
            ordered.push(by_name.remove(&name).unwrap());
        }
    }
    let mut rest: Vec<TenantTable> = parents.keys().map(|n| by_name.remove(n).unwrap()).collect();
    ordered.append(&mut rest);
    ordered
}

/// Lets this transaction delete and insert the tenant's rows while it is
/// being reset, and delete the rows the append-only guards keep.
pub async fn allow_tenant_reset(tx: &Transaction<'_>, tenant_id: &str) -> Result<()> {
    let tenant = parse_uuid_v4(tenant_id)?;
    tx.execute(
        "SELECT set_config('sequent.tenant_reset', $1, true)",
        &[&tenant.to_string()],
    )
    .await
    .context("Error allowing the tenant reset")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Resetting,
}

pub async fn lifecycle_state(tx: &Transaction<'_>, tenant_id: &str) -> Result<LifecycleState> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let row = tx
        .query_opt(
            "SELECT lifecycle_state FROM sequent_backend.tenant WHERE id = $1",
            &[&tenant],
        )
        .await
        .context("Error reading the tenant lifecycle state")?
        .ok_or_else(|| anyhow!("tenant {tenant} not found"))?;
    match row.get::<_, String>(0).as_str() {
        "ACTIVE" => Ok(LifecycleState::Active),
        "RESETTING" => Ok(LifecycleState::Resetting),
        other => Err(anyhow!("unknown tenant lifecycle state {other}")),
    }
}

/// Moves the tenant to RESETTING, from ACTIVE or (a retry) RESETTING, and
/// locks its row until the transaction ends.
pub async fn claim_reset(tx: &Transaction<'_>, tenant_id: &str) -> Result<LifecycleState> {
    let before = {
        let tenant = parse_uuid_v4(tenant_id)?;
        tx.query_opt(
            "SELECT lifecycle_state FROM sequent_backend.tenant WHERE id = $1 FOR UPDATE",
            &[&tenant],
        )
        .await?
        .ok_or_else(|| anyhow!("tenant {tenant} not found"))?
        .get::<_, String>(0)
    };
    set_lifecycle_state(tx, tenant_id, LifecycleState::Resetting).await?;
    Ok(if before == "RESETTING" {
        LifecycleState::Resetting
    } else {
        LifecycleState::Active
    })
}

pub async fn set_lifecycle_state(
    tx: &Transaction<'_>,
    tenant_id: &str,
    state: LifecycleState,
) -> Result<()> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let value = match state {
        LifecycleState::Active => "ACTIVE",
        LifecycleState::Resetting => "RESETTING",
    };
    tx.execute(
        "UPDATE sequent_backend.tenant SET lifecycle_state = $2 WHERE id = $1",
        &[&tenant, &value],
    )
    .await
    .context("Error setting the tenant lifecycle state")?;
    Ok(())
}

/// What a reset returns the tenant to.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    /// labels, annotations, is_active, voting_channels and settings without
    /// configuration_signing.
    pub tenant_row: Value,
    /// Tenant-level rows per table name.
    pub rows: Map<String, Value>,
    /// The tenant realm, see `services::tenant_reset::realm`.
    pub realm: Value,
}

pub async fn get_baseline(tx: &Transaction<'_>, tenant_id: &str) -> Result<Option<Baseline>> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let row = tx
        .query_opt(
            r#"SELECT tenant_row, "rows", realm FROM sequent_backend.tenant_reset_baseline
               WHERE tenant_id = $1"#,
            &[&tenant],
        )
        .await
        .context("Error reading the tenant reset baseline")?;
    Ok(row.map(|row| Baseline {
        tenant_row: row.get(0),
        rows: match row.get::<_, Value>(1) {
            Value::Object(map) => map,
            _ => Map::new(),
        },
        realm: row.get(2),
    }))
}

pub async fn has_baseline(tx: &Transaction<'_>, tenant_id: &str) -> Result<bool> {
    let tenant = parse_uuid_v4(tenant_id)?;
    Ok(tx
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM sequent_backend.tenant_reset_baseline WHERE tenant_id = $1)",
            &[&tenant],
        )
        .await?
        .get(0))
}

/// Whether the tenant has ever had a configuration package applied.
pub async fn has_imported(tx: &Transaction<'_>, tenant_id: &str) -> Result<bool> {
    let tenant = parse_uuid_v4(tenant_id)?;
    Ok(tx
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM sequent_backend.configuration_package WHERE tenant_id = $1)",
            &[&tenant],
        )
        .await?
        .get(0))
}

/// Stores `baseline` unless the tenant already has one. Returns whether it
/// was stored.
pub async fn save_baseline(
    tx: &Transaction<'_>,
    tenant_id: &str,
    baseline: &Baseline,
    task_id: Option<&str>,
) -> Result<bool> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let rows = Value::Object(baseline.rows.clone());
    let inserted = tx
        .execute(
            r#"INSERT INTO sequent_backend.tenant_reset_baseline
                   (tenant_id, captured_by_task, tenant_row, "rows", realm)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (tenant_id) DO NOTHING"#,
            &[
                &tenant,
                &task_id,
                &baseline.tenant_row,
                &rows,
                &baseline.realm,
            ],
        )
        .await
        .context("Error saving the tenant reset baseline")?;
    Ok(inserted == 1)
}

/// The tenant row's fields a reset restores.
pub async fn capture_tenant_row(tx: &Transaction<'_>, tenant_id: &str) -> Result<Value> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let row = tx
        .query_opt(
            r#"SELECT jsonb_build_object(
                   'labels', labels,
                   'annotations', annotations,
                   'is_active', is_active,
                   'voting_channels', voting_channels,
                   'settings', settings - 'configuration_signing'
               )
               FROM sequent_backend.tenant WHERE id = $1"#,
            &[&tenant],
        )
        .await?
        .ok_or_else(|| anyhow!("tenant {tenant} not found"))?;
    Ok(row.get(0))
}

/// Restores the tenant row's fields from `tenant_row`, keeping the current
/// configuration_signing (package signing trust survives a reset).
pub async fn restore_tenant_row(
    tx: &Transaction<'_>,
    tenant_id: &str,
    tenant_row: &Value,
) -> Result<()> {
    let tenant = parse_uuid_v4(tenant_id)?;
    tx.execute(
        r#"UPDATE sequent_backend.tenant t SET
               labels = NULLIF($2::jsonb -> 'labels', 'null'::jsonb),
               annotations = NULLIF($2::jsonb -> 'annotations', 'null'::jsonb),
               is_active = coalesce(($2::jsonb ->> 'is_active')::boolean, t.is_active),
               voting_channels = NULLIF($2::jsonb -> 'voting_channels', 'null'::jsonb),
               settings = CASE
                   WHEN t.settings ? 'configuration_signing' THEN
                       coalesce(NULLIF($2::jsonb -> 'settings', 'null'::jsonb), '{}'::jsonb)
                       || jsonb_build_object('configuration_signing', t.settings -> 'configuration_signing')
                   ELSE NULLIF($2::jsonb -> 'settings', 'null'::jsonb)
               END
           WHERE t.id = $1"#,
        &[&tenant, tenant_row],
    )
    .await
    .context("Error restoring the tenant row")?;
    Ok(())
}

fn capture_filter(table: &TenantTable) -> String {
    match table.name.as_str() {
        "document" => " AND r.id NOT IN (SELECT s.document_id \
             FROM sequent_backend.configuration_package_staging s WHERE s.tenant_id = $1)"
            .to_string(),
        "secret" => format!(" AND r.key NOT LIKE '{STAGED_SECRET_PREFIX}%'"),
        _ => String::new(),
    }
}

/// The tenant-level rows (no election event) of every restored table.
#[instrument(skip(tx), err)]
pub async fn capture_rows(tx: &Transaction<'_>, tenant_id: &str) -> Result<Map<String, Value>> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let mut captured = Map::new();
    for table in tenant_tables(tx).await? {
        if table.role != TableRole::Restored {
            continue;
        }
        let event_filter = if table.has_event {
            " AND r.election_event_id IS NULL"
        } else {
            ""
        };
        let sql = format!(
            "SELECT coalesce(jsonb_agg(to_jsonb(r)), '[]'::jsonb) FROM sequent_backend.{} r \
             WHERE r.tenant_id = $1{event_filter}{}",
            quote_ident(&table.name),
            capture_filter(&table),
        );
        let rows: Value = tx
            .query_one(sql.as_str(), &[&tenant])
            .await
            .with_context(|| format!("Error capturing {}", table.name))?
            .get(0);
        if rows.as_array().is_some_and(|r| !r.is_empty()) {
            captured.insert(table.name.clone(), rows);
        }
    }
    Ok(captured)
}

/// Rows deleted per table.
pub type TableCounts = BTreeMap<String, u64>;

/// Deletes the tenant's rows: everything except the preserved tables, the
/// tenant-level rows of its log, its reset tasks (they hold the history and
/// manifest of each reset) and the task `keep_task`. Needs
/// `allow_tenant_reset` in the same transaction.
#[instrument(skip(tx), err)]
pub async fn purge_tenant_rows(
    tx: &Transaction<'_>,
    tenant_id: &str,
    keep_task: Option<Uuid>,
) -> Result<TableCounts> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let mut counts = TableCounts::new();
    let mut tables = tenant_tables(tx).await?;
    tables.reverse();
    for table in tables {
        let name = quote_ident(&table.name);
        let deleted = match table.role {
            TableRole::Preserved => continue,
            TableRole::TenantLog if !table.has_event => continue,
            TableRole::TenantLog => {
                let sql = format!(
                    "DELETE FROM sequent_backend.{name} WHERE tenant_id = $1 \
                     AND election_event_id IS NOT NULL"
                );
                tx.execute(sql.as_str(), &[&tenant]).await
            }
            _ if table.name == "tasks_execution" => {
                tx.execute(
                    "DELETE FROM sequent_backend.tasks_execution WHERE tenant_id = $1 \
                     AND type <> 'RESET_TENANT' AND ($2::uuid IS NULL OR id <> $2)",
                    &[&tenant, &keep_task],
                )
                .await
            }
            _ => {
                let sql = format!("DELETE FROM sequent_backend.{name} WHERE tenant_id = $1");
                tx.execute(sql.as_str(), &[&tenant]).await
            }
        }
        .with_context(|| format!("Error deleting the tenant's rows from {}", table.name))?;
        if deleted > 0 {
            counts.insert(table.name, deleted);
        }
    }
    Ok(counts)
}

/// Inserts the baseline's rows again, parents first. Rows of tables that
/// are no longer restored, or no longer exist, are skipped; columns added
/// since the baseline take their default. Returns rows inserted per table.
#[instrument(skip(tx, rows), err)]
pub async fn restore_rows(
    tx: &Transaction<'_>,
    tenant_id: &str,
    rows: &Map<String, Value>,
) -> Result<TableCounts> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let mut counts = TableCounts::new();
    for table in tenant_tables(tx).await? {
        if table.role != TableRole::Restored {
            continue;
        }
        let Some(Value::Array(table_rows)) = rows.get(&table.name) else {
            continue;
        };
        // Only this tenant's rows, whatever the stored record says.
        let table_rows: Vec<Value> = table_rows
            .iter()
            .filter(|row| row.get("tenant_id").and_then(Value::as_str) == Some(&tenant.to_string()))
            .cloned()
            .collect();
        if table_rows.is_empty() {
            continue;
        }
        let recorded: BTreeSet<&str> = table_rows
            .iter()
            .filter_map(Value::as_object)
            .flat_map(|row| row.keys().map(String::as_str))
            .collect();
        let columns: Vec<String> = insertable_columns(tx, &table.name)
            .await?
            .into_iter()
            .filter(|column| recorded.contains(column.as_str()))
            .map(|column| quote_ident(&column))
            .collect();
        let columns = columns.join(", ");
        let name = quote_ident(&table.name);
        let sql = format!(
            "INSERT INTO sequent_backend.{name} ({columns}) OVERRIDING SYSTEM VALUE \
             SELECT {columns} FROM jsonb_populate_recordset(NULL::sequent_backend.{name}, $1::jsonb)"
        );
        let inserted = tx
            .execute(sql.as_str(), &[&Value::Array(table_rows)])
            .await
            .with_context(|| format!("Error restoring {}", table.name))?;
        counts.insert(table.name, inserted);
    }
    Ok(counts)
}

async fn insertable_columns(tx: &Transaction<'_>, table: &str) -> Result<Vec<String>> {
    Ok(tx
        .query(
            r#"
            SELECT a.attname::text
            FROM pg_attribute a
            JOIN pg_class c ON c.oid = a.attrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = 'sequent_backend' AND c.relname = $1
                AND a.attnum > 0 AND NOT a.attisdropped AND a.attgenerated = ''
            ORDER BY a.attnum
            "#,
            &[&table],
        )
        .await?
        .iter()
        .map(|row| row.get(0))
        .collect())
}

/// Rows the tenant has per table (all of them, any level), for the reset
/// preview and its verification.
pub async fn count_tenant_rows(tx: &Transaction<'_>, tenant_id: &str) -> Result<TableCounts> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let mut counts = TableCounts::new();
    for table in tenant_tables(tx).await? {
        let sql = format!(
            "SELECT count(*) FROM sequent_backend.{} WHERE tenant_id = $1",
            quote_ident(&table.name)
        );
        let count: i64 = tx.query_one(sql.as_str(), &[&tenant]).await?.get(0);
        if count > 0 {
            counts.insert(table.name, count as u64);
        }
    }
    Ok(counts)
}

/// Table names come from the catalog; quoting keeps an odd name from being
/// read as SQL.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(name: &str) -> TenantTable {
        TenantTable {
            name: name.into(),
            has_event: false,
            role: table_role(name),
        }
    }

    fn edge(child: &str, parent: &str) -> (String, String) {
        (child.into(), parent.into())
    }

    #[test]
    fn parents_come_first() {
        let tables = vec![table("c"), table("b"), table("a"), table("d")];
        let edges = [
            edge("c", "b"),
            edge("b", "a"),
            edge("c", "a"),
            edge("a", "a"),
        ];
        let names: Vec<_> = parents_first(tables, &edges)
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(names, ["a", "d", "b", "c"]);
    }

    #[test]
    fn a_cycle_goes_last_and_nothing_is_lost() {
        let tables = vec![table("x"), table("y"), table("z")];
        let edges = [edge("x", "y"), edge("y", "x")];
        let names: Vec<_> = parents_first(tables, &edges)
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(names, ["z", "x", "y"]);
    }

    #[test]
    fn listed_tables_keep_their_role() {
        assert_eq!(
            table_role("managed_webauthn_roster"),
            TableRole::NotRestored
        );
        assert_eq!(table_role("configuration_package"), TableRole::NotRestored);
        assert_eq!(table_role("audit_log_outbox"), TableRole::TenantLog);
        assert_eq!(table_role("tenant_reset_baseline"), TableRole::Preserved);
        assert_eq!(table_role("trustee"), TableRole::Restored);
    }

    #[test]
    fn no_table_has_two_roles() {
        let mut seen = BTreeSet::new();
        for (name, _) in PRESERVED_TABLES
            .iter()
            .chain(TENANT_LOG_TABLES)
            .chain(NOT_RESTORED_TABLES)
        {
            assert!(seen.insert(*name), "{name} is listed twice");
        }
    }

    #[test]
    fn identifiers_are_quoted() {
        assert_eq!(quote_ident("a\"b"), "\"a\"\"b\"");
    }
}
