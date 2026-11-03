// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Real PostgreSQL contracts of the election event Trash. Each test creates a
//! private database next to the one configured through HASURA_DB__*, applies
//! the Trash migration to the columns it builds on, and drops it at the end.

use super::*;
use crate::postgres::election_event_trash::{
    lock_claimed_election_event, lock_election_event_for_schedule, TrashedElectionEvent,
};
use crate::postgres::scheduled_event::find_scheduled_event_by_id;
use crate::services::database::PgConfig;
use deadpool_postgres::{Pool, Runtime};
use std::time::Duration;
use tokio_postgres::error::SqlState;
use tokio_postgres::NoTls;
use uuid::Uuid;

const BASE_SCHEMA: &str = r#"
    CREATE SCHEMA sequent_backend;
    CREATE TABLE sequent_backend.election_event (
        id uuid PRIMARY KEY,
        tenant_id uuid NOT NULL,
        is_archived boolean NOT NULL DEFAULT false,
        status jsonb,
        presentation jsonb
    );
    CREATE TABLE sequent_backend.election (
        id uuid PRIMARY KEY,
        tenant_id uuid NOT NULL,
        election_event_id uuid NOT NULL REFERENCES sequent_backend.election_event (id),
        status jsonb
    );
    CREATE TABLE sequent_backend.scheduled_event (
        id uuid PRIMARY KEY,
        tenant_id uuid,
        election_event_id uuid,
        created_at timestamptz DEFAULT now(),
        stopped_at timestamptz,
        archived_at timestamptz,
        labels jsonb,
        annotations jsonb,
        event_processor varchar,
        cron_config jsonb,
        event_payload jsonb,
        task_id varchar
    );
"#;

const TRASH_MIGRATION: &str = include_str!(
    "../../../../hasura/migrations/backend-db/1793700000000_election_event_trash/up.sql"
);
const TRASH_ROLLBACK: &str = include_str!(
    "../../../../hasura/migrations/backend-db/1793700000000_election_event_trash/down.sql"
);

struct TestDatabase {
    admin: Pool,
    name: String,
    pool: Pool,
}

impl TestDatabase {
    async fn create() -> Self {
        let config = PgConfig::from_env().unwrap().hasura_db;
        let admin = config.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();
        let name = format!("trash_{}", Uuid::new_v4().simple());
        admin
            .get()
            .await
            .unwrap()
            .batch_execute(&format!("CREATE DATABASE {name}"))
            .await
            .unwrap();
        let mut private = config.clone();
        private.dbname = Some(name.clone());
        let pool = private.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();
        let client = pool.get().await.unwrap();
        client.batch_execute(BASE_SCHEMA).await.unwrap();
        client.batch_execute(TRASH_MIGRATION).await.unwrap();
        Self { admin, name, pool }
    }

    async fn drop(self) {
        self.pool.close();
        let _ = self
            .admin
            .get()
            .await
            .unwrap()
            .batch_execute(&format!("DROP DATABASE {} WITH (FORCE)", self.name))
            .await;
    }

    async fn execute(
        &self,
        statement: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) {
        self.pool
            .get()
            .await
            .unwrap()
            .execute(statement, params)
            .await
            .unwrap();
    }

    async fn event(&self, tenant: Uuid, archived: bool, status: Value) -> Uuid {
        let id = Uuid::new_v4();
        self.execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, is_archived, status, presentation)
             VALUES ($1, $2, $3, $4, '{\"i18n\": {\"en\": {\"name\": \"Synthetic\"}}}')",
            &[&id, &tenant, &archived, &status],
        )
        .await;
        id
    }

    async fn election(&self, tenant: Uuid, event: Uuid, status: Value) -> Uuid {
        let id = Uuid::new_v4();
        self.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, status)
             VALUES ($1, $2, $3, $4)",
            &[&id, &tenant, &event, &status],
        )
        .await;
        id
    }

    async fn trash(&self, tenant: Uuid, event: Uuid) -> Result<TrashOutcome> {
        let mut client = self.pool.get().await?;
        let transaction = client.transaction().await?;
        let outcome =
            move_election_event_to_trash(&transaction, &tenant.to_string(), &event.to_string(), 7)
                .await?;
        transaction.commit().await?;
        Ok(outcome)
    }

    async fn restore(&self, tenant: Uuid, event: Uuid) -> Result<bool> {
        let mut client = self.pool.get().await?;
        let transaction = client.transaction().await?;
        let archived =
            restore_election_event(&transaction, &tenant.to_string(), &event.to_string()).await?;
        transaction.commit().await?;
        Ok(archived)
    }

    async fn trash_list(&self, tenant: Uuid) -> Vec<TrashedElectionEvent> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = client.transaction().await.unwrap();
        get_election_event_trash(&transaction, &tenant.to_string(), None, None)
            .await
            .unwrap()
            .items
    }

    async fn claim(&self) -> Vec<(String, String)> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = client.transaction().await.unwrap();
        let claimed = claim_expired_election_events(
            &transaction,
            PURGE_BATCH_SIZE,
            PURGE_RETRY_AFTER_SECONDS,
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        claimed
    }

    async fn expire(&self, event: Uuid) {
        self.execute(
            "UPDATE sequent_backend.election_event SET purge_after = now() - interval '1 millisecond' WHERE id = $1",
            &[&event],
        )
        .await;
    }
}

fn trash_error(result: Result<impl std::fmt::Debug>) -> TrashError {
    *result.unwrap_err().downcast_ref::<TrashError>().unwrap()
}

fn closed() -> Value {
    serde_json::json!({"voting_status": "CLOSED"})
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn trash_and_restore_keep_data_and_the_prior_archive_state() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    for archived in [false, true] {
        let event = db.event(tenant, archived, closed()).await;
        let TrashOutcome::Trashed {
            trashed_at,
            purge_after,
        } = db.trash(tenant, event).await.unwrap()
        else {
            panic!("not moved to Trash");
        };
        assert_eq!((purge_after - trashed_at).num_days(), 7);
        assert_eq!(
            db.trash(tenant, event).await.unwrap(),
            TrashOutcome::AlreadyInTrash
        );
        let listed = db.trash_list(tenant).await;
        let row = listed
            .iter()
            .find(|row| row.id == event.to_string())
            .unwrap();
        assert_eq!(
            (row.trashed_at, row.purge_after, row.archived_before_trash),
            (trashed_at, purge_after, archived)
        );

        assert_eq!(db.restore(tenant, event).await.unwrap(), archived);
        assert_eq!(
            trash_error(db.restore(tenant, event).await),
            TrashError::NotInTrash
        );
        assert!(db
            .trash_list(tenant)
            .await
            .iter()
            .all(|row| row.id != event.to_string()));
    }
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn other_tenants_cannot_trash_restore_or_list() {
    let db = TestDatabase::create().await;
    let (tenant, other) = (Uuid::new_v4(), Uuid::new_v4());
    let event = db.event(tenant, false, closed()).await;
    assert_eq!(
        trash_error(db.trash(other, event).await),
        TrashError::NotFound
    );
    db.trash(tenant, event).await.unwrap();
    assert_eq!(
        trash_error(db.restore(other, event).await),
        TrashError::NotFound
    );
    assert!(db.trash_list(other).await.is_empty());
    assert_eq!(db.trash_list(tenant).await.len(), 1);
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn open_or_paused_voting_is_not_moved_to_trash() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let open_event = db
        .event(tenant, false, serde_json::json!({"voting_status": "OPEN"}))
        .await;
    assert_eq!(
        trash_error(db.trash(tenant, open_event).await),
        TrashError::VotingInProgress
    );
    let paused_election = db.event(tenant, false, closed()).await;
    db.election(
        tenant,
        paused_election,
        serde_json::json!({"kiosk_voting_status": "PAUSED"}),
    )
    .await;
    assert_eq!(
        trash_error(db.trash(tenant, paused_election).await),
        TrashError::VotingInProgress
    );
    assert!(db.trash_list(tenant).await.is_empty());
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn restore_stops_only_the_schedules_that_came_due_in_trash() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let event = db.event(tenant, false, closed()).await;
    let schedule = |date: Option<&str>| match date {
        Some(date) => serde_json::json!({"scheduled_date": date}),
        None => serde_json::json!({"cron": "0 0 * * *"}),
    };
    let mut ids = vec![];
    for date in [
        Some("2020-01-01T00:00:00Z"),
        Some("2999-01-01T00:00:00Z"),
        None,
    ] {
        let id = Uuid::new_v4();
        db.execute(
            "INSERT INTO sequent_backend.scheduled_event (id, tenant_id, election_event_id, event_processor, cron_config)
             VALUES ($1, $2, $3, 'START_VOTING_PERIOD', $4)",
            &[&id, &tenant, &event, &schedule(date)],
        )
        .await;
        ids.push(id);
    }
    db.trash(tenant, event).await.unwrap();
    db.restore(tenant, event).await.unwrap();
    let client = db.pool.get().await.unwrap();
    for (id, stopped) in ids.iter().zip([true, false, false]) {
        let row = client
            .query_one(
                "SELECT stopped_at IS NOT NULL FROM sequent_backend.scheduled_event WHERE id = $1",
                &[id],
            )
            .await
            .unwrap();
        assert_eq!(row.get::<_, bool>(0), stopped);
    }
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn expired_events_are_claimed_once_and_can_no_longer_be_restored() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let (due, not_due) = (
        db.event(tenant, false, closed()).await,
        db.event(tenant, false, closed()).await,
    );
    db.trash(tenant, due).await.unwrap();
    db.trash(tenant, not_due).await.unwrap();
    assert!(db.claim().await.is_empty());
    db.expire(due).await;
    assert_eq!(
        db.claim().await,
        vec![(tenant.to_string(), due.to_string())]
    );
    assert!(db.claim().await.is_empty());
    assert_eq!(
        trash_error(db.restore(tenant, due).await),
        TrashError::PurgeStarted
    );
    let listed = db.trash_list(tenant).await;
    assert!(listed
        .iter()
        .find(|row| row.id == due.to_string())
        .unwrap()
        .purge_started_at
        .is_some());
    assert!(db.restore(tenant, not_due).await.is_ok());
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn a_failed_purge_keeps_its_claim_and_is_retried_after_the_retry_delay() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let event = db.event(tenant, false, closed()).await;
    db.trash(tenant, event).await.unwrap();
    db.expire(event).await;
    db.claim().await;

    let mut client = db.pool.get().await.unwrap();
    let purge = client.transaction().await.unwrap();
    assert!(
        lock_claimed_election_event(&purge, &tenant.to_string(), &event.to_string())
            .await
            .unwrap()
    );
    purge
        .execute(
            "DELETE FROM sequent_backend.election_event WHERE id = $1",
            &[&event],
        )
        .await
        .unwrap();
    purge.rollback().await.unwrap();

    assert_eq!(
        trash_error(db.restore(tenant, event).await),
        TrashError::PurgeStarted
    );
    assert!(db.claim().await.is_empty());
    db.execute(
        "UPDATE sequent_backend.election_event SET purge_started_at = now() - interval '2 hours' WHERE id = $1",
        &[&event],
    )
    .await;
    assert_eq!(
        db.claim().await,
        vec![(tenant.to_string(), event.to_string())]
    );
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn restore_and_claim_racing_never_both_win() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let event = db.event(tenant, false, closed()).await;
    db.trash(tenant, event).await.unwrap();
    db.expire(event).await;

    let mut claim_client = db.pool.get().await.unwrap();
    let claim = claim_client.transaction().await.unwrap();
    assert_eq!(
        claim_expired_election_events(&claim, PURGE_BATCH_SIZE, PURGE_RETRY_AFTER_SECONDS)
            .await
            .unwrap()
            .len(),
        1
    );
    let restore = tokio::spawn({
        let pool = db.pool.clone();
        async move {
            let mut client = pool.get().await?;
            let transaction = client.transaction().await?;
            restore_election_event(&transaction, &tenant.to_string(), &event.to_string()).await
        }
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!restore.is_finished(), "restore must wait for the claim");
    claim.commit().await.unwrap();
    assert_eq!(
        trash_error(restore.await.unwrap()),
        TrashError::PurgeStarted
    );
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn a_status_write_waiting_on_an_uncommitted_trash_is_refused() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let event = db.event(tenant, false, closed()).await;
    let election = db.election(tenant, event, closed()).await;

    let mut trash_client = db.pool.get().await.unwrap();
    let trash = trash_client.transaction().await.unwrap();
    move_election_event_to_trash(&trash, &tenant.to_string(), &event.to_string(), 7)
        .await
        .unwrap();
    let writer = tokio::spawn({
        let pool = db.pool.clone();
        async move {
            pool.get()
                .await
                .unwrap()
                .execute(
                    "UPDATE sequent_backend.election SET status = '{\"voting_status\": \"OPEN\"}' WHERE id = $1",
                    &[&election],
                )
                .await
        }
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !writer.is_finished(),
        "the status write must wait for the Trash"
    );
    trash.commit().await.unwrap();
    let error = writer.await.unwrap().unwrap_err();
    assert_eq!(
        error.code(),
        Some(&SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE)
    );
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn a_trash_waiting_on_an_uncommitted_status_write_sees_voting_open() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let event = db.event(tenant, false, closed()).await;
    let election = db.election(tenant, event, closed()).await;

    let mut writer_client = db.pool.get().await.unwrap();
    let writer = writer_client.transaction().await.unwrap();
    writer
        .execute(
            "UPDATE sequent_backend.election SET status = '{\"voting_status\": \"OPEN\"}' WHERE id = $1",
            &[&election],
        )
        .await
        .unwrap();
    let trash = tokio::spawn({
        let pool = db.pool.clone();
        async move {
            let mut client = pool.get().await?;
            let transaction = client.transaction().await?;
            let outcome = move_election_event_to_trash(
                &transaction,
                &tenant.to_string(),
                &event.to_string(),
                7,
            )
            .await?;
            transaction.commit().await?;
            Ok::<_, anyhow::Error>(outcome)
        }
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !trash.is_finished(),
        "the Trash must wait for the status write"
    );
    writer.commit().await.unwrap();
    assert_eq!(
        trash_error(trash.await.unwrap()),
        TrashError::VotingInProgress
    );
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn database_invariants_block_bypassing_the_trash() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let event = db.event(tenant, false, closed()).await;
    let election = db.election(tenant, event, closed()).await;
    db.trash(tenant, event).await.unwrap();
    let client = db.pool.get().await.unwrap();
    for (statement, code) in [
        ("UPDATE sequent_backend.election_event SET is_archived = false WHERE id = $1", SqlState::CHECK_VIOLATION),
        ("UPDATE sequent_backend.election_event SET purge_after = NULL WHERE id = $1", SqlState::CHECK_VIOLATION),
        (
            "UPDATE sequent_backend.election_event SET status = '{\"voting_status\": \"OPEN\"}' WHERE id = $1",
            SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE,
        ),
    ] {
        let error = client.execute(statement, &[&event]).await.unwrap_err();
        assert_eq!(error.code(), Some(&code), "{statement}");
    }
    let error = client
        .execute(
            "UPDATE sequent_backend.election SET status = '{\"voting_status\": \"OPEN\"}' WHERE id = $1",
            &[&election],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(&SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE)
    );
    let error = client
        .execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id) VALUES ($1, $2, $3)",
            &[&Uuid::new_v4(), &tenant, &event],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(&SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE)
    );
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn a_queued_voting_schedule_and_a_restore_never_reopen_voting() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let event = db.event(tenant, false, closed()).await;
    let schedule = Uuid::new_v4();
    db.execute(
        "INSERT INTO sequent_backend.scheduled_event (id, tenant_id, election_event_id, event_processor, cron_config)
         VALUES ($1, $2, $3, 'START_VOTING_PERIOD', '{\"scheduled_date\": \"2020-01-01T00:00:00Z\"}')",
        &[&schedule, &tenant, &event],
    )
    .await;
    db.trash(tenant, event).await.unwrap();
    let (tenant_id, event_id, schedule_id) =
        (tenant.to_string(), event.to_string(), schedule.to_string());

    // The worker reads the schedule first: the restore waits, and the worker
    // still sees the event archived, so its voting change is skipped.
    let mut worker_client = db.pool.get().await.unwrap();
    let worker = worker_client.transaction().await.unwrap();
    lock_election_event_for_schedule(&worker, &tenant_id, &event_id)
        .await
        .unwrap();
    let pending = find_scheduled_event_by_id(
        &worker,
        Some(tenant_id.clone()),
        Some(event_id.clone()),
        &schedule_id,
    )
    .await
    .unwrap();
    assert!(pending.is_some());
    let restore = tokio::spawn({
        let db_pool = db.pool.clone();
        async move {
            let mut client = db_pool.get().await?;
            let transaction = client.transaction().await?;
            restore_election_event(&transaction, &tenant.to_string(), &event.to_string()).await?;
            transaction.commit().await?;
            anyhow::Ok(())
        }
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !restore.is_finished(),
        "the restore must wait for the scheduled change"
    );
    let archived: bool = worker
        .query_one(
            "SELECT is_archived FROM sequent_backend.election_event WHERE id = $1",
            &[&event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(archived);
    worker.rollback().await.unwrap();
    restore.await.unwrap().unwrap();

    // The restore ran first and stopped the schedule that came due in Trash:
    // a worker that waited on it no longer finds the schedule to run.
    let mut client = db.pool.get().await.unwrap();
    let worker = client.transaction().await.unwrap();
    lock_election_event_for_schedule(&worker, &tenant_id, &event_id)
        .await
        .unwrap();
    assert!(find_scheduled_event_by_id(
        &worker,
        Some(tenant_id.clone()),
        Some(event_id.clone()),
        &schedule_id
    )
    .await
    .unwrap()
    .is_none());
    worker.rollback().await.unwrap();
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn every_event_in_trash_is_reachable_page_by_page() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let mut trashed = vec![];
    for _ in 0..5 {
        let event = db.event(tenant, false, closed()).await;
        db.trash(tenant, event).await.unwrap();
        trashed.push(event.to_string());
    }
    let mut client = db.pool.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    let mut seen = vec![];
    for offset in (0..6).step_by(2) {
        let page =
            get_election_event_trash(&transaction, &tenant.to_string(), Some(2), Some(offset))
                .await
                .unwrap();
        assert_eq!(page.total, 5);
        seen.extend(page.items.into_iter().map(|item| item.id));
    }
    seen.sort();
    trashed.sort();
    assert_eq!(seen, trashed);
    let past_the_end =
        get_election_event_trash(&transaction, &tenant.to_string(), Some(2), Some(10))
            .await
            .unwrap();
    assert!(past_the_end.items.is_empty());
    assert_eq!(past_the_end.total, 5);
    let capped =
        get_election_event_trash(&transaction, &tenant.to_string(), Some(10_000), Some(-3))
            .await
            .unwrap();
    assert_eq!(capped.items.len(), 5);
    db.drop().await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
async fn rolling_back_is_refused_while_a_purge_is_claimed() {
    let db = TestDatabase::create().await;
    let tenant = Uuid::new_v4();
    let (claimed, waiting) = (
        db.event(tenant, false, closed()).await,
        db.event(tenant, false, closed()).await,
    );
    db.trash(tenant, claimed).await.unwrap();
    db.trash(tenant, waiting).await.unwrap();
    db.expire(claimed).await;
    db.claim().await;

    let mut client = db.pool.get().await.unwrap();
    let rollback = client.transaction().await.unwrap();
    let error = rollback.batch_execute(TRASH_ROLLBACK).await.unwrap_err();
    assert_eq!(
        error.code(),
        Some(&SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE)
    );
    rollback.rollback().await.unwrap();
    assert_eq!(
        trash_error(db.restore(tenant, claimed).await),
        TrashError::PurgeStarted
    );

    db.execute(
        "DELETE FROM sequent_backend.election_event WHERE id = $1",
        &[&claimed],
    )
    .await;
    client.batch_execute(TRASH_ROLLBACK).await.unwrap();
    let archived: bool = client
        .query_one(
            "SELECT is_archived FROM sequent_backend.election_event WHERE id = $1",
            &[&waiting],
        )
        .await
        .unwrap()
        .get(0);
    assert!(archived);
    db.drop().await;
}
