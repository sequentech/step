// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The PostgreSQL adapters of tasks, schedules, reports, documents and locks
//! that take a transaction, against the migrated schema. Each test writes its
//! own rows in a transaction, checks what the adapter returns and what it
//! stored, and rolls back.

// The render-report future nests as deeply as the library's, which sets the same.
#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::types::hasura::core::{
    Document, DocumentAnnotations, SupportMaterial, TasksExecution,
};
use sequent_core::types::scheduled_event::{CronConfig, EventProcessors, ScheduledEvent};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use tokio_postgres::error::SqlState;
use uuid::Uuid;
use windmill::postgres::reports::{Report, ReportCronConfig, ReportType};
use windmill::postgres::{
    document, lock, render_report, reports, scheduled_event, tasks_execution,
};
use windmill::services::reports::template_renderer::EReportEncryption;
use windmill::tasks::render_report::{FormatType, RenderTemplateBody};

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

/// A tenant with two election events, and another tenant with one, so every
/// query can be checked against rows it must not return or change.
struct World {
    ids: Ids,
    tenant: String,
    event: String,
    other_event: String,
    other_tenant: String,
    other_tenant_event: String,
}

impl World {
    async fn new(tx: &Transaction<'_>, ids: Ids) -> World {
        let world = World {
            tenant: ids.id(1),
            event: ids.id(2),
            other_event: ids.id(3),
            other_tenant: ids.id(4),
            other_tenant_event: ids.id(5),
            ids,
        };
        for tenant in [&world.tenant, &world.other_tenant] {
            tx.execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1::text::uuid, $1)",
                &[tenant],
            )
            .await
            .unwrap();
        }
        for (tenant, event) in [
            (&world.tenant, &world.event),
            (&world.tenant, &world.other_event),
            (&world.other_tenant, &world.other_tenant_event),
        ] {
            tx.execute(
                "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
                 VALUES ($1::text::uuid, $2::text::uuid, 'RSA256')",
                &[event, tenant],
            )
            .await
            .unwrap();
        }
        world
    }

    /// A row of this test; numbers below 10 name the world itself.
    fn id(&self, n: u32) -> String {
        self.ids.id(n)
    }

    /// Whether a row id belongs to this test, for queries across tenants.
    fn owns(&self, id: &str) -> bool {
        id.starts_with(&format!("{:08}-", self.ids.0))
    }
}

/// Fixed times: rows are ordered and compared by these, never by the clock.
const H10: &str = "2026-01-01T10:00:00Z";
const NEXT_DAY: &str = "2026-01-02T10:00:00Z";

fn at(timestamp: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(timestamp)
        .unwrap()
        .with_timezone(&Local)
}

fn utc(timestamp: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(timestamp)
        .unwrap()
        .with_timezone(&Utc)
}

/// The start of the transaction, which `now()` and column defaults record.
async fn now(tx: &Transaction<'_>) -> DateTime<Utc> {
    tx.query_one("SELECT now()", &[]).await.unwrap().get(0)
}

async fn set(tx: &Transaction<'_>, table: &str, id: &str, assignments: &str) {
    tx.batch_execute(&format!(
        "UPDATE sequent_backend.{table} SET {assignments} WHERE id = '{id}'"
    ))
    .await
    .unwrap();
}

/// The row of `table` with this id as `to_jsonb` renders it, without the
/// timestamp columns listed.
async fn stored(tx: &Transaction<'_>, table: &str, id: &str, timestamps: &[&str]) -> Value {
    let row: Value = tx
        .query_one(
            &format!(
                "SELECT to_jsonb(r) FROM sequent_backend.{table} r WHERE r.id = $1::text::uuid"
            ),
            &[&id],
        )
        .await
        .unwrap()
        .get(0);
    let Value::Object(mut columns) = row else {
        panic!("{table} row is not an object");
    };
    for timestamp in timestamps {
        columns.remove(*timestamp);
    }
    Value::Object(columns)
}

async fn count(tx: &Transaction<'_>, table: &str, tenant: &str) -> i64 {
    tx.query_one(
        &format!("SELECT count(*) FROM sequent_backend.{table} WHERE tenant_id = $1::text::uuid"),
        &[&tenant],
    )
    .await
    .unwrap()
    .get(0)
}

fn sql_state(error: &anyhow::Error) -> Option<SqlState> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
        .and_then(|error| error.code().cloned())
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

// tasks_execution

/// An export in progress, with every optional column left NULL.
async fn task_row(tx: &Transaction<'_>, tenant: &str, event: Option<&str>, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.tasks_execution
             (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, 'Export event',
                 'EXPORT_ELECTION_EVENT', 'IN_PROGRESS', 'admin-user')",
        &[&id, &tenant, &event],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn merge_task_execution_annotations_merges_into_the_stored_annotations() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    task_row(&tx, &w.tenant, Some(&w.event), &w.id(10)).await;
    set(
        &tx,
        "tasks_execution",
        &w.id(10),
        "annotations = '{\"kept\": 1, \"replaced\": 1}'",
    )
    .await;

    tasks_execution::merge_task_execution_annotations(
        &tx,
        &w.tenant,
        &w.id(10),
        &json!({"replaced": 2, "retry": {"attempt": 1}}),
    )
    .await
    .unwrap();

    assert_eq!(
        stored(&tx, "tasks_execution", &w.id(10), &[]).await["annotations"],
        json!({"kept": 1, "replaced": 2, "retry": {"attempt": 1}})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn merge_task_execution_annotations_creates_annotations_when_none_are_stored() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    task_row(&tx, &w.tenant, None, &w.id(10)).await;

    tasks_execution::merge_task_execution_annotations(&tx, &w.tenant, &w.id(10), &json!({"a": 1}))
        .await
        .unwrap();

    assert_eq!(
        stored(&tx, "tasks_execution", &w.id(10), &[]).await["annotations"],
        json!({"a": 1})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn merge_task_execution_annotations_fails_for_a_task_of_another_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    task_row(&tx, &w.other_tenant, None, &w.id(10)).await;

    let error = tasks_execution::merge_task_execution_annotations(
        &tx,
        &w.tenant,
        &w.id(10),
        &json!({"a": 1}),
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Expected to update one task execution, updated 0"
    );
    assert_eq!(
        stored(&tx, "tasks_execution", &w.id(10), &[]).await["annotations"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn merge_task_execution_annotations_rejects_an_invalid_task_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error =
        tasks_execution::merge_task_execution_annotations(&tx, &w.tenant, "task-1", &json!({}))
            .await
            .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Failed to parse task_execution_id as UUID"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_task_by_id_with_transaction_returns_the_task_with_its_columns() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    task_row(&tx, &w.tenant, Some(&w.event), &w.id(10)).await;
    set(
        &tx,
        "tasks_execution",
        &w.id(10),
        &format!(
            "execution_status = 'SUCCESS', created_at = '{H10}', start_at = '{H10}',
             end_at = '{NEXT_DAY}', annotations = '{{\"document_id\": \"d\"}}',
             labels = '{{\"origin\": \"admin\"}}', logs = '[{{\"log_text\": \"Done\"}}]'"
        ),
    )
    .await;

    let task = tasks_execution::get_task_by_id_with_transaction(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    assert_eq!(
        task,
        TasksExecution {
            id: w.id(10),
            tenant_id: w.tenant.clone(),
            election_event_id: Some(w.event.clone()),
            name: "Export event".into(),
            task_type: "EXPORT_ELECTION_EVENT".into(),
            execution_status: "SUCCESS".into(),
            created_at: at(H10),
            start_at: Some(at(H10)),
            end_at: Some(at(NEXT_DAY)),
            annotations: Some(json!({"document_id": "d"})),
            labels: Some(json!({"origin": "admin"})),
            logs: Some(json!([{"log_text": "Done"}])),
            executed_by_user: "admin-user".into(),
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_task_by_id_with_transaction_fails_for_a_task_of_another_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    task_row(&tx, &w.other_tenant, None, &w.id(10)).await;

    let error = tasks_execution::get_task_by_id_with_transaction(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Error fetching task: query returned an unexpected number of rows"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_export_task_with_transaction_makes_another_attempt_at_the_task_wait() {
    // Advisory locks are shared across sessions, so no row has to be committed.
    let pool = schema::pool().await;
    let ids = ids!();
    let (task, other_task) = (ids.id(10), ids.id(11));
    let mut holder = pool.get().await.unwrap();
    let held = holder.transaction().await.unwrap();
    tasks_execution::lock_export_task_with_transaction(&held, &task)
        .await
        .unwrap();

    let mut contender = pool.get().await.unwrap();
    let waiting = contender.transaction().await.unwrap();
    waiting
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    tasks_execution::lock_export_task_with_transaction(&waiting, &other_task)
        .await
        .unwrap();
    let error = tasks_execution::lock_export_task_with_transaction(&waiting, &task)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Error locking export task");
    assert_eq!(sql_state(&error), Some(SqlState::LOCK_NOT_AVAILABLE));
    waiting.rollback().await.unwrap();
    held.rollback().await.unwrap();

    let retry = contender.transaction().await.unwrap();
    retry
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    tasks_execution::lock_export_task_with_transaction(&retry, &task)
        .await
        .unwrap();
    retry.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tasks_by_election_event_id_returns_the_event_tasks_of_the_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    task_row(&tx, &w.tenant, Some(&w.event), &w.id(10)).await;
    task_row(&tx, &w.tenant, Some(&w.event), &w.id(11)).await;
    task_row(&tx, &w.tenant, Some(&w.other_event), &w.id(12)).await;
    task_row(&tx, &w.tenant, None, &w.id(13)).await;
    // No foreign key ties a task's event to its tenant.
    task_row(&tx, &w.other_tenant, Some(&w.event), &w.id(14)).await;

    let tasks = tasks_execution::get_tasks_by_election_event_id(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(
        sorted(tasks.into_iter().map(|task| task.id).collect()),
        [w.id(10), w.id(11)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tasks_by_election_event_id_calls_an_invalid_event_id_a_task_uuid() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = tasks_execution::get_tasks_by_election_event_id(&tx, &w.tenant, "event-1")
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .starts_with("Error parsing task UUID: invalid UUID 'event-1'"));
    tx.rollback().await.unwrap();
}

// scheduled_event

/// A report schedule with no stop or archive time.
async fn schedule_row(
    tx: &Transaction<'_>,
    tenant: Option<&str>,
    event: Option<&str>,
    id: &str,
    task_id: &str,
) {
    tx.execute(
        "INSERT INTO sequent_backend.scheduled_event
             (id, tenant_id, election_event_id, created_at, event_processor, task_id,
              cron_config, event_payload)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, '2026-01-01T10:00:00Z',
                 'CREATE_REPORT', $4,
                 '{\"cron\": null, \"scheduled_date\": \"2026-02-01T10:00:00Z\"}',
                 '{\"report_id\": \"report-1\"}')",
        &[&id, &tenant, &event, &task_id],
    )
    .await
    .unwrap();
}

fn cron(scheduled_date: &str) -> CronConfig {
    CronConfig {
        cron: None,
        scheduled_date: Some(scheduled_date.into()),
    }
}

fn schedule_ids(schedules: Vec<ScheduledEvent>) -> Vec<String> {
    sorted(schedules.into_iter().map(|schedule| schedule.id).collect())
}

/// `stopped_at` and `archived_at` of a schedule.
async fn schedule_times(
    tx: &Transaction<'_>,
    id: &str,
) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let row = tx
        .query_one(
            "SELECT stopped_at, archived_at FROM sequent_backend.scheduled_event
             WHERE id = $1::text::uuid",
            &[&id],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1))
}

#[tokio::test]
async fn insert_scheduled_event_returns_and_stores_the_schedule() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let inserted = scheduled_event::insert_scheduled_event(
        &tx,
        &w.tenant,
        &w.event,
        EventProcessors::SEND_TEMPLATE,
        "send-template-1",
        cron("2026-02-01T10:00:00Z"),
        json!({"template_id": "template-1"}),
    )
    .await
    .unwrap();

    assert_eq!(
        inserted,
        ScheduledEvent {
            id: inserted.id.clone(),
            tenant_id: Some(w.tenant.clone()),
            election_event_id: Some(w.event.clone()),
            created_at: Some(now(&tx).await),
            stopped_at: None,
            archived_at: None,
            labels: None,
            annotations: None,
            event_processor: Some(EventProcessors::SEND_TEMPLATE),
            cron_config: Some(cron("2026-02-01T10:00:00Z")),
            event_payload: Some(json!({"template_id": "template-1"})),
            task_id: Some("send-template-1".into()),
        }
    );
    assert_eq!(
        stored(&tx, "scheduled_event", &inserted.id, &["created_at"]).await,
        json!({
            "id": inserted.id,
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "stopped_at": null,
            "archived_at": null,
            "labels": null,
            "annotations": null,
            "event_processor": "SEND_TEMPLATE",
            "event_payload": {"template_id": "template-1"},
            "created_by": null,
            "task_id": "send-template-1",
            "cron_config": {"cron": null, "scheduled_date": "2026-02-01T10:00:00Z"},
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_scheduled_event_rejects_an_invalid_election_event_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = scheduled_event::insert_scheduled_event(
        &tx,
        &w.tenant,
        "event-1",
        EventProcessors::CREATE_REPORT,
        "report-task",
        cron("2026-02-01T10:00:00Z"),
        json!({}),
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "Error parsing election_event_id as UUID");
    assert_eq!(count(&tx, "scheduled_event", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_new_scheduled_event_keeps_every_supplied_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let schedule = ScheduledEvent {
        id: w.id(10),
        tenant_id: Some(w.tenant.clone()),
        election_event_id: Some(w.event.clone()),
        created_at: Some(utc(H10)),
        stopped_at: Some(utc(NEXT_DAY)),
        archived_at: None,
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": 1})),
        event_processor: Some(EventProcessors::ALLOW_TALLY),
        cron_config: Some(cron("2026-02-01T10:00:00Z")),
        event_payload: Some(json!({"election_id": w.id(20)})),
        task_id: Some("allow-tally-1".into()),
    };

    let inserted = scheduled_event::insert_new_scheduled_event(&tx, schedule.clone())
        .await
        .unwrap();

    assert_eq!(inserted, schedule);
    assert_eq!(
        stored(
            &tx,
            "scheduled_event",
            &w.id(10),
            &["created_at", "stopped_at"]
        )
        .await,
        json!({
            "id": w.id(10),
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "archived_at": null,
            "labels": {"label": 1},
            "annotations": {"note": 1},
            "event_processor": "ALLOW_TALLY",
            "event_payload": {"election_id": w.id(20)},
            "created_by": null,
            "task_id": "allow-tally-1",
            "cron_config": {"cron": null, "scheduled_date": "2026-02-01T10:00:00Z"},
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_new_scheduled_event_accepts_a_schedule_without_tenant_or_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let schedule = ScheduledEvent {
        id: w.id(10),
        tenant_id: None,
        election_event_id: None,
        created_at: None,
        stopped_at: None,
        archived_at: None,
        labels: None,
        annotations: None,
        event_processor: None,
        cron_config: None,
        event_payload: None,
        task_id: None,
    };

    let inserted = scheduled_event::insert_new_scheduled_event(&tx, schedule.clone())
        .await
        .unwrap();

    // The column list names created_at, so its default does not apply.
    assert_eq!(inserted, schedule);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_all_active_events_returns_every_unstopped_schedule_of_any_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.other_tenant_event),
        &w.id(11),
        "task-11",
    )
    .await;
    schedule_row(&tx, None, None, &w.id(12), "task-12").await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(13), "task-13").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(13),
        &format!("stopped_at = '{H10}'"),
    )
    .await;
    // Archiving normally stops a schedule too; one archived but never stopped
    // is still active here.
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(14), "task-14").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(14),
        &format!("archived_at = '{H10}'"),
    )
    .await;

    let active = scheduled_event::find_all_active_events(&tx).await.unwrap();

    assert_eq!(
        schedule_ids(
            active
                .into_iter()
                .filter(|schedule| w.owns(&schedule.id))
                .collect()
        ),
        [w.id(10), w.id(11), w.id(12), w.id(14)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_id_maps_the_schedule_columns() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;

    let found = scheduled_event::find_scheduled_event_by_id(
        &tx,
        Some(w.tenant.clone()),
        Some(w.event.clone()),
        &w.id(10),
    )
    .await
    .unwrap();

    assert_eq!(
        found,
        Some(ScheduledEvent {
            id: w.id(10),
            tenant_id: Some(w.tenant.clone()),
            election_event_id: Some(w.event.clone()),
            created_at: Some(utc(H10)),
            stopped_at: None,
            archived_at: None,
            labels: None,
            annotations: None,
            event_processor: Some(EventProcessors::CREATE_REPORT),
            cron_config: Some(cron("2026-02-01T10:00:00Z")),
            event_payload: Some(json!({"report_id": "report-1"})),
            task_id: Some("task-10".into()),
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_id_skips_stopped_and_archived_schedules() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(10),
        &format!("stopped_at = '{H10}'"),
    )
    .await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(11), "task-11").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(11),
        &format!("archived_at = '{H10}'"),
    )
    .await;

    for id in [w.id(10), w.id(11)] {
        let found = scheduled_event::find_scheduled_event_by_id(
            &tx,
            Some(w.tenant.clone()),
            Some(w.event.clone()),
            &id,
        )
        .await
        .unwrap();
        assert_eq!(found, None, "{id}");
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_id_without_tenant_or_event_matches_any_scope() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.other_tenant_event),
        &w.id(10),
        "task-10",
    )
    .await;

    let found = scheduled_event::find_scheduled_event_by_id(&tx, None, None, &w.id(10))
        .await
        .unwrap();

    assert_eq!(found.map(|schedule| schedule.id), Some(w.id(10)));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_id_is_none_for_a_schedule_of_another_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.other_tenant_event),
        &w.id(10),
        "task-10",
    )
    .await;

    let found =
        scheduled_event::find_scheduled_event_by_id(&tx, Some(w.tenant.clone()), None, &w.id(10))
            .await
            .unwrap();

    assert_eq!(found, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_id_fails_on_an_unknown_event_processor() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(10),
        "event_processor = 'CLOSE_EVERYTHING'",
    )
    .await;

    let error = scheduled_event::find_scheduled_event_by_id(&tx, None, None, &w.id(10))
        .await
        .unwrap_err();

    assert_eq!(
        format!("{error:#}"),
        "Error converting rows into ScheduledEvent: \
         Error mapping \"CLOSE_EVERYTHING\" into an EventProcessor: VariantNotFound"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_task_id_returns_a_stopped_but_not_an_archived_schedule() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(
        &tx,
        Some(&w.tenant),
        Some(&w.event),
        &w.id(10),
        "stopped-task",
    )
    .await;
    set(
        &tx,
        "scheduled_event",
        &w.id(10),
        &format!("stopped_at = '{H10}'"),
    )
    .await;
    schedule_row(
        &tx,
        Some(&w.tenant),
        Some(&w.event),
        &w.id(11),
        "archived-task",
    )
    .await;
    set(
        &tx,
        "scheduled_event",
        &w.id(11),
        &format!("stopped_at = '{H10}', archived_at = '{H10}'"),
    )
    .await;

    let stopped =
        scheduled_event::find_scheduled_event_by_task_id(&tx, &w.tenant, &w.event, "stopped-task")
            .await
            .unwrap();
    let archived =
        scheduled_event::find_scheduled_event_by_task_id(&tx, &w.tenant, &w.event, "archived-task")
            .await
            .unwrap();

    assert_eq!(
        stopped.map(|schedule| (schedule.id, schedule.stopped_at)),
        Some((w.id(10), Some(utc(H10))))
    );
    assert_eq!(archived, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_task_id_only_looks_in_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(
        &tx,
        Some(&w.tenant),
        Some(&w.other_event),
        &w.id(10),
        "task-1",
    )
    .await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.event),
        &w.id(11),
        "task-1",
    )
    .await;

    let found =
        scheduled_event::find_scheduled_event_by_task_id(&tx, &w.tenant, &w.event, "task-1")
            .await
            .unwrap();

    assert_eq!(found, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn stop_scheduled_event_stops_only_the_target_schedule() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(11), "task-11").await;

    scheduled_event::stop_scheduled_event(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    assert_eq!(
        schedule_times(&tx, &w.id(10)).await,
        (Some(now(&tx).await), None)
    );
    assert_eq!(schedule_times(&tx, &w.id(11)).await, (None, None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn stop_scheduled_event_keeps_the_first_stop_time() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(10),
        &format!("stopped_at = '{H10}'"),
    )
    .await;

    scheduled_event::stop_scheduled_event(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    assert_eq!(schedule_times(&tx, &w.id(10)).await, (Some(utc(H10)), None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn stop_scheduled_event_leaves_a_schedule_of_another_tenant_running() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.other_tenant_event),
        &w.id(10),
        "task-10",
    )
    .await;

    scheduled_event::stop_scheduled_event(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    assert_eq!(schedule_times(&tx, &w.id(10)).await, (None, None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn archive_scheduled_event_stops_and_archives_only_the_target_schedule() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(11), "task-11").await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.other_tenant_event),
        &w.id(12),
        "task-12",
    )
    .await;

    scheduled_event::archive_scheduled_event(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();
    scheduled_event::archive_scheduled_event(&tx, &w.tenant, &w.id(12))
        .await
        .unwrap();

    let started = now(&tx).await;
    assert_eq!(
        schedule_times(&tx, &w.id(10)).await,
        (Some(started), Some(started))
    );
    for untouched in [w.id(11), w.id(12)] {
        assert_eq!(schedule_times(&tx, &untouched).await, (None, None));
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn archive_scheduled_event_overwrites_an_earlier_stop_time() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(10),
        &format!("stopped_at = '{H10}'"),
    )
    .await;

    scheduled_event::archive_scheduled_event(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    let started = now(&tx).await;
    assert_eq!(
        schedule_times(&tx, &w.id(10)).await,
        (Some(started), Some(started))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_scheduled_event_replaces_the_cron_config_and_keeps_the_payload() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(11), "task-11").await;

    scheduled_event::update_scheduled_event(
        &tx,
        &w.tenant,
        &w.id(10),
        CronConfig {
            cron: Some("0 10 * * *".into()),
            scheduled_date: None,
        },
        None,
    )
    .await
    .unwrap();

    let updated = stored(&tx, "scheduled_event", &w.id(10), &["created_at"]).await;
    assert_eq!(
        (&updated["cron_config"], &updated["event_payload"]),
        (
            &json!({"cron": "0 10 * * *", "scheduled_date": null}),
            &json!({"report_id": "report-1"})
        )
    );
    assert_eq!(
        stored(&tx, "scheduled_event", &w.id(11), &["created_at"]).await["cron_config"],
        json!({"cron": null, "scheduled_date": "2026-02-01T10:00:00Z"})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_scheduled_event_adds_the_voting_channels_to_the_payload() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(11), "task-11").await;
    set(&tx, "scheduled_event", &w.id(11), "event_payload = NULL").await;
    let channels = vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK];

    for id in [w.id(10), w.id(11)] {
        scheduled_event::update_scheduled_event(
            &tx,
            &w.tenant,
            &id,
            cron("2026-03-01T10:00:00Z"),
            Some(&channels),
        )
        .await
        .unwrap();
    }

    assert_eq!(
        stored(&tx, "scheduled_event", &w.id(10), &["created_at"]).await["event_payload"],
        json!({"report_id": "report-1", "voting_channels": ["ONLINE", "KIOSK"]})
    );
    assert_eq!(
        stored(&tx, "scheduled_event", &w.id(11), &["created_at"]).await["event_payload"],
        json!({"voting_channels": ["ONLINE", "KIOSK"]})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_scheduled_event_leaves_a_stopped_schedule_unchanged() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(10),
        &format!("stopped_at = '{H10}'"),
    )
    .await;

    scheduled_event::update_scheduled_event(
        &tx,
        &w.tenant,
        &w.id(10),
        cron("2026-03-01T10:00:00Z"),
        Some(&vec![VotingStatusChannel::ONLINE]),
    )
    .await
    .unwrap();

    let row = stored(
        &tx,
        "scheduled_event",
        &w.id(10),
        &["created_at", "stopped_at"],
    )
    .await;
    assert_eq!(
        (&row["cron_config"], &row["event_payload"]),
        (
            &json!({"cron": null, "scheduled_date": "2026-02-01T10:00:00Z"}),
            &json!({"report_id": "report-1"})
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_election_event_id_returns_the_unarchived_event_schedules() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(11), "task-11").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(11),
        &format!("stopped_at = '{H10}'"),
    )
    .await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(12), "task-12").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(12),
        &format!("archived_at = '{H10}'"),
    )
    .await;
    schedule_row(
        &tx,
        Some(&w.tenant),
        Some(&w.other_event),
        &w.id(13),
        "task-13",
    )
    .await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.event),
        &w.id(14),
        "task-14",
    )
    .await;

    let schedules =
        scheduled_event::find_scheduled_event_by_election_event_id(&tx, &w.tenant, &w.event)
            .await
            .unwrap();

    assert_eq!(schedule_ids(schedules), [w.id(10), w.id(11)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn find_scheduled_event_by_election_event_id_and_event_processor_returns_its_unarchived_schedules(
) {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(10), "task-10").await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(11), "task-11").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(11),
        &format!("stopped_at = '{H10}'"),
    )
    .await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(12), "task-12").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(12),
        &format!("archived_at = '{H10}'"),
    )
    .await;
    schedule_row(&tx, Some(&w.tenant), Some(&w.event), &w.id(13), "task-13").await;
    set(
        &tx,
        "scheduled_event",
        &w.id(13),
        "event_processor = 'SEND_TEMPLATE'",
    )
    .await;
    schedule_row(
        &tx,
        Some(&w.tenant),
        Some(&w.other_event),
        &w.id(14),
        "task-14",
    )
    .await;
    schedule_row(
        &tx,
        Some(&w.other_tenant),
        Some(&w.event),
        &w.id(15),
        "task-15",
    )
    .await;

    let schedules = scheduled_event::find_scheduled_event_by_election_event_id_and_event_processor(
        &tx,
        &w.tenant,
        &w.event,
        "CREATE_REPORT",
    )
    .await
    .unwrap();

    assert_eq!(schedule_ids(schedules), [w.id(10), w.id(11)]);
    tx.rollback().await.unwrap();
}

// reports

/// A report of `report_type` in the event, with no election, alias, cron
/// config or labels.
async fn report_row(tx: &Transaction<'_>, tenant: &str, event: &str, id: &str, report_type: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.report
             (id, tenant_id, election_event_id, report_type, created_at)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4, '2026-01-01T10:00:00Z')",
        &[&id, &tenant, &event, &report_type],
    )
    .await
    .unwrap();
}

fn report(w: &World, n: u32) -> Report {
    Report {
        id: w.id(n),
        election_event_id: w.event.clone(),
        tenant_id: w.tenant.clone(),
        election_id: Some(w.id(20)),
        report_type: ReportType::ELECTORAL_RESULTS.to_string(),
        template_alias: Some("results-template".into()),
        encryption_policy: EReportEncryption::ConfiguredPassword,
        cron_config: Some(ReportCronConfig {
            is_active: true,
            last_document_produced: None,
            cron_expression: "46 0 * * *".into(),
            email_recipients: vec!["ops@example.org".into()],
            executer_username: "admin".into(),
        }),
        created_at: utc(H10),
        permission_label: Some(vec!["north".into()]),
    }
}

/// A report as it serializes, which also shows every field of a type that
/// has no `PartialEq`.
fn json_of(report: &Report) -> Value {
    serde_json::to_value(report).unwrap()
}

fn report_ids(reports: Vec<Report>) -> Vec<String> {
    sorted(reports.into_iter().map(|report| report.id).collect())
}

#[tokio::test]
async fn insert_reports_stores_every_report_with_its_columns() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    reports::insert_reports(&tx, &w.tenant, &w.event, &[report(&w, 10), report(&w, 11)])
        .await
        .unwrap();

    assert_eq!(
        stored(&tx, "report", &w.id(10), &["created_at"]).await,
        json!({
            "id": w.id(10),
            "election_event_id": w.event,
            "tenant_id": w.tenant,
            "election_id": w.id(20),
            "report_type": "ELECTORAL_RESULTS",
            "cron_config": {
                "is_active": true,
                "last_document_produced": null,
                "cron_expression": "46 0 * * *",
                "email_recipients": ["ops@example.org"],
                "executer_username": "admin",
            },
            "encryption_policy": "configured_password",
            "template_alias": "results-template",
            "permission_label": ["north"],
        })
    );
    let found = reports::get_report_by_id(&tx, &w.tenant, &w.id(11))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(json_of(&found), json_of(&report(&w, 11)));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_reports_files_reports_under_the_given_tenant_and_event() {
    // The tenant and event arguments win over the reports' own fields.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let foreign = Report {
        tenant_id: w.other_tenant.clone(),
        election_event_id: w.other_tenant_event.clone(),
        ..report(&w, 10)
    };

    reports::insert_reports(&tx, &w.tenant, &w.event, &[foreign])
        .await
        .unwrap();

    let row = stored(&tx, "report", &w.id(10), &["created_at"]).await;
    assert_eq!(
        (&row["tenant_id"], &row["election_event_id"]),
        (&json!(w.tenant), &json!(w.event))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_reports_reads_back_a_missing_cron_config_as_the_default() {
    // None is stored as JSON null, which reads back as a default cron config.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let without_cron = Report {
        cron_config: None,
        ..report(&w, 10)
    };

    reports::insert_reports(&tx, &w.tenant, &w.event, &[without_cron])
        .await
        .unwrap();

    let column: Option<String> = tx
        .query_one(
            "SELECT jsonb_typeof(cron_config) FROM sequent_backend.report
             WHERE id = $1::text::uuid",
            &[&w.id(10)],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(column.as_deref(), Some("null"));
    let found = reports::get_report_by_id(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.cron_config, Some(ReportCronConfig::default()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_report_by_id_is_none_for_a_report_of_another_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(
        &tx,
        &w.other_tenant,
        &w.other_tenant_event,
        &w.id(10),
        "ELECTORAL_RESULTS",
    )
    .await;

    let found = reports::get_report_by_id(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    assert!(found.is_none());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_report_by_id_reads_an_unreadable_cron_config_as_the_default() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    set(
        &tx,
        "report",
        &w.id(10),
        "cron_config = '{\"is_active\": \"yes\"}'",
    )
    .await;

    let found = reports::get_report_by_id(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.cron_config, Some(ReportCronConfig::default()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_report_by_id_fails_on_an_unknown_encryption_policy() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    set(
        &tx,
        "report",
        &w.id(10),
        "encryption_policy = 'generated_password'",
    )
    .await;

    let error = reports::get_report_by_id(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap_err();

    assert!(error.to_string().starts_with(
        "Error converting rows into Report: error deserializing encryption_policy: \
         VariantNotFound \"generated_password\""
    ));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_all_active_reports_returns_the_reports_with_an_active_cron_config() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for (n, tenant, event, cron_config) in [
        (10, &w.tenant, &w.event, "'{\"is_active\": true}'"),
        (
            11,
            &w.other_tenant,
            &w.other_tenant_event,
            "'{\"is_active\": true}'",
        ),
        (12, &w.tenant, &w.event, "'{\"is_active\": false}'"),
        (13, &w.tenant, &w.event, "'{}'"),
        (14, &w.tenant, &w.event, "NULL"),
    ] {
        report_row(&tx, tenant, event, &w.id(n), "ELECTORAL_RESULTS").await;
        set(
            &tx,
            "report",
            &w.id(n),
            &format!("cron_config = {cron_config}"),
        )
        .await;
    }

    let active = reports::get_all_active_reports(&tx).await.unwrap();

    assert_eq!(
        report_ids(
            active
                .into_iter()
                .filter(|report| w.owns(&report.id))
                .collect()
        ),
        [w.id(10), w.id(11)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_report_last_document_time_records_the_transaction_time_in_utc() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    set(
        &tx,
        "report",
        &w.id(10),
        "cron_config = '{\"is_active\": true, \"cron_expression\": \"46 0 * * *\"}'",
    )
    .await;

    reports::update_report_last_document_time(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    let produced = now(&tx).await.format("%Y-%m-%dT%H:%M:%S%.6f").to_string();
    assert_eq!(
        stored(&tx, "report", &w.id(10), &["created_at"]).await["cron_config"],
        json!({
            "is_active": true,
            "cron_expression": "46 0 * * *",
            "last_document_produced": produced,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_report_last_document_time_creates_a_cron_config_when_none_is_stored() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;

    reports::update_report_last_document_time(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap();

    let produced = now(&tx).await.format("%Y-%m-%dT%H:%M:%S%.6f").to_string();
    assert_eq!(
        stored(&tx, "report", &w.id(10), &["created_at"]).await["cron_config"],
        json!({"last_document_produced": produced})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_report_last_document_time_fails_for_a_report_of_another_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(
        &tx,
        &w.other_tenant,
        &w.other_tenant_event,
        &w.id(10),
        "ELECTORAL_RESULTS",
    )
    .await;

    let error = reports::update_report_last_document_time(&tx, &w.tenant, &w.id(10))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "No report found with the given tenant_id and id"
    );
    assert_eq!(
        stored(&tx, "report", &w.id(10), &["created_at"]).await["cron_config"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_template_alias_for_report_returns_the_alias_of_the_election_report() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    set(
        &tx,
        "report",
        &w.id(10),
        &format!(
            "election_id = '{}', template_alias = 'election-alias'",
            w.id(20)
        ),
    )
    .await;

    let alias = reports::get_template_alias_for_report(
        &tx,
        &w.tenant,
        &w.event,
        &ReportType::ELECTORAL_RESULTS,
        Some(&w.id(20)),
    )
    .await
    .unwrap();

    assert_eq!(alias.as_deref(), Some("election-alias"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_template_alias_for_report_uses_only_event_templates_as_fallback() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    set(
        &tx,
        "report",
        &w.id(10),
        &format!(
            "election_id = '{}', template_alias = 'other-alias'",
            w.id(21)
        ),
    )
    .await;

    for election_id in [Some(w.id(20)), None] {
        let alias = reports::get_template_alias_for_report(
            &tx,
            &w.tenant,
            &w.event,
            &ReportType::ELECTORAL_RESULTS,
            election_id.as_deref(),
        )
        .await
        .unwrap();
        assert_eq!(alias, None);
    }
    report_row(&tx, &w.tenant, &w.event, &w.id(11), "ELECTORAL_RESULTS").await;
    set(&tx, "report", &w.id(11), "template_alias = 'event-alias'").await;
    for election_id in [Some(w.id(20)), None] {
        let alias = reports::get_template_alias_for_report(
            &tx,
            &w.tenant,
            &w.event,
            &ReportType::ELECTORAL_RESULTS,
            election_id.as_deref(),
        )
        .await
        .unwrap();
        assert_eq!(alias.as_deref(), Some("event-alias"));
    }
    let exact = reports::get_template_alias_for_report(
        &tx,
        &w.tenant,
        &w.event,
        &ReportType::ELECTORAL_RESULTS,
        Some(&w.id(21)),
    )
    .await
    .unwrap();
    assert_eq!(exact.as_deref(), Some("other-alias"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_template_alias_for_report_is_none_without_a_report_of_the_type_in_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "BALLOT_RECEIPT").await;
    report_row(
        &tx,
        &w.tenant,
        &w.other_event,
        &w.id(11),
        "ELECTORAL_RESULTS",
    )
    .await;
    report_row(
        &tx,
        &w.other_tenant,
        &w.event,
        &w.id(12),
        "ELECTORAL_RESULTS",
    )
    .await;
    for n in [10, 11, 12] {
        set(&tx, "report", &w.id(n), "template_alias = 'alias'").await;
    }

    for election_id in [None, Some(w.id(20))] {
        let alias = reports::get_template_alias_for_report(
            &tx,
            &w.tenant,
            &w.event,
            &ReportType::ELECTORAL_RESULTS,
            election_id.as_deref(),
        )
        .await
        .unwrap();
        assert_eq!(alias, None);
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_reports_by_election_event_id_returns_the_event_reports_of_the_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    report_row(&tx, &w.tenant, &w.event, &w.id(11), "BALLOT_RECEIPT").await;
    report_row(
        &tx,
        &w.tenant,
        &w.other_event,
        &w.id(12),
        "ELECTORAL_RESULTS",
    )
    .await;
    report_row(
        &tx,
        &w.other_tenant,
        &w.event,
        &w.id(13),
        "ELECTORAL_RESULTS",
    )
    .await;

    let found = reports::get_reports_by_election_event_id(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(report_ids(found), [w.id(10), w.id(11)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_reports_by_election_id_returns_the_election_reports_of_the_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for (n, tenant, election) in [
        (10, &w.tenant, w.id(20)),
        (11, &w.tenant, w.id(21)),
        (12, &w.other_tenant, w.id(20)),
    ] {
        report_row(&tx, tenant, &w.event, &w.id(n), "ELECTORAL_RESULTS").await;
        set(
            &tx,
            "report",
            &w.id(n),
            &format!("election_id = '{election}'"),
        )
        .await;
    }
    report_row(&tx, &w.tenant, &w.event, &w.id(13), "ELECTORAL_RESULTS").await;

    let found = reports::get_reports_by_election_id(&tx, &w.tenant, &w.id(20))
        .await
        .unwrap();

    assert_eq!(report_ids(found), [w.id(10)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_reports_by_election_id_rejects_an_invalid_election_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = reports::get_reports_by_election_id(&tx, &w.tenant, "election-1")
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Error parsing election_id as UUID");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_report_by_type_only_returns_a_report_of_the_requested_election() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    set(
        &tx,
        "report",
        &w.id(10),
        &format!("election_id = '{}'", w.id(21)),
    )
    .await;
    report_row(&tx, &w.tenant, &w.event, &w.id(11), "BALLOT_RECEIPT").await;
    set(
        &tx,
        "report",
        &w.id(11),
        &format!("election_id = '{}'", w.id(20)),
    )
    .await;

    let found = reports::get_report_by_type(
        &tx,
        &w.tenant,
        &w.event,
        "ELECTORAL_RESULTS",
        &Some(w.id(20)),
    )
    .await
    .unwrap();
    let of_the_other_election = reports::get_report_by_type(
        &tx,
        &w.tenant,
        &w.event,
        "ELECTORAL_RESULTS",
        &Some(w.id(21)),
    )
    .await
    .unwrap();

    assert!(found.is_none());
    assert_eq!(
        of_the_other_election.map(|report| report.id),
        Some(w.id(10))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_report_by_type_drops_the_election_filter_for_an_invalid_election_id() {
    // An election id that does not parse is ignored instead of rejected.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    report_row(&tx, &w.tenant, &w.event, &w.id(10), "ELECTORAL_RESULTS").await;
    set(
        &tx,
        "report",
        &w.id(10),
        &format!("election_id = '{}'", w.id(21)),
    )
    .await;

    let found = reports::get_report_by_type(
        &tx,
        &w.tenant,
        &w.event,
        "ELECTORAL_RESULTS",
        &Some("election-1".into()),
    )
    .await
    .unwrap();

    assert_eq!(found.map(|report| report.id), Some(w.id(10)));
    tx.rollback().await.unwrap();
}

// document

async fn document_row(
    tx: &Transaction<'_>,
    tenant: &str,
    event: Option<&str>,
    id: &str,
    annotations: Option<Value>,
) {
    tx.execute(
        "INSERT INTO sequent_backend.document
             (id, tenant_id, election_event_id, name, media_type, size, annotations, created_at,
              last_updated_at)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, 'results.pdf',
                 'application/pdf', 1024, $4, '2026-01-01T10:00:00Z', '2026-01-02T10:00:00Z')",
        &[&id, &tenant, &event, &annotations],
    )
    .await
    .unwrap();
}

async fn support_material_row(
    tx: &Transaction<'_>,
    [tenant, event, id, document_id]: [&str; 4],
    is_hidden: Option<bool>,
) {
    tx.execute(
        "INSERT INTO sequent_backend.support_material
             (id, tenant_id, election_event_id, kind, data, labels, annotations, document_id,
              is_hidden, created_at, last_updated_at)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, 'PDF',
                 '{\"title\": \"Guide\"}', '{\"label\": 1}', '{\"note\": 1}', $4, $5,
                 '2026-01-01T10:00:00Z', '2026-01-02T10:00:00Z')",
        &[&id, &tenant, &event, &document_id, &is_hidden],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn insert_document_returns_and_stores_the_document() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let inserted = document::insert_document(
        &tx,
        &w.tenant,
        Some(w.event.clone()),
        "results.pdf",
        "application/pdf",
        2048,
        true,
        Some(w.id(10)),
    )
    .await
    .unwrap();

    let started = now(&tx).await.with_timezone(&Local);
    assert_eq!(
        inserted,
        Document {
            id: w.id(10),
            tenant_id: Some(w.tenant.clone()),
            election_event_id: Some(w.event.clone()),
            name: Some("results.pdf".into()),
            media_type: Some("application/pdf".into()),
            size: Some(2048),
            labels: None,
            annotations: None,
            created_at: Some(started),
            last_updated_at: Some(started),
            is_public: Some(true),
        }
    );
    assert_eq!(
        stored(
            &tx,
            "document",
            &w.id(10),
            &["created_at", "last_updated_at"]
        )
        .await,
        json!({
            "id": w.id(10),
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "name": "results.pdf",
            "media_type": "application/pdf",
            "size": 2048,
            "labels": null,
            "annotations": null,
            "is_public": true,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_document_without_an_id_stores_a_new_v4_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let inserted = document::insert_document(
        &tx,
        &w.tenant,
        None,
        "logo.png",
        "image/png",
        10,
        false,
        None,
    )
    .await
    .unwrap();

    assert_eq!(Uuid::parse_str(&inserted.id).unwrap().get_version_num(), 4);
    let row = stored(
        &tx,
        "document",
        &inserted.id,
        &["created_at", "last_updated_at"],
    )
    .await;
    assert_eq!(
        (&row["tenant_id"], &row["election_event_id"]),
        (&json!(w.tenant), &Value::Null)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_document_with_annotations_stores_the_access_annotations() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let inserted = document::insert_document_with_annotations(
        &tx,
        &w.tenant,
        Some(w.event.clone()),
        "voters.csv",
        "text/csv",
        10,
        false,
        Some(w.id(10)),
        Some(&DocumentAnnotations::password_protected("secret-1")),
    )
    .await
    .unwrap();

    let annotations = json!({"access": {"password_secret_id": "secret-1"}});
    assert_eq!(inserted.annotations, Some(annotations.clone()));
    assert_eq!(
        stored(
            &tx,
            "document",
            &w.id(10),
            &["created_at", "last_updated_at"]
        )
        .await["annotations"],
        annotations
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_document_rejects_an_invalid_document_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = document::insert_document(
        &tx,
        &w.tenant,
        None,
        "logo.png",
        "image/png",
        10,
        false,
        Some("document-1".into()),
    )
    .await
    .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'document-1'"));
    assert_eq!(count(&tx, "document", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_document_returns_the_document_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    document_row(
        &tx,
        &w.tenant,
        Some(&w.event),
        &w.id(10),
        Some(json!({"a": 1})),
    )
    .await;

    let found = document::get_document(&tx, &w.tenant, Some(w.event.clone()), &w.id(10))
        .await
        .unwrap();

    assert_eq!(
        found,
        Some(Document {
            id: w.id(10),
            tenant_id: Some(w.tenant.clone()),
            election_event_id: Some(w.event.clone()),
            name: Some("results.pdf".into()),
            media_type: Some("application/pdf".into()),
            size: Some(1024),
            labels: None,
            annotations: Some(json!({"a": 1})),
            created_at: Some(at(H10)),
            last_updated_at: Some(at(NEXT_DAY)),
            is_public: Some(false),
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_document_is_none_outside_the_tenant_or_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    document_row(&tx, &w.tenant, Some(&w.other_event), &w.id(10), None).await;
    document_row(
        &tx,
        &w.other_tenant,
        Some(&w.other_tenant_event),
        &w.id(11),
        None,
    )
    .await;

    let in_other_event = document::get_document(&tx, &w.tenant, Some(w.event.clone()), &w.id(10))
        .await
        .unwrap();
    let of_other_tenant = document::get_document(&tx, &w.tenant, None, &w.id(11))
        .await
        .unwrap();

    assert_eq!((in_other_event, of_other_tenant), (None, None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_document_without_an_event_matches_any_event_of_the_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    document_row(&tx, &w.tenant, Some(&w.other_event), &w.id(10), None).await;

    for event in [None, Some(String::new())] {
        let found = document::get_document(&tx, &w.tenant, event, &w.id(10))
            .await
            .unwrap();
        assert_eq!(found.map(|document| document.id), Some(w.id(10)));
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_documents_deletes_only_the_listed_documents_of_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(10), None).await;
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(11), None).await;
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(12), None).await;
    document_row(&tx, &w.tenant, Some(&w.other_event), &w.id(13), None).await;
    document_row(&tx, &w.other_tenant, Some(&w.event), &w.id(14), None).await;

    let deleted = document::delete_documents(
        &tx,
        &w.tenant,
        &w.event,
        &[w.id(10), w.id(11), w.id(13), w.id(14)],
    )
    .await
    .unwrap();

    assert_eq!(deleted, 2);
    let remaining: Vec<String> = tx
        .query(
            "SELECT id::text FROM sequent_backend.document
             WHERE tenant_id IN ($1::text::uuid, $2::text::uuid) ORDER BY id",
            &[&w.tenant, &w.other_tenant],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(remaining, [w.id(12), w.id(13), w.id(14)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_documents_with_no_ids_deletes_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(10), None).await;

    let deleted = document::delete_documents(&tx, &w.tenant, &w.event, &[])
        .await
        .unwrap();

    assert_eq!((deleted, count(&tx, "document", &w.tenant).await), (0, 1));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_documents_rejects_an_invalid_id_before_deleting() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(10), None).await;

    let error =
        document::delete_documents(&tx, &w.tenant, &w.event, &[w.id(10), "document-2".into()])
            .await
            .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'document-2'"));
    assert_eq!(count(&tx, "document", &w.tenant).await, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_exportable_document_ids_leaves_out_voter_secret_documents() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let voter_secrets = serde_json::to_value(DocumentAnnotations::voter_secret_export()).unwrap();
    let password =
        serde_json::to_value(DocumentAnnotations::password_protected("secret-1")).unwrap();
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(10), None).await;
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(11), Some(password)).await;
    document_row(
        &tx,
        &w.tenant,
        Some(&w.event),
        &w.id(12),
        Some(json!({"other": true})),
    )
    .await;
    document_row(
        &tx,
        &w.tenant,
        Some(&w.event),
        &w.id(13),
        Some(voter_secrets),
    )
    .await;
    document_row(&tx, &w.tenant, Some(&w.other_event), &w.id(14), None).await;

    let exportable = document::get_exportable_document_ids(&tx, &w.tenant, &w.event, false)
        .await
        .unwrap();

    assert_eq!(exportable, HashSet::from([w.id(10), w.id(11), w.id(12)]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_exportable_document_ids_includes_voter_secret_documents_when_allowed() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let voter_secrets = serde_json::to_value(DocumentAnnotations::voter_secret_export()).unwrap();
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(10), None).await;
    document_row(
        &tx,
        &w.tenant,
        Some(&w.event),
        &w.id(11),
        Some(voter_secrets),
    )
    .await;

    let exportable = document::get_exportable_document_ids(&tx, &w.tenant, &w.event, true)
        .await
        .unwrap();

    assert_eq!(exportable, HashSet::from([w.id(10), w.id(11)]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_exportable_document_ids_fails_on_access_annotations_it_cannot_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    document_row(&tx, &w.tenant, Some(&w.event), &w.id(10), None).await;
    document_row(
        &tx,
        &w.tenant,
        Some(&w.event),
        &w.id(11),
        Some(json!({"access": "public"})),
    )
    .await;

    let error = document::get_exportable_document_ids(&tx, &w.tenant, &w.event, true)
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid type: string \"public\", expected struct DocumentAccess"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_support_material_documents_pairs_visible_materials_with_their_documents() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (tenant, event) = (w.tenant.as_str(), w.event.as_str());
    for n in [10, 11, 12] {
        document_row(&tx, tenant, Some(event), &w.id(n), None).await;
    }
    document_row(&tx, tenant, Some(&w.other_event), &w.id(13), None).await;
    support_material_row(&tx, [tenant, event, &w.id(20), &w.id(10)], Some(false)).await;
    support_material_row(&tx, [tenant, event, &w.id(21), &w.id(11)], Some(true)).await;
    // A material whose visibility was never set is left out too.
    support_material_row(&tx, [tenant, event, &w.id(22), &w.id(12)], None).await;
    support_material_row(&tx, [tenant, event, &w.id(23), &w.id(13)], Some(false)).await;
    support_material_row(
        &tx,
        [tenant, &w.other_event, &w.id(24), &w.id(13)],
        Some(false),
    )
    .await;

    let pairs = document::get_support_material_documents(&tx, tenant, event)
        .await
        .unwrap();

    assert_eq!(
        pairs,
        Some(vec![(
            SupportMaterial {
                id: w.id(20),
                created_at: at(H10),
                last_updated_at: at(NEXT_DAY),
                kind: "PDF".into(),
                data: json!({"title": "Guide"}),
                tenant_id: w.tenant.clone(),
                election_event_id: w.event.clone(),
                labels: json!({"label": 1}),
                annotations: json!({"note": 1}),
                document_id: Some(w.id(10)),
                is_hidden: Some(false),
            },
            Document {
                id: w.id(10),
                tenant_id: Some(w.tenant.clone()),
                election_event_id: Some(w.event.clone()),
                name: Some("results.pdf".into()),
                media_type: Some("application/pdf".into()),
                size: Some(1024),
                labels: None,
                annotations: None,
                created_at: Some(at(H10)),
                last_updated_at: Some(at(NEXT_DAY)),
                is_public: Some(false),
            }
        )])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_support_material_documents_is_an_empty_list_without_materials() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let pairs = document::get_support_material_documents(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(pairs, Some(vec![]));
    tx.rollback().await.unwrap();
}

// lock

/// Expiry times far enough from today that they stay past or future.
const EXPIRED: &str = "2020-01-01T00:00:00Z";
const LATER: &str = "2100-01-01T00:00:00Z";
const LATER_STILL: &str = "2101-01-01T00:00:00Z";

async fn lock_row(tx: &Transaction<'_>, key: &str, value: &str, expiry_date: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.lock (key, value, expiry_date)
         VALUES ($1, $2, $3::text::timestamptz)",
        &[&key, &value, &expiry_date],
    )
    .await
    .unwrap();
}

/// The value and expiry of the lock on `key`, if any.
async fn held(tx: &Transaction<'_>, key: &str) -> Option<(String, DateTime<Local>)> {
    tx.query_opt(
        "SELECT value, expiry_date FROM sequent_backend.lock WHERE key = $1",
        &[&key],
    )
    .await
    .unwrap()
    .map(|row| (row.get(0), row.get(1)))
}

#[tokio::test]
async fn upsert_lock_acquires_a_free_key() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let key = format!("{}-lock", ids!().id(10));

    let acquired = lock::upsert_lock(&tx, &key, "worker-1", at(LATER))
        .await
        .unwrap();

    assert_eq!(
        (
            acquired.key.as_str(),
            acquired.value.as_str(),
            acquired.expiry_date
        ),
        (key.as_str(), "worker-1", Some(at(LATER)))
    );
    assert_eq!(held(&tx, &key).await, Some(("worker-1".into(), at(LATER))));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn upsert_lock_refuses_a_key_another_holder_keeps_alive() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let key = format!("{}-lock", ids!().id(10));
    lock_row(&tx, &key, "worker-1", LATER).await;

    let error = lock::upsert_lock(&tx, &key, "worker-2", at(LATER_STILL))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Couldn't upsert lock");
    assert_eq!(held(&tx, &key).await, Some(("worker-1".into(), at(LATER))));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn upsert_lock_takes_over_an_expired_lock() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let key = format!("{}-lock", ids!().id(10));
    lock_row(&tx, &key, "worker-1", EXPIRED).await;

    let acquired = lock::upsert_lock(&tx, &key, "worker-2", at(LATER))
        .await
        .unwrap();

    assert_eq!(acquired.value, "worker-2");
    assert_eq!(held(&tx, &key).await, Some(("worker-2".into(), at(LATER))));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn upsert_lock_renews_a_lock_for_its_holder() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let key = format!("{}-lock", ids!().id(10));
    lock_row(&tx, &key, "worker-1", LATER).await;

    let renewed = lock::upsert_lock(&tx, &key, "worker-1", at(LATER_STILL))
        .await
        .unwrap();

    assert_eq!(renewed.expiry_date, Some(at(LATER_STILL)));
    assert_eq!(
        held(&tx, &key).await,
        Some(("worker-1".into(), at(LATER_STILL)))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_lock_only_releases_a_lock_for_its_holder() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let key = format!("{}-lock", ids!().id(10));
    lock_row(&tx, &key, "worker-1", LATER).await;

    lock::delete_lock(&tx, &key, "worker-2").await.unwrap();
    assert_eq!(held(&tx, &key).await, Some(("worker-1".into(), at(LATER))));

    lock::delete_lock(&tx, &key, "worker-1").await.unwrap();
    assert_eq!(held(&tx, &key).await, None);
    tx.rollback().await.unwrap();
}

// render_report

fn template(template: &str, name: &str) -> RenderTemplateBody {
    RenderTemplateBody {
        template: template.into(),
        name: name.into(),
        variables: Map::new(),
        format: FormatType::TEXT,
    }
}

async fn documents_named(tx: &Transaction<'_>, name: &str) -> i64 {
    tx.query_one(
        "SELECT count(*) FROM sequent_backend.document WHERE name = $1",
        &[&name],
    )
    .await
    .unwrap()
    .get(0)
}

#[tokio::test]
async fn render_report_task_fails_for_an_unknown_tenant_before_writing_a_document() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let ids = ids!();
    let name = format!("{}-report.html", ids.id(10));

    let error = render_report::render_report_task(
        &tx,
        template("Hello {{username}}", &name),
        ids.id(1),
        ids.id(2),
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "Error obtaining Tenant");
    assert_eq!(documents_named(&tx, &name).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn render_report_task_fails_on_an_invalid_template_before_writing_a_document() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let name = format!("{}-report.html", w.id(10));

    let error = render_report::render_report_task(
        &tx,
        template("Hello {{#if}}", &name),
        w.tenant.clone(),
        w.event.clone(),
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .starts_with("Failed to parse template Template error: invalid handlebars syntax"));
    assert_eq!(documents_named(&tx, &name).await, 0);
    tx.rollback().await.unwrap();
}
