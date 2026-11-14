// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Election event Trash on a database with every migration: what moving to
//! Trash refuses and audits, restoring the prior archive state, the purge
//! claim, and the interleavings with restores, status writers and scheduled
//! changes. Tests that commit use tenants of their own; the claim, which
//! spans tenants, is exercised by one test only.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Pool;
use serde_json::{json, Value};
use std::time::Duration;
use tokio_postgres::error::SqlState;
use uuid::Uuid;
use windmill::postgres::election_event_trash::claim_expired_election_events;
use windmill::postgres::scheduled_event::{find_scheduled_event_by_id, lock_scheduled_event};
use windmill::postgres::trusted_write;
use windmill::services::audit_log::AuditActor;
use windmill::services::delete_election_event::ElectionEventDeletionRefusal;
use windmill::services::election_event_trash::{
    get_election_event_trash, move_election_event_to_trash, restore_election_event, TrashError,
    TrashOutcome, PURGE_BATCH_SIZE, PURGE_RETRY_AFTER_SECONDS,
};

const CLOSED: &str = r#"{"voting_status": "CLOSED"}"#;
const OPEN: &str = r#"{"voting_status": "OPEN"}"#;

struct Fixture {
    pool: Pool,
    tenant: Uuid,
}

impl Fixture {
    async fn new() -> Self {
        let pool = schema::pool().await;
        let tenant = Uuid::new_v4();
        pool.get()
            .await
            .unwrap()
            .execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
                &[&tenant, &format!("tenant-{tenant}")],
            )
            .await
            .unwrap();
        Fixture { pool, tenant }
    }

    async fn event(&self, archived: bool) -> Uuid {
        let event = Uuid::new_v4();
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        trusted_write(&tx).await.unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election_event
                 (id, tenant_id, encryption_protocol, is_archived, status, presentation)
             VALUES ($1, $2, 'RSA256', $3, $4::text::jsonb, $5)",
            &[
                &event,
                &self.tenant,
                &archived,
                &CLOSED,
                &json!({"i18n": {"en": {"name": "Synthetic event"}}}),
            ],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        event
    }

    async fn election(&self, event: Uuid, label: Option<&str>) -> Uuid {
        let election = Uuid::new_v4();
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        trusted_write(&tx).await.unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, status, permission_label)
             VALUES ($1, $2, $3, $4::text::jsonb, $5)",
            &[&election, &self.tenant, &event, &CLOSED, &label],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        election
    }

    async fn trash(&self, event: Uuid, labels: &[String]) -> anyhow::Result<TrashOutcome> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let outcome = move_election_event_to_trash(
            &tx,
            &self.tenant.to_string(),
            &event.to_string(),
            7,
            labels,
            Some(&actor()),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(outcome)
    }

    async fn restore(&self, event: Uuid, labels: &[String]) -> anyhow::Result<bool> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let archived = restore_election_event(
            &tx,
            &self.tenant.to_string(),
            &event.to_string(),
            labels,
            Some(&actor()),
        )
        .await?;
        tx.commit().await?;
        Ok(archived)
    }

    async fn audit_actions(&self, event: Uuid) -> Vec<String> {
        self.pool
            .get()
            .await
            .unwrap()
            .query(
                "SELECT details FROM sequent_backend.audit_log_outbox
                 WHERE tenant_id = $1 ORDER BY seq",
                &[&self.tenant],
            )
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get::<_, Value>(0))
            .filter(|details| details["object"]["id"] == json!(event.to_string()))
            .map(|details| details["action"].as_str().unwrap().to_string())
            .collect()
    }

    async fn expire(&self, event: Uuid) {
        self.pool
            .get()
            .await
            .unwrap()
            .execute(
                "UPDATE sequent_backend.election_event
                 SET purge_after = now() - interval '1 millisecond' WHERE id = $1",
                &[&event],
            )
            .await
            .unwrap();
    }
}

fn actor() -> AuditActor {
    AuditActor {
        user_id: Some("synthetic-admin".into()),
        username: Some("synthetic-admin".into()),
        role: Some("admin-user".into()),
    }
}

fn trash_error<T: std::fmt::Debug>(result: anyhow::Result<T>) -> TrashError {
    *result.unwrap_err().downcast_ref::<TrashError>().unwrap()
}

async fn begin(pool: &Pool) -> deadpool_postgres::Client {
    pool.get().await.unwrap()
}

async fn blocked<T: Send + 'static>(task: &tokio::task::JoinHandle<T>) -> bool {
    tokio::time::sleep(Duration::from_millis(500)).await;
    !task.is_finished()
}

#[tokio::test]
async fn trash_and_restore_keep_the_archive_state_and_are_audited() {
    let fixture = Fixture::new().await;
    for archived in [false, true] {
        let event = fixture.event(archived).await;
        assert!(matches!(
            fixture.trash(event, &[]).await.unwrap(),
            TrashOutcome::Trashed { .. }
        ));
        assert_eq!(
            fixture.trash(event, &[]).await.unwrap(),
            TrashOutcome::AlreadyInTrash
        );
        assert_eq!(fixture.restore(event, &[]).await.unwrap(), archived);
        assert_eq!(
            trash_error(fixture.restore(event, &[]).await),
            TrashError::NotInTrash
        );
        assert_eq!(fixture.audit_actions(event).await, ["trash", "restore"]);
    }
}

#[tokio::test]
async fn trash_keeps_the_deletion_refusals() {
    let fixture = Fixture::new().await;
    let labelled = fixture.event(false).await;
    fixture.election(labelled, Some("north")).await;
    let refusal = fixture.trash(labelled, &[]).await.unwrap_err();
    assert_eq!(
        refusal.downcast_ref::<ElectionEventDeletionRefusal>(),
        Some(&ElectionEventDeletionRefusal::PermissionLabels)
    );
    assert!(fixture.trash(labelled, &["north".into()]).await.is_ok());
    let restore = fixture.restore(labelled, &[]).await.unwrap_err();
    assert_eq!(
        restore.downcast_ref::<ElectionEventDeletionRefusal>(),
        Some(&ElectionEventDeletionRefusal::PermissionLabels)
    );
    assert!(!fixture.restore(labelled, &["north".into()]).await.unwrap());

    let voting = fixture.event(false).await;
    let mut client = begin(&fixture.pool).await;
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET status = $2::text::jsonb WHERE id = $1",
        &[&voting, &OPEN],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let refusal = fixture.trash(voting, &[]).await.unwrap_err();
    assert_eq!(
        refusal.downcast_ref::<ElectionEventDeletionRefusal>(),
        Some(&ElectionEventDeletionRefusal::VotingInProgress)
    );
}

#[tokio::test]
async fn other_tenants_cannot_trash_restore_or_list() {
    let fixture = Fixture::new().await;
    let other = Fixture::new().await;
    let event = fixture.event(false).await;
    assert_eq!(
        trash_error(other.trash(event, &[]).await),
        TrashError::NotFound
    );
    fixture.trash(event, &[]).await.unwrap();
    assert_eq!(
        trash_error(other.restore(event, &[]).await),
        TrashError::NotFound
    );
    let mut client = begin(&fixture.pool).await;
    let tx = client.transaction().await.unwrap();
    let listed = get_election_event_trash(&tx, &other.tenant.to_string(), None, None)
        .await
        .unwrap();
    assert_eq!((listed.items.len(), listed.total), (0, 0));
    let listed = get_election_event_trash(&tx, &fixture.tenant.to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(listed.items[0].id, event.to_string());
}

#[tokio::test]
async fn a_status_write_waiting_on_an_uncommitted_trash_is_refused() {
    let fixture = Fixture::new().await;
    let event = fixture.event(false).await;
    let election = fixture.election(event, None).await;
    let mut trash_client = begin(&fixture.pool).await;
    let trash = trash_client.transaction().await.unwrap();
    move_election_event_to_trash(
        &trash,
        &fixture.tenant.to_string(),
        &event.to_string(),
        7,
        &[],
        None,
        None,
    )
    .await
    .unwrap();
    let writer = tokio::spawn({
        let pool = fixture.pool.clone();
        async move {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            trusted_write(&tx).await.unwrap();
            tx.execute(
                "UPDATE sequent_backend.election SET status = $2::text::jsonb WHERE id = $1",
                &[&election, &OPEN],
            )
            .await
        }
    });
    assert!(
        blocked(&writer).await,
        "the status write must wait for the Trash"
    );
    trash.commit().await.unwrap();
    let error = writer.await.unwrap().unwrap_err();
    assert_eq!(
        error.code(),
        Some(&SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE)
    );
}

#[tokio::test]
async fn a_queued_scheduled_change_and_a_restore_take_turns() {
    let fixture = Fixture::new().await;
    let event = fixture.event(false).await;
    let schedule = Uuid::new_v4();
    let mut client = begin(&fixture.pool).await;
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.scheduled_event
             (id, tenant_id, election_event_id, event_processor, cron_config)
         VALUES ($1, $2, $3, 'START_VOTING_PERIOD', $4)",
        &[
            &schedule,
            &fixture.tenant,
            &event,
            &json!({"scheduled_date": "2020-01-01T00:00:00Z"}),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    fixture.trash(event, &[]).await.unwrap();
    let (tenant, event_id, schedule_id) = (
        fixture.tenant.to_string(),
        event.to_string(),
        schedule.to_string(),
    );

    // The scheduled change holds the scheduler's locks: the restore waits,
    // and the change sees the event archived, which its wrapper skips.
    let mut worker_client = begin(&fixture.pool).await;
    let worker = worker_client.transaction().await.unwrap();
    lock_scheduled_event(&worker, &tenant, &event_id, &schedule_id)
        .await
        .unwrap();
    let restore = tokio::spawn({
        let pool = fixture.pool.clone();
        let (tenant, event_id) = (tenant.clone(), event_id.clone());
        async move {
            let mut client = pool.get().await?;
            let tx = client.transaction().await?;
            restore_election_event(&tx, &tenant, &event_id, &[], None).await?;
            tx.commit().await?;
            anyhow::Ok(())
        }
    });
    assert!(
        blocked(&restore).await,
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

    // After the restore, the schedule that came due in Trash is stopped: a
    // change that waited on it finds nothing to run.
    let worker = worker_client.transaction().await.unwrap();
    lock_scheduled_event(&worker, &tenant, &event_id, &schedule_id)
        .await
        .unwrap();
    assert!(
        find_scheduled_event_by_id(&worker, Some(tenant), Some(event_id), &schedule_id)
            .await
            .unwrap()
            .is_none()
    );
}

/// The only test that claims: the claim spans every tenant of the database.
#[tokio::test]
async fn expired_events_are_claimed_once_and_restore_and_claim_never_both_win() {
    let fixture = Fixture::new().await;
    let claim = |pool: Pool| async move {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let claimed =
            claim_expired_election_events(&tx, PURGE_BATCH_SIZE, PURGE_RETRY_AFTER_SECONDS)
                .await
                .unwrap();
        tx.commit().await.unwrap();
        claimed
            .into_iter()
            .map(|(_, event)| event)
            .collect::<Vec<_>>()
    };
    let (due, not_due) = (fixture.event(false).await, fixture.event(false).await);
    fixture.trash(due, &[]).await.unwrap();
    fixture.trash(not_due, &[]).await.unwrap();
    fixture.expire(due).await;
    let claimed = claim(fixture.pool.clone()).await;
    assert!(claimed.contains(&due.to_string()));
    assert!(!claimed.contains(&not_due.to_string()));
    assert!(!claim(fixture.pool.clone()).await.contains(&due.to_string()));
    assert_eq!(
        trash_error(fixture.restore(due, &[]).await),
        TrashError::PurgeStarted
    );

    // A claim in progress makes a restore wait, which then finds the purge
    // started.
    let racing = fixture.event(false).await;
    fixture.trash(racing, &[]).await.unwrap();
    fixture.expire(racing).await;
    let mut claim_client = begin(&fixture.pool).await;
    let claim_tx = claim_client.transaction().await.unwrap();
    let claimed =
        claim_expired_election_events(&claim_tx, PURGE_BATCH_SIZE, PURGE_RETRY_AFTER_SECONDS)
            .await
            .unwrap();
    assert!(claimed
        .iter()
        .any(|(_, event)| event == &racing.to_string()));
    let restore = tokio::spawn({
        let pool = fixture.pool.clone();
        let (tenant, event) = (fixture.tenant.to_string(), racing.to_string());
        async move {
            let mut client = pool.get().await?;
            let tx = client.transaction().await?;
            restore_election_event(&tx, &tenant, &event, &[], None).await
        }
    });
    assert!(
        blocked(&restore).await,
        "the restore must wait for the claim"
    );
    claim_tx.commit().await.unwrap();
    assert_eq!(
        trash_error(restore.await.unwrap()),
        TrashError::PurgeStarted
    );
    assert!(fixture.restore(not_due, &[]).await.is_ok());
}

#[tokio::test]
async fn database_invariants_block_bypassing_the_trash() {
    let fixture = Fixture::new().await;
    let event = fixture.event(false).await;
    fixture.trash(event, &[]).await.unwrap();
    let client = begin(&fixture.pool).await;
    for statement in [
        "UPDATE sequent_backend.election_event SET is_archived = false WHERE id = $1",
        "UPDATE sequent_backend.election_event SET purge_after = NULL WHERE id = $1",
    ] {
        let error = client.execute(statement, &[&event]).await.unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::CHECK_VIOLATION),
            "{statement}"
        );
    }
    let error = client
        .execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&Uuid::new_v4(), &fixture.tenant, &event],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(&SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE)
    );
}
