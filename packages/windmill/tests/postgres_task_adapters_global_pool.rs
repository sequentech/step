// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The task-execution adapters that open their own connection instead of
//! taking a transaction, and the maintenance task. They use the process-wide
//! pools of `get_hasura_pool` and `get_keycloak_pool`, which read
//! `HASURA_DB__*` and `KEYCLOAK_DB__*` once, on first use. This test binary
//! points both at the fixture database before that first use, restores the
//! environment and checks where the pools connect, so these adapters never
//! reach the database the environment names. Their writes commit: each test
//! uses a tenant of its own and deletes its rows at the end.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Pool;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;
use windmill::postgres::{maintenance, tasks_execution};
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};

static FIXTURE: OnceCell<&'static Pool> = OnceCell::const_new();

/// One runtime for every test. Pooled connections are driven by the runtime
/// that opened them, and these pools hand a connection to whichever test asks
/// next, so a runtime per test would close connections other tests still use.
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

fn run<F: Future>(test: F) -> F::Output {
    RUNTIME.block_on(test)
}

async fn current_database(pool: &Pool) -> String {
    pool.get()
        .await
        .unwrap()
        .query_one("SELECT current_database()", &[])
        .await
        .unwrap()
        .get(0)
}

/// The `HASURA_DB__*` and `KEYCLOAK_DB__*` variables.
fn pool_settings() -> Vec<(String, OsString)> {
    std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value)))
        .filter(|(key, _)| key.starts_with("HASURA_DB__") || key.starts_with("KEYCLOAK_DB__"))
        .collect()
}

/// The fixture pool, once both global pools connect to the fixture database.
/// Every test calls this first, so no test reads the environment while it is
/// being changed.
async fn fixture() -> &'static Pool {
    *FIXTURE
        .get_or_init(|| async {
            let pool = schema::pool().await;
            let database = current_database(pool).await;
            let saved = pool_settings();
            // Both pools get the fixture's server settings and database.
            for (key, _) in &saved {
                std::env::remove_var(key);
            }
            for (key, value) in &saved {
                if let Some(setting) = key.strip_prefix("HASURA_DB__") {
                    std::env::set_var(key, value);
                    std::env::set_var(format!("KEYCLOAK_DB__{setting}"), value);
                }
            }
            std::env::set_var("HASURA_DB__DBNAME", &database);
            std::env::set_var("KEYCLOAK_DB__DBNAME", &database);
            let (hasura, keycloak) = (get_hasura_pool().await, get_keycloak_pool().await);
            for (key, _) in pool_settings() {
                std::env::remove_var(key);
            }
            for (key, value) in &saved {
                std::env::set_var(key, value);
            }
            assert_eq!(current_database(&hasura).await, database);
            assert_eq!(current_database(&keycloak).await, database);
            pool
        })
        .await
}

/// Row identifiers of one test. The test's line keeps them apart from the rows
/// of tests running concurrently; `n` tells its own rows apart.
struct Ids(u32);

impl Ids {
    fn id(&self, n: u32) -> String {
        format!("{:08}-0000-4000-8000-{n:012}", self.0)
    }
}

macro_rules! ids {
    () => {
        Ids(line!())
    };
}

/// A committed tenant; `delete` removes it with its tasks.
struct Tenant(String);

impl Tenant {
    async fn new(id: String) -> Tenant {
        fixture()
            .await
            .get()
            .await
            .unwrap()
            .execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1::text::uuid, $1)",
                &[&id],
            )
            .await
            .unwrap();
        Tenant(id)
    }

    async fn delete(self) {
        let client = fixture().await.get().await.unwrap();
        client
            .execute(
                "DELETE FROM sequent_backend.tasks_execution WHERE tenant_id = $1::text::uuid",
                &[&self.0],
            )
            .await
            .unwrap();
        client
            .execute(
                "DELETE FROM sequent_backend.tenant WHERE id = $1::text::uuid",
                &[&self.0],
            )
            .await
            .unwrap();
    }
}

/// The committed row as `to_jsonb` renders it, without its timestamps.
async fn stored(id: &str) -> Value {
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT to_jsonb(r) - 'created_at' - 'start_at' - 'end_at'
             FROM sequent_backend.tasks_execution r WHERE r.id = $1::text::uuid",
            &[&id],
        )
        .await
        .unwrap()
        .get(0)
}

/// Whether the committed row has an end time, and whether that end time is
/// at or after `since`.
async fn ended(id: &str, since: chrono::DateTime<chrono::Local>) -> (bool, bool) {
    let row = fixture()
        .await
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT end_at IS NOT NULL, coalesce(end_at >= $2, false)
             FROM sequent_backend.tasks_execution WHERE id = $1::text::uuid",
            &[&id, &since],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1))
}

/// The server clock, read in a transaction of its own.
async fn server_now() -> chrono::DateTime<chrono::Local> {
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .query_one("SELECT now()", &[])
        .await
        .unwrap()
        .get(0)
}

async fn tasks_named(name: &str) -> i64 {
    fixture()
        .await
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT count(*) FROM sequent_backend.tasks_execution WHERE name = $1",
            &[&name],
        )
        .await
        .unwrap()
        .get(0)
}

fn logs() -> Value {
    json!([{"created_date": "2026-01-01T10:00:00Z", "log_text": "Started"}])
}

/// An export in progress, created through the adapter.
async fn export(tenant: &Tenant, event: &str) -> TasksExecution {
    tasks_execution::insert_tasks_execution(
        &tenant.0,
        Some(event),
        "Export event",
        "EXPORT_ELECTION_EVENT",
        TasksExecutionStatus::IN_PROGRESS,
        Some(json!({"kept": 1, "replaced": 1})),
        Some(json!({"origin": "admin"})),
        Some(logs()),
        "admin-user",
    )
    .await
    .unwrap()
}

#[test]
fn the_global_pools_connect_to_the_migrated_fixture_database() {
    run(async {
        let fixture_database = current_database(fixture().await).await;

        for global in [get_hasura_pool().await, get_keycloak_pool().await] {
            assert_eq!(current_database(&global).await, fixture_database);
            let migrated: bool = global
                .get()
                .await
                .unwrap()
                .query_one(
                    "SELECT to_regclass('sequent_backend.tasks_execution') IS NOT NULL",
                    &[],
                )
                .await
                .unwrap()
                .get(0);
            assert!(migrated);
        }
    })
}

#[test]
fn insert_tasks_execution_commits_the_task_with_its_columns() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;

        let task = export(&tenant, &ids.id(2)).await;

        let row = fixture()
            .await
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT created_at, start_at = created_at AND end_at IS NULL AS open
             FROM sequent_backend.tasks_execution WHERE id = $1::text::uuid",
                &[&task.id],
            )
            .await
            .unwrap();
        let created_at = row.get("created_at");
        assert!(row.get::<_, bool>("open"));
        assert_eq!(
            task,
            TasksExecution {
                id: task.id.clone(),
                tenant_id: ids.id(1),
                election_event_id: Some(ids.id(2)),
                name: "Export event".into(),
                task_type: "EXPORT_ELECTION_EVENT".into(),
                execution_status: "IN_PROGRESS".into(),
                created_at,
                start_at: Some(created_at),
                end_at: None,
                annotations: Some(json!({"kept": 1, "replaced": 1})),
                labels: Some(json!({"origin": "admin"})),
                logs: Some(logs()),
                executed_by_user: "admin-user".into(),
            }
        );
        assert_eq!(
            stored(&task.id).await,
            json!({
                "id": task.id,
                "tenant_id": ids.id(1),
                "election_event_id": ids.id(2),
                "name": "Export event",
                "type": "EXPORT_ELECTION_EVENT",
                "execution_status": "IN_PROGRESS",
                "annotations": {"kept": 1, "replaced": 1},
                "labels": {"origin": "admin"},
                "logs": logs(),
                "executed_by_user": "admin-user",
            })
        );
        tenant.delete().await;
    })
}

#[test]
fn insert_tasks_execution_stores_an_empty_election_event_id_as_null() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;

        let task = tasks_execution::insert_tasks_execution(
            &tenant.0,
            Some(""),
            "Import tenant",
            "IMPORT_TENANT",
            TasksExecutionStatus::IN_PROGRESS,
            None,
            None,
            None,
            "admin-user",
        )
        .await
        .unwrap();

        assert_eq!(task.election_event_id, None);
        let row = stored(&task.id).await;
        assert_eq!(
            (
                &row["election_event_id"],
                &row["annotations"],
                &row["labels"],
                &row["logs"]
            ),
            (&Value::Null, &Value::Null, &Value::Null, &Value::Null)
        );
        tenant.delete().await;
    })
}

#[test]
fn insert_tasks_execution_rejects_an_invalid_tenant_id() {
    run(async {
        fixture().await;

        let error = tasks_execution::insert_tasks_execution(
            "tenant-1",
            None,
            "Rejected tenant task",
            "IMPORT_TENANT",
            TasksExecutionStatus::IN_PROGRESS,
            None,
            None,
            None,
            "admin-user",
        )
        .await
        .unwrap_err();

        assert!(error
            .to_string()
            .starts_with("Error parsing tenant UUID: invalid UUID 'tenant-1'"));
        assert_eq!(tasks_named("Rejected tenant task").await, 0);
    })
}

#[test]
fn insert_tasks_execution_rejects_an_invalid_election_event_id() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;

        let error = tasks_execution::insert_tasks_execution(
            &tenant.0,
            Some("event-1"),
            "Rejected event task",
            "EXPORT_ELECTION_EVENT",
            TasksExecutionStatus::IN_PROGRESS,
            None,
            None,
            None,
            "admin-user",
        )
        .await
        .unwrap_err();

        assert!(error
            .to_string()
            .starts_with("Error parsing election event UUID: invalid UUID 'event-1'"));
        assert_eq!(tasks_named("Rejected event task").await, 0);
        tenant.delete().await;
    })
}

#[test]
fn update_task_execution_status_finishes_the_task_with_new_logs_and_merged_annotations() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;
        let task = export(&tenant, &ids.id(2)).await;
        let before = server_now().await;
        let finished_logs = json!([{"created_date": "2026-01-01T11:00:00Z", "log_text": "Done"}]);

        tasks_execution::update_task_execution_status(
            &tenant.0,
            &task.id,
            TasksExecutionStatus::SUCCESS,
            Some(finished_logs.clone()),
            json!({"replaced": 2, "document_id": "document-1"}),
        )
        .await
        .unwrap();

        let row = stored(&task.id).await;
        assert_eq!(
            (&row["execution_status"], &row["logs"], &row["annotations"]),
            (
                &json!("SUCCESS"),
                &finished_logs,
                &json!({"kept": 1, "replaced": 2, "document_id": "document-1"})
            )
        );
        assert_eq!(ended(&task.id, before).await, (true, true));
        tenant.delete().await;
    })
}

#[test]
fn update_task_execution_status_keeps_the_task_open_while_in_progress() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;
        let task = export(&tenant, &ids.id(2)).await;

        tasks_execution::update_task_execution_status(
            &tenant.0,
            &task.id,
            TasksExecutionStatus::IN_PROGRESS,
            Some(logs()),
            json!({}),
        )
        .await
        .unwrap();

        assert_eq!(stored(&task.id).await["execution_status"], "IN_PROGRESS");
        assert_eq!(ended(&task.id, server_now().await).await, (false, false));
        tenant.delete().await;
    })
}

#[test]
fn update_task_execution_status_without_logs_clears_the_stored_logs() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;
        let task = export(&tenant, &ids.id(2)).await;

        tasks_execution::update_task_execution_status(
            &tenant.0,
            &task.id,
            TasksExecutionStatus::FAILED,
            None,
            json!({}),
        )
        .await
        .unwrap();

        let row = stored(&task.id).await;
        assert_eq!(
            (&row["execution_status"], &row["logs"], &row["annotations"]),
            (
                &json!("FAILED"),
                &Value::Null,
                &json!({"kept": 1, "replaced": 1})
            )
        );
        tenant.delete().await;
    })
}

#[test]
fn update_task_execution_status_leaves_a_task_of_another_tenant_unchanged() {
    run(async {
        let ids = ids!();
        let (tenant, other_tenant) = (Tenant::new(ids.id(1)).await, Tenant::new(ids.id(3)).await);
        let task = export(&other_tenant, &ids.id(2)).await;

        tasks_execution::update_task_execution_status(
            &tenant.0,
            &task.id,
            TasksExecutionStatus::SUCCESS,
            None,
            json!({"replaced": 2}),
        )
        .await
        .unwrap();

        let row = stored(&task.id).await;
        assert_eq!(
            (&row["execution_status"], &row["logs"], &row["annotations"]),
            (
                &json!("IN_PROGRESS"),
                &logs(),
                &json!({"kept": 1, "replaced": 1})
            )
        );
        tenant.delete().await;
        other_tenant.delete().await;
    })
}

#[test]
fn update_task_execution_status_rejects_an_invalid_task_id() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;

        let error = tasks_execution::update_task_execution_status(
            &tenant.0,
            "task-1",
            TasksExecutionStatus::SUCCESS,
            None,
            json!({}),
        )
        .await
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Failed to parse task_execution_id as UUID"
        );
        tenant.delete().await;
    })
}

#[test]
fn update_export_task_execution_status_if_in_progress_finishes_an_active_export() {
    run(async {
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;
        let task = export(&tenant, &ids.id(2)).await;
        let before = server_now().await;

        let updated = tasks_execution::update_export_task_execution_status_if_in_progress(
            &tenant.0,
            &task.id,
            TasksExecutionStatus::SUCCESS,
            None,
            json!({"document_id": "document-1"}),
        )
        .await
        .unwrap();

        assert!(updated);
        let row = stored(&task.id).await;
        assert_eq!(
            (&row["execution_status"], &row["logs"], &row["annotations"]),
            (
                &json!("SUCCESS"),
                &Value::Null,
                &json!({"kept": 1, "replaced": 1, "document_id": "document-1"})
            )
        );
        assert_eq!(ended(&task.id, before).await, (true, true));
        tenant.delete().await;
    })
}

#[test]
fn update_export_task_execution_status_if_in_progress_leaves_a_finished_export_unchanged() {
    run(async {
        // A late or duplicate delivery must not turn a SUCCESS into a FAILED.
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;
        let task = export(&tenant, &ids.id(2)).await;
        assert!(
            tasks_execution::update_export_task_execution_status_if_in_progress(
                &tenant.0,
                &task.id,
                TasksExecutionStatus::SUCCESS,
                Some(logs()),
                json!({"document_id": "document-1"}),
            )
            .await
            .unwrap()
        );

        let updated = tasks_execution::update_export_task_execution_status_if_in_progress(
            &tenant.0,
            &task.id,
            TasksExecutionStatus::FAILED,
            None,
            json!({"document_id": "document-2"}),
        )
        .await
        .unwrap();

        assert!(!updated);
        let row = stored(&task.id).await;
        assert_eq!(
            (&row["execution_status"], &row["logs"], &row["annotations"]),
            (
                &json!("SUCCESS"),
                &logs(),
                &json!({"kept": 1, "replaced": 1, "document_id": "document-1"})
            )
        );
        tenant.delete().await;
    })
}

#[test]
fn update_export_task_execution_status_if_in_progress_is_false_for_another_tenant() {
    run(async {
        let ids = ids!();
        let (tenant, other_tenant) = (Tenant::new(ids.id(1)).await, Tenant::new(ids.id(3)).await);
        let task = export(&other_tenant, &ids.id(2)).await;

        let updated = tasks_execution::update_export_task_execution_status_if_in_progress(
            &tenant.0,
            &task.id,
            TasksExecutionStatus::FAILED,
            None,
            json!({}),
        )
        .await
        .unwrap();

        assert!(!updated);
        assert_eq!(stored(&task.id).await["execution_status"], "IN_PROGRESS");
        tenant.delete().await;
        other_tenant.delete().await;
    })
}

#[test]
fn get_task_by_id_finds_a_task_by_its_id_alone() {
    run(async {
        // No tenant is given: any tenant's task is returned.
        let ids = ids!();
        let tenant = Tenant::new(ids.id(1)).await;
        let task = export(&tenant, &ids.id(2)).await;

        let found = tasks_execution::get_task_by_id(&task.id).await.unwrap();

        assert_eq!(found, task);
        tenant.delete().await;
    })
}

#[test]
fn get_task_by_id_fails_for_a_missing_task() {
    run(async {
        let ids = ids!();
        fixture().await;

        let error = tasks_execution::get_task_by_id(&ids.id(10))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Error fetching task: query returned an unexpected number of rows"
        );
    })
}

#[test]
fn get_task_by_id_rejects_an_invalid_task_id() {
    run(async {
        fixture().await;

        let error = tasks_execution::get_task_by_id("task-1").await.unwrap_err();

        assert!(error
            .to_string()
            .starts_with("Error parsing task UUID: invalid UUID 'task-1'"));
    })
}

#[test]
fn vacuum_analyze_direct_runs_on_both_configured_databases() {
    run(async {
        fixture().await;

        maintenance::vacuum_analyze_direct().await.unwrap();
    })
}
