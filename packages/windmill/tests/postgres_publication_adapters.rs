// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot publication, publication file, ballot style and cast vote
//! adapters against the migrated schema. Every test writes its own rows in a
//! transaction, checks them with independent SQL and rolls the transaction back.
//! The publication handoff test removes its committed cross-connection fixtures.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local, TimeZone, Utc};
use deadpool_postgres::{Object, Transaction};
use futures::TryStreamExt;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::types::hasura::core::{BallotPublication, BallotStyle};
use serde_json::{json, Value};
use std::cell::Cell;
use tokio_postgres::error::SqlState;
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;
use windmill::domain::publication_files::{PublishedBallotStyle, FILES_ANNOTATION};
use windmill::postgres::{ballot_publication, ballot_style, cast_vote, publication_files};
use windmill::services::ballot_styles::ballot_publication::prepare_ballot_publication;
use windmill::services::cast_votes::{CastVote, CastVoteStatus};

const BAD_UUID: &str = "not-a-uuid";

async fn connect() -> Object {
    schema::pool().await.get().await.unwrap()
}

/// Noon (UTC) of the given day of January 2026.
fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
}

fn local(instant: DateTime<Utc>) -> DateTime<Local> {
    instant.with_timezone(&Local)
}

fn strings(ids: &[Uuid]) -> Vec<String> {
    ids.iter().map(Uuid::to_string).collect()
}

fn sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

fn assert_invalid_uuid(result: anyhow::Result<impl std::fmt::Debug>) {
    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("invalid UUID 'not-a-uuid'"), "{message}");
}

async fn scalar<T: for<'r> FromSql<'r>>(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> T {
    tx.query_one(sql, params).await.unwrap().get(0)
}

async fn now(tx: &Transaction<'_>) -> DateTime<Utc> {
    scalar(tx, "SELECT now()", &[]).await
}

/// A tenant and one of its election events.
#[derive(Clone, Copy, Debug)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
}

impl Scope {
    fn tenant_id(&self) -> String {
        self.tenant.to_string()
    }

    fn event_id(&self) -> String {
        self.event.to_string()
    }
}

#[derive(Default)]
struct PublicationRow {
    election_ids: Vec<Uuid>,
    election_id: Option<Uuid>,
    published_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    annotations: Option<Value>,
}

fn published(day: u32) -> PublicationRow {
    PublicationRow {
        published_at: Some(at(day)),
        ..Default::default()
    }
}

/// Row builders with fixed identifiers. `seed` keeps the keys of concurrently
/// running tests apart; pass `line!()`, which is unique per test in this file.
struct Fixture<'t, 'c> {
    tx: &'t Transaction<'c>,
    seed: u32,
    next: Cell<u32>,
}

impl<'t, 'c> Fixture<'t, 'c> {
    fn new(tx: &'t Transaction<'c>, seed: u32) -> Self {
        Self {
            tx,
            seed,
            next: Cell::new(0),
        }
    }

    /// The next fixed v4 UUID of this test.
    fn id(&self) -> Uuid {
        let n = self.next.get() + 1;
        self.next.set(n);
        Uuid::parse_str(&format!("{:08x}-0000-4000-8000-{n:012x}", self.seed)).unwrap()
    }

    /// A new tenant with one election event.
    async fn scope(&self) -> Scope {
        let tenant = self.id();
        self.tx
            .execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
                &[&tenant, &format!("tenant-{tenant}")],
            )
            .await
            .unwrap();
        self.event_in(tenant).await
    }

    /// Another election event of `tenant`.
    async fn event_in(&self, tenant: Uuid) -> Scope {
        let event = self.id();
        self.tx
            .execute(
                "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
                 VALUES ($1, $2, 'RSA256')",
                &[&event, &tenant],
            )
            .await
            .unwrap();
        Scope { tenant, event }
    }

    async fn election(&self, scope: Scope) -> Uuid {
        let id = self.id();
        self.election_as(scope, id, None).await;
        id
    }

    async fn election_with_revotes(&self, scope: Scope, revotes: i32) -> Uuid {
        let id = self.id();
        self.election_as(scope, id, Some(revotes)).await;
        id
    }

    async fn election_as(&self, scope: Scope, id: Uuid, revotes: Option<i32>) {
        self.tx
            .execute(
                "INSERT INTO sequent_backend.election
                     (id, tenant_id, election_event_id, num_allowed_revotes)
                 VALUES ($1, $2, $3, $4)",
                &[&id, &scope.tenant, &scope.event, &revotes],
            )
            .await
            .unwrap();
    }

    async fn area(&self, scope: Scope) -> Uuid {
        let id = self.id();
        self.area_as(scope, id).await;
        id
    }

    async fn area_as(&self, scope: Scope, id: Uuid) {
        self.tx
            .execute(
                "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
                 VALUES ($1, $2, $3, $4)",
                &[&id, &scope.tenant, &scope.event, &format!("area-{id}")],
            )
            .await
            .unwrap();
    }

    async fn publication(&self, scope: Scope, row: PublicationRow) -> Uuid {
        let id = self.id();
        self.publication_as(scope, id, row).await;
        id
    }

    async fn publication_as(&self, scope: Scope, id: Uuid, row: PublicationRow) {
        self.tx
            .execute(
                "INSERT INTO sequent_backend.ballot_publication
                     (id, tenant_id, election_event_id, election_ids, election_id,
                      published_at, deleted_at, annotations)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &id,
                    &scope.tenant,
                    &scope.event,
                    &row.election_ids,
                    &row.election_id,
                    &row.published_at,
                    &row.deleted_at,
                    &row.annotations,
                ],
            )
            .await
            .unwrap();
    }

    async fn style(&self, scope: Scope, publication: Uuid, election: Uuid, area: Uuid) -> Uuid {
        let id = self.id();
        self.style_as(scope, id, [publication, election, area], None)
            .await;
        id
    }

    async fn deleted_style(
        &self,
        scope: Scope,
        publication: Uuid,
        election: Uuid,
        area: Uuid,
    ) -> Uuid {
        let id = self.id();
        self.style_as(scope, id, [publication, election, area], Some(at(20)))
            .await;
        id
    }

    /// `[publication, election, area]` of a ballot style.
    async fn style_as(
        &self,
        scope: Scope,
        id: Uuid,
        [publication, election, area]: [Uuid; 3],
        deleted_at: Option<DateTime<Utc>>,
    ) {
        self.tx
            .execute(
                "INSERT INTO sequent_backend.ballot_style
                     (id, tenant_id, election_event_id, election_id, area_id,
                      ballot_publication_id, deleted_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &id,
                    &scope.tenant,
                    &scope.event,
                    &election,
                    &area,
                    &publication,
                    &deleted_at,
                ],
            )
            .await
            .unwrap();
    }

    /// A cast vote written with SQL; it still passes the revote trigger.
    async fn vote(
        &self,
        scope: Scope,
        election: Uuid,
        area: Uuid,
        voter: &str,
        status: &str,
    ) -> Uuid {
        let id = self.id();
        self.vote_as(scope, id, [election, area], voter, status)
            .await;
        id
    }

    async fn vote_as(
        &self,
        scope: Scope,
        id: Uuid,
        [election, area]: [Uuid; 2],
        voter: &str,
        status: &str,
    ) {
        self.tx
            .execute(
                "INSERT INTO sequent_backend.cast_vote
                     (id, tenant_id, election_event_id, election_id, area_id,
                      voter_id_string, status, content)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'ciphertext')",
                &[
                    &id,
                    &scope.tenant,
                    &scope.event,
                    &election,
                    &area,
                    &voter,
                    &status,
                ],
            )
            .await
            .unwrap();
    }
}

/// `(is_generated, published_at, deleted_at)` of a publication.
async fn publication_state(
    tx: &Transaction<'_>,
    scope: Scope,
    id: Uuid,
) -> (bool, Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let row = tx
        .query_one(
            "SELECT is_generated, published_at, deleted_at
             FROM sequent_backend.ballot_publication
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&scope.tenant, &scope.event, &id],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}

async fn style_deleted_at(tx: &Transaction<'_>, scope: Scope, id: Uuid) -> Option<DateTime<Utc>> {
    scalar(
        tx,
        "SELECT deleted_at FROM sequent_backend.ballot_style
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&scope.tenant, &scope.event, &id],
    )
    .await
}

fn publication_ids(publications: &[BallotPublication]) -> Vec<String> {
    sorted(publications.iter().map(|p| p.id.clone()).collect())
}

fn style_ids(styles: &[BallotStyle]) -> Vec<String> {
    sorted(styles.iter().map(|s| s.id.clone()).collect())
}

// ---------------------------------------------------------------------------
// ballot_publication
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_generation_worker_waits_for_publication_commit_without_blocking_the_task_ledger() {
    let mut producer = connect().await;
    let setup = producer.transaction().await.unwrap();
    let f = Fixture::new(&setup, line!());
    let scope = f.scope().await;
    let first = f.election(scope).await;
    let second = f.election(scope).await;
    let other = f.event_in(scope.tenant).await;
    f.election(other).await;
    setup.commit().await.unwrap();
    let mut worker = connect().await;
    let mut ledger = connect().await;

    for selected in [None, Some(first.to_string())] {
        // A publication already committed before dispatch is a valid control.
        let tx = producer.transaction().await.unwrap();
        let committed = prepare_ballot_publication(
            &tx,
            scope.tenant_id(),
            scope.event_id(),
            selected.clone(),
            "publisher".into(),
        )
        .await
        .unwrap();
        assert_eq!(committed.election_id, selected);
        assert_eq!(
            sorted(committed.election_ids.clone().unwrap()),
            sorted(match &selected {
                Some(id) => vec![id.clone()],
                None => vec![first.to_string(), second.to_string()],
            })
        );
        tx.commit().await.unwrap();
        let tx = worker.transaction().await.unwrap();
        ballot_publication::lock_publication_event(&tx, &scope.tenant_id(), &scope.event_id())
            .await
            .unwrap();
        assert!(ballot_publication::get_ballot_publication_by_id(
            &tx,
            &scope.tenant_id(),
            &scope.event_id(),
            &committed.id,
        )
        .await
        .unwrap()
        .is_some());
        tx.rollback().await.unwrap();

        for commit in [true, false] {
            let pending = producer.transaction().await.unwrap();
            let publication = prepare_ballot_publication(
                &pending,
                scope.tenant_id(),
                scope.event_id(),
                selected.clone(),
                "publisher".into(),
            )
            .await
            .unwrap();

            // The task ledger writes on its own connection before enqueueing.
            // Its event foreign key must remain compatible with the barrier.
            let task_tx = ledger.transaction().await.unwrap();
            task_tx
                .batch_execute("SET LOCAL lock_timeout = '100ms'")
                .await
                .unwrap();
            assert_eq!(task_tx.execute(
                "INSERT INTO sequent_backend.tasks_execution
                 (tenant_id, election_event_id, name, type, execution_status, executed_by_user)
                 VALUES ($1, $2, 'Generate ballots', 'GENERATE_BALLOT_PUBLICATION', 'IN_PROGRESS', 'publisher')",
                &[&scope.tenant, &scope.event],
            ).await.unwrap(), 1);
            task_tx.commit().await.unwrap();

            let reading = worker.transaction().await.unwrap();
            reading
                .batch_execute("SET LOCAL lock_timeout = '100ms'")
                .await
                .unwrap();
            let blocked = ballot_publication::lock_publication_event(
                &reading,
                &scope.tenant_id(),
                &scope.event_id(),
            )
            .await;
            match blocked {
                Ok(()) => {
                    let visible = ballot_publication::get_ballot_publication_by_id(
                        &reading,
                        &scope.tenant_id(),
                        &scope.event_id(),
                        &publication.id,
                    )
                    .await
                    .unwrap()
                    .is_some();
                    panic!("worker passed the producer commit barrier; queued publication visible: {visible}");
                }
                Err(error) => {
                    let code = error
                        .chain()
                        .find_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
                        .and_then(tokio_postgres::Error::as_db_error)
                        .map(|error| error.code());
                    assert_eq!(code, Some(&SqlState::LOCK_NOT_AVAILABLE), "{error:#}");
                }
            }
            reading.rollback().await.unwrap();
            if commit {
                pending.commit().await.unwrap();
            } else {
                pending.rollback().await.unwrap();
            }

            let reading = worker.transaction().await.unwrap();
            reading
                .batch_execute("SET LOCAL lock_timeout = '100ms'")
                .await
                .unwrap();
            ballot_publication::lock_publication_event(
                &reading,
                &scope.tenant_id(),
                &scope.event_id(),
            )
            .await
            .unwrap();
            let visible = ballot_publication::get_ballot_publication_by_id(
                &reading,
                &scope.tenant_id(),
                &scope.event_id(),
                &publication.id,
            )
            .await
            .unwrap();
            assert_eq!(visible.is_some(), commit);
            reading.rollback().await.unwrap();
        }
    }

    let cleanup = producer.transaction().await.unwrap();
    for table in [
        "tasks_execution",
        "ballot_publication",
        "election",
        "election_event",
    ] {
        cleanup
            .execute(
                &format!("DELETE FROM sequent_backend.{table} WHERE tenant_id = $1"),
                &[&scope.tenant],
            )
            .await
            .unwrap();
    }
    cleanup
        .execute(
            "DELETE FROM sequent_backend.tenant WHERE id = $1",
            &[&scope.tenant],
        )
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

#[tokio::test]
async fn lock_publication_event_row_locks_only_the_requested_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    f.event_in(a.tenant).await;

    ballot_publication::lock_publication_event(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    // FOR UPDATE stamps the locking transaction into the xmax of each row.
    let locked: Vec<Uuid> = tx
        .query(
            "SELECT id FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND xmax::text = (txid_current() % 4294967296)::text",
            &[&a.tenant],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(locked, vec![a.event]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_publication_event_fails_when_the_tenant_has_no_such_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let b = f.scope().await;

    for (tenant, event) in [(b.tenant, a.event), (a.tenant, f.id())] {
        let error = ballot_publication::lock_publication_event(
            &tx,
            &tenant.to_string(),
            &event.to_string(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Can't find ballot publication election event"
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_publication_event_rejects_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;

    assert_invalid_uuid(
        ballot_publication::lock_publication_event(&tx, BAD_UUID, &a.event_id()).await,
    );
    assert_invalid_uuid(
        ballot_publication::lock_publication_event(&tx, &a.tenant_id(), BAD_UUID).await,
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_ballot_publication_by_id_maps_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let id = f.id();
    tx.execute(
        "INSERT INTO sequent_backend.ballot_publication
             (id, tenant_id, election_event_id, labels, annotations, created_at,
              created_by_user_id, is_generated, election_ids, published_at, election_id)
         VALUES ($1, $2, $3, $4, $5, $6, 'admin-user', true, $7, $8, $9)",
        &[
            &id,
            &a.tenant,
            &a.event,
            &json!({"label": 1}),
            &json!({"note": "text"}),
            &at(1),
            &vec![first, second],
            &at(2),
            &first,
        ],
    )
    .await
    .unwrap();

    let publication = ballot_publication::get_ballot_publication_by_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &id.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(
        publication,
        Some(BallotPublication {
            id: id.to_string(),
            tenant_id: a.tenant_id(),
            election_event_id: a.event_id(),
            labels: Some(json!({"label": 1})),
            annotations: Some(json!({"note": "text"})),
            created_at: Some(local(at(1))),
            deleted_at: None,
            created_by_user_id: Some("admin-user".to_string()),
            is_generated: Some(true),
            election_ids: Some(strings(&[first, second])),
            published_at: Some(local(at(2))),
            election_id: Some(first.to_string()),
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_ballot_publication_by_id_only_finds_publications_of_the_given_tenant_and_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    // The primary key includes tenant and event, so the same id can recur.
    let id = f.publication(a, published(1)).await;
    f.publication_as(other, id, published(2)).await;
    let get = |scope: Scope, id: Uuid| {
        let tx = &tx;
        async move {
            ballot_publication::get_ballot_publication_by_id(
                tx,
                &scope.tenant_id(),
                &scope.event_id(),
                &id.to_string(),
            )
            .await
            .unwrap()
        }
    };

    assert_eq!(get(a, id).await.unwrap().published_at, Some(local(at(1))));
    assert_eq!(
        get(other, id).await.unwrap().published_at,
        Some(local(at(2)))
    );
    assert_eq!(get(sibling, id).await, None);
    assert_eq!(get(a, f.id()).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_ballot_publication_by_id_returns_soft_deleted_publications() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f
        .publication(
            a,
            PublicationRow {
                deleted_at: Some(at(3)),
                ..Default::default()
            },
        )
        .await;

    let publication = ballot_publication::get_ballot_publication_by_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &id.to_string(),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(publication.deleted_at, Some(local(at(3))));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_ballot_publication_by_id_rejects_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.publication(a, published(1)).await.to_string();

    assert_invalid_uuid(
        ballot_publication::get_ballot_publication_by_id(&tx, BAD_UUID, &a.event_id(), &id).await,
    );
    assert_invalid_uuid(
        ballot_publication::get_ballot_publication_by_id(&tx, &a.tenant_id(), BAD_UUID, &id).await,
    );
    assert_invalid_uuid(
        ballot_publication::get_ballot_publication_by_id(
            &tx,
            &a.tenant_id(),
            &a.event_id(),
            BAD_UUID,
        )
        .await,
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_ballot_publication_status_sets_generation_and_publication_time_of_one_publication()
{
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let target = f.publication(a, PublicationRow::default()).await;
    let neighbour = f.publication(a, PublicationRow::default()).await;

    let updated = ballot_publication::update_ballot_publication_status(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &target.to_string(),
        true,
        Some(local(at(4))),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(updated.id, target.to_string());
    assert_eq!(updated.is_generated, Some(true));
    assert_eq!(updated.published_at, Some(local(at(4))));
    assert_eq!(
        publication_state(&tx, a, target).await,
        (true, Some(at(4)), None)
    );
    assert_eq!(
        publication_state(&tx, a, neighbour).await,
        (false, None, None)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_ballot_publication_status_leaves_soft_deleted_publications_unchanged() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let deleted = f
        .publication(
            a,
            PublicationRow {
                deleted_at: Some(at(3)),
                ..Default::default()
            },
        )
        .await;

    let updated = ballot_publication::update_ballot_publication_status(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &deleted.to_string(),
        true,
        Some(local(at(4))),
    )
    .await
    .unwrap();

    assert_eq!(updated, None);
    assert_eq!(
        publication_state(&tx, a, deleted).await,
        (false, None, Some(at(3)))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_ballot_publication_status_leaves_other_tenants_publications_unchanged() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let id = f.publication(a, PublicationRow::default()).await;
    f.publication_as(other, id, PublicationRow::default()).await;

    let mismatched = ballot_publication::update_ballot_publication_status(
        &tx,
        &other.tenant_id(),
        &a.event_id(),
        &id.to_string(),
        true,
        Some(local(at(4))),
    )
    .await
    .unwrap();
    ballot_publication::update_ballot_publication_status(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &id.to_string(),
        true,
        Some(local(at(4))),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(mismatched, None);
    assert_eq!(
        publication_state(&tx, a, id).await,
        (true, Some(at(4)), None)
    );
    assert_eq!(publication_state(&tx, other, id).await, (false, None, None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_ballot_publication_also_updates_soft_deleted_publications() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let deleted = f
        .publication(
            a,
            PublicationRow {
                published_at: Some(at(2)),
                deleted_at: Some(at(3)),
                ..Default::default()
            },
        )
        .await;

    let updated = ballot_publication::update_ballot_publication(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &deleted.to_string(),
        true,
        None,
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(updated.is_generated, Some(true));
    assert_eq!(updated.published_at, None);
    assert_eq!(
        publication_state(&tx, a, deleted).await,
        (true, None, Some(at(3)))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_ballot_publication_only_updates_the_publication_of_the_given_scope() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let id = f.publication(a, PublicationRow::default()).await;
    f.publication_as(sibling, id, PublicationRow::default())
        .await;
    let neighbour = f.publication(a, PublicationRow::default()).await;

    let updated = ballot_publication::update_ballot_publication(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &id.to_string(),
        true,
        Some(local(at(5))),
    )
    .await
    .unwrap();
    let missing = ballot_publication::update_ballot_publication(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &f.id().to_string(),
        true,
        None,
    )
    .await
    .unwrap();

    assert_eq!(updated.unwrap().published_at, Some(local(at(5))));
    assert_eq!(missing, None);
    assert_eq!(
        publication_state(&tx, a, id).await,
        (true, Some(at(5)), None)
    );
    assert_eq!(
        publication_state(&tx, sibling, id).await,
        (false, None, None)
    );
    assert_eq!(
        publication_state(&tx, a, neighbour).await,
        (false, None, None)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_publication_returns_the_most_recently_published_live_publication() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    f.publication(a, published(1)).await;
    let latest = f.publication(a, published(2)).await;
    f.publication(
        a,
        PublicationRow {
            published_at: Some(at(3)),
            deleted_at: Some(at(4)),
            ..Default::default()
        },
    )
    .await;
    f.publication(sibling, published(5)).await;

    let publication =
        ballot_publication::get_latest_ballot_publication(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap();

    assert_eq!(publication.map(|p| p.id), Some(latest.to_string()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_publication_prefers_an_unpublished_draft_over_published_ones() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    f.publication(a, published(2)).await;
    let draft = f.publication(a, PublicationRow::default()).await;

    // `ORDER BY published_at DESC` sorts NULL first in PostgreSQL.
    let publication =
        ballot_publication::get_latest_ballot_publication(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap();

    assert_eq!(publication.map(|p| p.id), Some(draft.to_string()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_publication_returns_none_without_live_publications() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    f.publication(
        a,
        PublicationRow {
            published_at: Some(at(1)),
            deleted_at: Some(at(2)),
            ..Default::default()
        },
    )
    .await;

    let publication =
        ballot_publication::get_latest_ballot_publication(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap();

    assert_eq!(publication, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_ballot_publication_lists_the_live_publications_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let draft = f.publication(a, PublicationRow::default()).await;
    let live = f.publication(a, published(1)).await;
    f.publication(
        a,
        PublicationRow {
            deleted_at: Some(at(2)),
            ..Default::default()
        },
    )
    .await;
    f.publication(sibling, published(1)).await;

    let publications =
        ballot_publication::get_ballot_publication(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap();

    assert_eq!(publication_ids(&publications), strings(&[draft, live]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_ballot_publication_writes_an_unpublished_publication() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second) = (f.election(a).await, f.election(a).await);

    let publication = ballot_publication::insert_ballot_publication(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        strings(&[first, second]),
        "admin-user".to_string(),
        Some(second.to_string()),
    )
    .await
    .unwrap()
    .unwrap();

    let id = Uuid::parse_str(&publication.id).unwrap();
    assert_eq!(
        publication,
        BallotPublication {
            id: id.to_string(),
            tenant_id: a.tenant_id(),
            election_event_id: a.event_id(),
            labels: None,
            annotations: None,
            created_at: Some(local(now(&tx).await)),
            deleted_at: None,
            created_by_user_id: Some("admin-user".to_string()),
            is_generated: Some(false),
            election_ids: Some(strings(&[first, second])),
            published_at: None,
            election_id: Some(second.to_string()),
        }
    );
    let row = tx
        .query_one(
            "SELECT election_ids, election_id, created_by_user_id
             FROM sequent_backend.ballot_publication
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&a.tenant, &a.event, &id],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, Vec<Uuid>>(0), vec![first, second]);
    assert_eq!(row.get::<_, Option<Uuid>>(1), Some(second));
    assert_eq!(row.get::<_, String>(2), "admin-user");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_ballot_publication_rejects_invalid_election_ids_before_writing() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await.to_string();

    assert_invalid_uuid(
        ballot_publication::insert_ballot_publication(
            &tx,
            &a.tenant_id(),
            &a.event_id(),
            vec![election.clone(), BAD_UUID.to_string()],
            "admin-user".to_string(),
            None,
        )
        .await,
    );
    assert_invalid_uuid(
        ballot_publication::insert_ballot_publication(
            &tx,
            &a.tenant_id(),
            &a.event_id(),
            vec![election],
            "admin-user".to_string(),
            Some(BAD_UUID.to_string()),
        )
        .await,
    );

    let count: i64 = scalar(
        &tx,
        "SELECT count(*) FROM sequent_backend.ballot_publication WHERE tenant_id = $1",
        &[&a.tenant],
    )
    .await;
    assert_eq!(count, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_previous_publication_election_returns_the_latest_earlier_publication_of_the_election()
{
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let only_first = f
        .publication(
            a,
            PublicationRow {
                election_ids: vec![first],
                published_at: Some(at(1)),
                ..Default::default()
            },
        )
        .await;
    let both = f
        .publication(
            a,
            PublicationRow {
                election_ids: vec![first, second],
                published_at: Some(at(2)),
                ..Default::default()
            },
        )
        .await;
    let only_second = f
        .publication(
            a,
            PublicationRow {
                election_ids: vec![second],
                published_at: Some(at(3)),
                ..Default::default()
            },
        )
        .await;
    let previous = |day: u32, election: Uuid| {
        let tx = &tx;
        async move {
            ballot_publication::get_previous_publication_election(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                Some(local(at(day))),
                &election.to_string(),
            )
            .await
            .unwrap()
            .map(|publication| publication.id)
        }
    };

    assert_eq!(previous(4, first).await, Some(both.to_string()));
    assert_eq!(previous(4, second).await, Some(only_second.to_string()));
    assert_eq!(previous(2, first).await, Some(only_first.to_string()));
    assert_eq!(previous(1, first).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_previous_publication_election_includes_soft_deleted_publications() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    // Publishing soft-deletes every other publication, so the previous one
    // is normally a deleted row.
    let retired = f
        .publication(
            a,
            PublicationRow {
                election_ids: vec![election],
                published_at: Some(at(1)),
                deleted_at: Some(at(2)),
                ..Default::default()
            },
        )
        .await;

    let previous = ballot_publication::get_previous_publication_election(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        Some(local(at(2))),
        &election.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(previous.map(|p| p.id), Some(retired.to_string()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_previous_publication_election_ignores_drafts_other_events_and_a_missing_time() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    f.election_as(sibling, election, None).await;
    let earlier = f
        .publication(
            a,
            PublicationRow {
                election_ids: vec![election],
                published_at: Some(at(1)),
                ..Default::default()
            },
        )
        .await;
    f.publication(
        a,
        PublicationRow {
            election_ids: vec![election],
            ..Default::default()
        },
    )
    .await;
    f.publication(
        sibling,
        PublicationRow {
            election_ids: vec![election],
            published_at: Some(at(2)),
            ..Default::default()
        },
    )
    .await;
    let previous = |published_at: Option<DateTime<Local>>| {
        let tx = &tx;
        async move {
            ballot_publication::get_previous_publication_election(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                published_at,
                &election.to_string(),
            )
            .await
            .unwrap()
            .map(|publication| publication.id)
        }
    };

    assert_eq!(
        previous(Some(local(at(3)))).await,
        Some(earlier.to_string())
    );
    // `published_at < NULL` matches nothing.
    assert_eq!(previous(None).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_previous_publication_only_considers_event_wide_publications() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let event_wide = f.publication(a, published(1)).await;
    f.publication(
        a,
        PublicationRow {
            election_id: Some(election),
            election_ids: vec![election],
            published_at: Some(at(2)),
            ..Default::default()
        },
    )
    .await;
    f.publication(a, PublicationRow::default()).await;
    f.publication(sibling, published(2)).await;

    let previous = ballot_publication::get_previous_publication(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        Some(local(at(3))),
    )
    .await
    .unwrap();

    assert_eq!(previous.map(|p| p.id), Some(event_wide.to_string()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_previous_publication_returns_none_before_the_first_publication() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    f.publication(a, published(2)).await;
    let previous = |published_at: Option<DateTime<Local>>| {
        let tx = &tx;
        async move {
            ballot_publication::get_previous_publication(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                published_at,
            )
            .await
            .unwrap()
        }
    };

    assert_eq!(previous(Some(local(at(2)))).await, None);
    assert_eq!(previous(None).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn soft_delete_other_ballot_publications_retires_every_other_live_publication_and_its_styles()
{
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    let target = f.publication(a, published(3)).await;
    let older = f.publication(a, published(1)).await;
    let draft = f.publication(a, PublicationRow::default()).await;
    let retired = f
        .publication(
            a,
            PublicationRow {
                deleted_at: Some(at(2)),
                ..Default::default()
            },
        )
        .await;
    let target_style = f.style(a, target, election, area).await;
    let older_style = f.style(a, older, election, area).await;
    let retired_style = f.deleted_style(a, retired, election, area).await;
    let sibling_election = f.election(sibling).await;
    let sibling_area = f.area(sibling).await;
    let sibling_publication = f.publication(sibling, published(1)).await;
    let sibling_style = f
        .style(sibling, sibling_publication, sibling_election, sibling_area)
        .await;

    let (publications, styles) = ballot_publication::soft_delete_other_ballot_publications(
        &tx,
        &target.to_string(),
        &a.event_id(),
        &a.tenant_id(),
        None,
    )
    .await
    .unwrap();

    let now = now(&tx).await;
    assert_eq!(sorted(publications), strings(&[older, draft]));
    assert_eq!(styles, strings(&[older_style]));
    assert_eq!(publication_state(&tx, a, target).await.2, None);
    assert_eq!(publication_state(&tx, a, older).await.2, Some(now));
    assert_eq!(publication_state(&tx, a, draft).await.2, Some(now));
    assert_eq!(publication_state(&tx, a, retired).await.2, Some(at(2)));
    assert_eq!(
        publication_state(&tx, sibling, sibling_publication).await.2,
        None
    );
    assert_eq!(style_deleted_at(&tx, a, target_style).await, None);
    assert_eq!(style_deleted_at(&tx, a, older_style).await, Some(now));
    assert_eq!(style_deleted_at(&tx, a, retired_style).await, Some(at(20)));
    assert_eq!(style_deleted_at(&tx, sibling, sibling_style).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn soft_delete_other_ballot_publications_for_an_election_only_retires_that_elections_rows() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let area = f.area(a).await;
    let for_election = |election: Uuid, day: u32| PublicationRow {
        election_id: Some(election),
        election_ids: vec![election],
        published_at: Some(at(day)),
        ..Default::default()
    };
    let target = f.publication(a, for_election(first, 3)).await;
    let older_first = f.publication(a, for_election(first, 1)).await;
    let older_second = f.publication(a, for_election(second, 1)).await;
    let event_wide = f.publication(a, published(2)).await;
    let older_first_style = f.style(a, older_first, first, area).await;
    let older_second_style = f.style(a, older_second, second, area).await;
    let event_wide_first_style = f.style(a, event_wide, first, area).await;
    let event_wide_second_style = f.style(a, event_wide, second, area).await;

    let (publications, styles) = ballot_publication::soft_delete_other_ballot_publications(
        &tx,
        &target.to_string(),
        &a.event_id(),
        &a.tenant_id(),
        Some(first.to_string()),
    )
    .await
    .unwrap();

    // The event-wide publication stays live, yet loses its style of the election.
    assert_eq!(publications, strings(&[older_first]));
    assert_eq!(
        sorted(styles),
        strings(&[older_first_style, event_wide_first_style])
    );
    assert_eq!(publication_state(&tx, a, older_second).await.2, None);
    assert_eq!(publication_state(&tx, a, event_wide).await.2, None);
    assert_eq!(style_deleted_at(&tx, a, older_second_style).await, None);
    assert_eq!(
        style_deleted_at(&tx, a, event_wide_second_style).await,
        None
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn soft_delete_other_ballot_publications_rejects_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let target = f.publication(a, published(1)).await.to_string();
    let (tenant, event) = (a.tenant_id(), a.event_id());

    for (publication, event, tenant, election) in [
        (BAD_UUID, event.as_str(), tenant.as_str(), None),
        (target.as_str(), BAD_UUID, tenant.as_str(), None),
        (target.as_str(), event.as_str(), BAD_UUID, None),
        (
            target.as_str(),
            event.as_str(),
            tenant.as_str(),
            Some(BAD_UUID),
        ),
    ] {
        assert_invalid_uuid(
            ballot_publication::soft_delete_other_ballot_publications(
                &tx,
                publication,
                event,
                tenant,
                election.map(str::to_string),
            )
            .await,
        );
    }
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// publication_files
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_publication_annotations_returns_the_annotations_or_none_when_unset() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let annotated = f
        .publication(
            a,
            PublicationRow {
                annotations: Some(json!({FILES_ANNOTATION: "root"})),
                ..Default::default()
            },
        )
        .await;
    let bare = f.publication(a, PublicationRow::default()).await;

    assert_eq!(
        publication_files::get_publication_annotations(&tx, a.tenant, a.event, annotated)
            .await
            .unwrap(),
        Some(json!({FILES_ANNOTATION: "root"}))
    );
    assert_eq!(
        publication_files::get_publication_annotations(&tx, a.tenant, a.event, bare)
            .await
            .unwrap(),
        None
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_publication_annotations_fails_for_a_publication_outside_the_scope() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let publication = f.publication(a, PublicationRow::default()).await;

    for (tenant, event, publication) in [
        (a.tenant, sibling.event, publication),
        (a.tenant, a.event, f.id()),
    ] {
        let error = publication_files::get_publication_annotations(&tx, tenant, event, publication)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "query returned an unexpected number of rows"
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_publication_event_returns_the_id_presentation_and_description_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET presentation = $2, description = 'Board election', labels = '{\"x\": 1}'
         WHERE id = $1",
        &[&a.event, &json!({"i18n": {"en": {"name": "Board"}}})],
    )
    .await
    .unwrap();

    let event = publication_files::get_publication_event(&tx, a.tenant, a.event)
        .await
        .unwrap();

    assert_eq!(
        event,
        json!({
            "id": a.event_id(),
            "presentation": {"i18n": {"en": {"name": "Board"}}},
            "description": "Board election",
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_publication_event_fails_for_an_event_of_another_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;

    let error = publication_files::get_publication_event(&tx, other.tenant, a.event)
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "query returned an unexpected number of rows"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_publication_elections_returns_the_elections_listed_by_the_publication() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let listed = f.election(a).await;
    f.election(a).await; // not listed
    f.election_as(sibling, listed, None).await;
    tx.execute(
        "UPDATE sequent_backend.election
         SET presentation = '{\"sort_order\": 1}', description = 'Listed',
             spoil_ballot_option = true, is_consolidated_ballot_encoding = false,
             labels = '{\"l\": 1}', annotations = '{\"a\": 1}', eml = 'not exported'
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&a.tenant, &a.event, &listed],
    )
    .await
    .unwrap();
    let publication = f
        .publication(
            a,
            PublicationRow {
                election_ids: vec![listed],
                ..Default::default()
            },
        )
        .await;

    let elections =
        publication_files::get_publication_elections(&tx, a.tenant, a.event, publication)
            .await
            .unwrap();

    assert_eq!(elections.len(), 1, "{elections:?}");
    let election = elections[0].as_object().unwrap();
    assert_eq!(
        sorted(election.keys().cloned().collect()),
        vec![
            "annotations",
            "created_at",
            "description",
            "election_event_id",
            "id",
            "is_consolidated_ballot_encoding",
            "labels",
            "last_updated_at",
            "presentation",
            "spoil_ballot_option",
            "tenant_id",
        ]
    );
    assert_eq!(election["id"], json!(listed.to_string()));
    assert_eq!(election["tenant_id"], json!(a.tenant_id()));
    assert_eq!(election["election_event_id"], json!(a.event_id()));
    assert_eq!(election["presentation"], json!({"sort_order": 1}));
    assert_eq!(election["description"], json!("Listed"));
    assert_eq!(election["spoil_ballot_option"], json!(true));
    assert_eq!(election["is_consolidated_ballot_encoding"], json!(false));
    assert_eq!(election["labels"], json!({"l": 1}));
    assert_eq!(election["annotations"], json!({"a": 1}));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn stream_publication_styles_streams_every_style_of_the_publication_including_deleted_ones() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL TimeZone = 'UTC'")
        .await
        .unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    let publication = f.publication(a, published(1)).await;
    let other_publication = f.publication(a, published(2)).await;
    let live = f.style(a, publication, election, area).await;
    let deleted = f.deleted_style(a, publication, election, area).await;
    f.style(a, other_publication, election, area).await;
    f.election_as(sibling, election, None).await;
    f.area_as(sibling, area).await;
    f.publication_as(sibling, publication, published(1)).await;
    f.style(sibling, publication, election, area).await;
    tx.execute(
        "UPDATE sequent_backend.ballot_style
         SET ballot_eml = '{\"ballot\": 1}', ballot_signature = '\\x0102'::bytea,
             status = 'PUBLISHED', labels = '{\"l\": 1}', annotations = '{\"a\": 1}'
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&a.tenant, &a.event, &live],
    )
    .await
    .unwrap();

    let styles: Vec<Value> =
        publication_files::stream_publication_styles(&tx, a.tenant, a.event, publication)
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

    let mut by_id: Vec<(String, Value)> = styles
        .into_iter()
        .map(|style| (style["id"].as_str().unwrap().to_string(), style))
        .collect();
    by_id.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        by_id.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        strings(&[live, deleted])
    );
    let mut live_style = by_id[0].1.clone();
    let created_at = live_style
        .as_object_mut()
        .unwrap()
        .remove("created_at")
        .unwrap();
    let last_updated_at = live_style
        .as_object_mut()
        .unwrap()
        .remove("last_updated_at")
        .unwrap();
    assert!(created_at.is_string() && last_updated_at.is_string());
    assert_eq!(
        live_style,
        json!({
            "id": live.to_string(),
            "tenant_id": a.tenant_id(),
            "election_event_id": a.event_id(),
            "election_id": election.to_string(),
            "area_id": area.to_string(),
            "annotations": {"a": 1},
            "labels": {"l": 1},
            "ballot_eml": "{\"ballot\": 1}",
            "ballot_signature": "\\x0102",
            "status": "PUBLISHED",
            "deleted_at": null,
        })
    );
    assert_eq!(by_id[1].1["deleted_at"], json!("2026-01-20T12:00:00+00:00"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn merge_ballot_publication_annotation_adds_or_replaces_one_text_annotation() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let annotated = f
        .publication(
            a,
            PublicationRow {
                annotations: Some(json!({"kept": {"nested": true}, FILES_ANNOTATION: "old"})),
                ..Default::default()
            },
        )
        .await;
    let bare = f.publication(a, PublicationRow::default()).await;

    for publication in [annotated, bare] {
        publication_files::merge_ballot_publication_annotation(
            &tx,
            a.tenant,
            a.event,
            publication,
            FILES_ANNOTATION,
            "new-root",
        )
        .await
        .unwrap();
    }

    let annotations = |id: Uuid| {
        let tx = &tx;
        async move {
            scalar::<Option<Value>>(
                tx,
                "SELECT annotations FROM sequent_backend.ballot_publication
                 WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
                &[&a.tenant, &a.event, &id],
            )
            .await
        }
    };
    assert_eq!(
        annotations(annotated).await,
        Some(json!({"kept": {"nested": true}, FILES_ANNOTATION: "new-root"}))
    );
    assert_eq!(
        annotations(bare).await,
        Some(json!({FILES_ANNOTATION: "new-root"}))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn merge_ballot_publication_annotation_only_updates_the_publication_of_the_scope() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let id = f.publication(a, PublicationRow::default()).await;
    f.publication_as(other, id, PublicationRow::default()).await;
    f.publication(a, PublicationRow::default()).await;

    publication_files::merge_ballot_publication_annotation(
        &tx, a.tenant, a.event, id, "key", "value",
    )
    .await
    .unwrap();

    let annotated: Vec<(Uuid, Uuid)> = tx
        .query(
            "SELECT tenant_id, id FROM sequent_backend.ballot_publication
             WHERE annotations IS NOT NULL AND tenant_id = ANY($1)",
            &[&vec![a.tenant, other.tenant]],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    assert_eq!(annotated, vec![(a.tenant, id)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_published_ballot_styles_returns_live_styles_of_published_publications_newest_first() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second, unrequested) = (
        f.election(a).await,
        f.election(a).await,
        f.election(a).await,
    );
    let (voter_area, other_area) = (f.area(a).await, f.area(a).await);
    let older = f.publication(a, published(1)).await;
    let newer = f.publication(a, published(2)).await;
    let draft = f.publication(a, PublicationRow::default()).await;
    let retired = f
        .publication(
            a,
            PublicationRow {
                published_at: Some(at(3)),
                deleted_at: Some(at(4)),
                ..Default::default()
            },
        )
        .await;
    let older_first = f.style(a, older, first, voter_area).await;
    f.deleted_style(a, older, second, voter_area).await;
    let newer_second = f.style(a, newer, second, voter_area).await;
    let newer_first = f.style(a, newer, first, voter_area).await;
    f.style(a, newer, first, other_area).await;
    f.style(a, newer, unrequested, voter_area).await;
    f.style(a, draft, first, voter_area).await;
    f.style(a, retired, first, voter_area).await;

    let styles = publication_files::get_published_ballot_styles(
        &tx,
        a.tenant,
        a.event,
        voter_area,
        &[first, second],
        FILES_ANNOTATION,
    )
    .await
    .unwrap();

    assert_eq!(
        styles
            .iter()
            .map(|style| (style.id, style.publication_id, style.election_id))
            .collect::<Vec<_>>(),
        vec![
            (newer_first, newer, first),
            (newer_second, newer, second),
            (older_first, older, first),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_published_ballot_styles_maps_the_object_root_and_the_live_election_policy() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election_with_revotes(a, 3).await;
    let bare_election = f.election(a).await;
    tx.execute(
        "UPDATE sequent_backend.election
         SET status = '{\"voting_status\": \"OPEN\"}', voting_channels = '{\"online\": true}'
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&a.tenant, &a.event, &election],
    )
    .await
    .unwrap();
    let area = f.area(a).await;
    let rooted = f
        .publication(
            a,
            PublicationRow {
                published_at: Some(at(2)),
                annotations: Some(json!({FILES_ANNOTATION: "tenant-x/root", "other": "y"})),
                ..Default::default()
            },
        )
        .await;
    let rootless = f.publication(a, published(1)).await;
    let rooted_style = f.style(a, rooted, election, area).await;
    let rootless_style = f.style(a, rootless, bare_election, area).await;

    let styles = publication_files::get_published_ballot_styles(
        &tx,
        a.tenant,
        a.event,
        area,
        &[election, bare_election],
        FILES_ANNOTATION,
    )
    .await
    .unwrap();

    assert_eq!(
        styles,
        vec![
            PublishedBallotStyle {
                id: rooted_style,
                election_id: election,
                publication_id: rooted,
                root: Some("tenant-x/root".to_string()),
                status: Some(json!({"voting_status": "OPEN"})),
                num_allowed_revotes: Some(3),
                voting_channels: Some(json!({"online": true})),
            },
            PublishedBallotStyle {
                id: rootless_style,
                election_id: bare_election,
                publication_id: rootless,
                root: None,
                status: None,
                num_allowed_revotes: None,
                voting_channels: None,
            },
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_published_ballot_styles_never_returns_styles_of_other_tenants_or_events() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    let [election, area, publication, style] = [f.id(), f.id(), f.id(), f.id()];
    // Identical ids in three scopes; only the root annotation tells them apart.
    for (scope, root) in [(a, "a"), (sibling, "sibling"), (other, "other")] {
        f.election_as(scope, election, None).await;
        f.area_as(scope, area).await;
        f.publication_as(
            scope,
            publication,
            PublicationRow {
                published_at: Some(at(1)),
                annotations: Some(json!({FILES_ANNOTATION: root})),
                ..Default::default()
            },
        )
        .await;
        f.style_as(scope, style, [publication, election, area], None)
            .await;
    }

    let styles = publication_files::get_published_ballot_styles(
        &tx,
        a.tenant,
        a.event,
        area,
        &[election],
        FILES_ANNOTATION,
    )
    .await
    .unwrap();

    assert_eq!(
        styles
            .iter()
            .map(|style| style.root.clone())
            .collect::<Vec<_>>(),
        vec![Some("a".to_string())]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_event_status_distinguishes_a_missing_event_from_a_missing_status() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let with_status = f.scope().await;
    let without_status = f.event_in(with_status.tenant).await;
    let other = f.scope().await;
    tx.execute(
        "UPDATE sequent_backend.election_event SET status = '{\"voting_status\": \"OPEN\"}'
         WHERE id = $1",
        &[&with_status.event],
    )
    .await
    .unwrap();
    let status = |tenant: Uuid, event: Uuid| {
        let tx = &tx;
        async move {
            publication_files::get_election_event_status(tx, tenant, event)
                .await
                .unwrap()
        }
    };

    assert_eq!(
        status(with_status.tenant, with_status.event).await,
        Some(Some(json!({"voting_status": "OPEN"})))
    );
    assert_eq!(
        status(without_status.tenant, without_status.event).await,
        Some(None)
    );
    assert_eq!(status(other.tenant, with_status.event).await, None);
    assert_eq!(status(with_status.tenant, f.id()).await, None);
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// ballot_style
// ---------------------------------------------------------------------------

#[tokio::test]
async fn insert_ballot_style_writes_and_returns_the_style() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    let publication = f.publication(a, PublicationRow::default()).await;
    let id = f.id();

    let style = ballot_style::insert_ballot_style(
        &tx,
        &id.to_string(),
        &a.tenant_id(),
        &a.event_id(),
        &election.to_string(),
        &area.to_string(),
        Some("{\"ballot\": true}".to_string()),
        Some("PENDING".to_string()),
        &publication.to_string(),
    )
    .await
    .unwrap();

    let now = Some(local(now(&tx).await));
    assert_eq!(
        style,
        BallotStyle {
            id: id.to_string(),
            tenant_id: a.tenant_id(),
            election_id: election.to_string(),
            area_id: Some(area.to_string()),
            created_at: now,
            last_updated_at: now,
            labels: None,
            annotations: None,
            ballot_eml: Some("{\"ballot\": true}".to_string()),
            ballot_signature: None,
            status: Some("PENDING".to_string()),
            election_event_id: a.event_id(),
            deleted_at: None,
            ballot_publication_id: publication.to_string(),
        }
    );
    let row = tx
        .query_one(
            "SELECT election_id, area_id, ballot_publication_id, ballot_eml, status
             FROM sequent_backend.ballot_style
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&a.tenant, &a.event, &id],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, Uuid>(0), election);
    assert_eq!(row.get::<_, Uuid>(1), area);
    assert_eq!(row.get::<_, Uuid>(2), publication);
    assert_eq!(row.get::<_, String>(3), "{\"ballot\": true}");
    assert_eq!(row.get::<_, String>(4), "PENDING");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_ballot_style_reports_a_publication_of_another_event_as_an_opaque_insert_error() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    let foreign_publication = f.publication(sibling, PublicationRow::default()).await;

    let error = ballot_style::insert_ballot_style(
        &tx,
        &f.id().to_string(),
        &a.tenant_id(),
        &a.event_id(),
        &election.to_string(),
        &area.to_string(),
        None,
        None,
        &foreign_publication.to_string(),
    )
    .await
    .unwrap_err();

    // The foreign key violation is flattened to tokio-postgres' "db error".
    assert_eq!(error.to_string(), "Error inserting row: db error");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_ballot_style_rejects_invalid_uuids_before_writing() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await.to_string();
    let area = f.area(a).await.to_string();
    let publication = f
        .publication(a, PublicationRow::default())
        .await
        .to_string();
    let id = f.id().to_string();
    let (tenant, event) = (a.tenant_id(), a.event_id());
    let fields = [&id, &tenant, &event, &election, &area, &publication];

    for position in 0..fields.len() {
        let mut values: Vec<&str> = fields.iter().map(|value| value.as_str()).collect();
        values[position] = BAD_UUID;
        assert_invalid_uuid(
            ballot_style::insert_ballot_style(
                &tx, values[0], values[1], values[2], values[3], values[4], None, None, values[5],
            )
            .await,
        );
    }
    let count: i64 = scalar(
        &tx,
        "SELECT count(*) FROM sequent_backend.ballot_style WHERE tenant_id = $1",
        &[&a.tenant],
    )
    .await;
    assert_eq!(count, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_all_ballot_styles_returns_the_live_styles_of_the_area_and_elections() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (requested, unrequested) = (f.election(a).await, f.election(a).await);
    let (area, other_area) = (f.area(a).await, f.area(a).await);
    let publication = f.publication(a, published(1)).await;
    let wanted = f.style(a, publication, requested, area).await;
    f.deleted_style(a, publication, requested, area).await;
    f.style(a, publication, unrequested, area).await;
    f.style(a, publication, requested, other_area).await;

    let styles = ballot_style::get_all_ballot_styles(
        &tx,
        &a.tenant_id(),
        &area.to_string(),
        &strings(&[requested]),
    )
    .await
    .unwrap();

    assert_eq!(style_ids(&styles), strings(&[wanted]));
    assert_eq!(styles[0].area_id, Some(area.to_string()));
    assert_eq!(styles[0].election_id, requested.to_string());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_all_ballot_styles_never_returns_styles_of_other_tenants() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    let [election, area, publication, style] = [f.id(), f.id(), f.id(), f.id()];
    for scope in [a, sibling, other] {
        f.election_as(scope, election, None).await;
        f.area_as(scope, area).await;
        f.publication_as(scope, publication, published(1)).await;
        f.style_as(scope, style, [publication, election, area], None)
            .await;
    }

    let styles = ballot_style::get_all_ballot_styles(
        &tx,
        &a.tenant_id(),
        &area.to_string(),
        &strings(&[election]),
    )
    .await
    .unwrap();

    // Without an event argument, identical ids in another event of the
    // tenant match too.
    let mut scopes: Vec<(String, String)> = styles
        .iter()
        .map(|style| (style.tenant_id.clone(), style.election_event_id.clone()))
        .collect();
    scopes.sort();
    assert_eq!(
        scopes,
        sorted(vec![
            (a.tenant_id(), a.event_id()),
            (a.tenant_id(), sibling.event_id()),
        ])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_all_ballot_styles_returns_nothing_without_a_matching_style() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (area, empty_area) = (f.area(a).await, f.area(a).await);
    let publication = f.publication(a, published(1)).await;
    f.style(a, publication, election, area).await;
    let styles = |area: Uuid, elections: Vec<String>| {
        let tx = &tx;
        async move {
            ballot_style::get_all_ballot_styles(tx, &a.tenant_id(), &area.to_string(), &elections)
                .await
                .unwrap()
        }
    };

    assert!(styles(empty_area, strings(&[election])).await.is_empty());
    assert!(styles(area, vec![]).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_all_ballot_styles_rejects_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (tenant, area, election) = (a.tenant_id(), f.id().to_string(), f.id().to_string());

    for (tenant, area, election) in [
        (BAD_UUID, area.as_str(), election.as_str()),
        (tenant.as_str(), BAD_UUID, election.as_str()),
        (tenant.as_str(), area.as_str(), BAD_UUID),
    ] {
        assert_invalid_uuid(
            ballot_style::get_all_ballot_styles(&tx, tenant, area, &vec![election.to_string()])
                .await,
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn export_event_ballot_styles_returns_the_live_styles_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let area = f.area(a).await;
    let publication = f.publication(a, published(1)).await;
    let first_style = f.style(a, publication, first, area).await;
    let second_style = f.style(a, publication, second, area).await;
    f.deleted_style(a, publication, first, area).await;
    let sibling_election = f.election(sibling).await;
    let sibling_area = f.area(sibling).await;
    let sibling_publication = f.publication(sibling, published(1)).await;
    f.style(sibling, sibling_publication, sibling_election, sibling_area)
        .await;

    let styles = ballot_style::export_event_ballot_styles(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    assert_eq!(style_ids(&styles), strings(&[first_style, second_style]));
    assert!(styles
        .iter()
        .all(|style| style.ballot_publication_id == publication.to_string()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_ballot_styles_by_elections_returns_live_styles_of_the_requested_elections() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let (requested, unrequested) = (f.election(a).await, f.election(a).await);
    let area = f.area(a).await;
    let publication = f.publication(a, published(1)).await;
    let wanted = f.style(a, publication, requested, area).await;
    f.style(a, publication, unrequested, area).await;
    f.deleted_style(a, publication, requested, area).await;
    f.election_as(other, requested, None).await;
    f.area_as(other, area).await;
    f.publication_as(other, publication, published(1)).await;
    f.style(other, publication, requested, area).await;

    let styles = ballot_style::get_ballot_styles_by_elections(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &strings(&[requested]),
    )
    .await
    .unwrap();

    assert_eq!(style_ids(&styles), strings(&[wanted]));
    assert_invalid_uuid(
        ballot_style::get_ballot_styles_by_elections(
            &tx,
            &a.tenant_id(),
            &a.event_id(),
            &vec![BAD_UUID.to_string()],
        )
        .await,
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_ballot_styles_by_elections_includes_live_styles_of_unpublished_publications() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    let live = f.publication(a, published(1)).await;
    let draft = f.publication(a, PublicationRow::default()).await;
    let live_style = f.style(a, live, election, area).await;
    let draft_style = f.style(a, draft, election, area).await;

    let styles = ballot_style::get_ballot_styles_by_elections(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &strings(&[election]),
    )
    .await
    .unwrap();

    assert_eq!(
        style_ids(&styles),
        sorted(strings(&[live_style, draft_style]))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_publication_ballot_styles_orders_by_election_and_area_and_honours_the_limit() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let (low_area, high_area) = (f.area(a).await, f.area(a).await);
    let publication = f.publication(a, published(1)).await;
    let other_publication = f.publication(a, published(2)).await;
    let second_low = f.style(a, publication, second, low_area).await;
    let first_high = f.deleted_style(a, publication, first, high_area).await;
    let first_low = f.style(a, publication, first, low_area).await;
    f.style(a, other_publication, first, low_area).await;
    let styles = |limit: Option<usize>| {
        let tx = &tx;
        async move {
            ballot_style::get_publication_ballot_styles(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                &publication.to_string(),
                limit,
            )
            .await
            .unwrap()
            .into_iter()
            .map(|style| style.id)
            .collect::<Vec<_>>()
        }
    };

    // Soft-deleted styles are included.
    assert_eq!(
        styles(None).await,
        strings(&[first_low, first_high, second_low])
    );
    assert_eq!(styles(Some(2)).await, strings(&[first_low, first_high]));
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// cast_vote
// ---------------------------------------------------------------------------

struct Ballot<'a> {
    voter: &'a str,
    status: CastVoteStatus,
}

fn valid(voter: &str) -> Ballot<'_> {
    Ballot {
        voter,
        status: CastVoteStatus::Valid,
    }
}

async fn cast(
    tx: &Transaction<'_>,
    scope: Scope,
    [election, area]: [Uuid; 2],
    ballot: Ballot<'_>,
) -> anyhow::Result<CastVote> {
    cast_vote::insert_cast_vote(
        tx,
        &scope.tenant,
        &scope.event,
        &election,
        &area,
        "ciphertext",
        ballot.voter,
        "ballot-hash",
        &[1, 2, 3],
        &Some("203.0.113.7".to_string()),
        &Some("ES".to_string()),
        VotingStatusChannel::KIOSK,
        ballot.status,
    )
    .await
}

/// Inserts a vote the trigger must reject, restoring the transaction after.
async fn rejected(
    tx: &Transaction<'_>,
    scope: Scope,
    target: [Uuid; 2],
    ballot: Ballot<'_>,
) -> anyhow::Error {
    tx.batch_execute("SAVEPOINT rejected_vote").await.unwrap();
    let error = cast(tx, scope, target, ballot).await.unwrap_err();
    tx.batch_execute("ROLLBACK TO SAVEPOINT rejected_vote")
        .await
        .unwrap();
    error
}

/// What `insert_cast_vote` callers inspect: the context message and the
/// PL/pgSQL `RAISE EXCEPTION` of the database error it wraps.
fn assert_trigger_rejection(error: &anyhow::Error, message: &str) {
    assert_eq!(error.to_string(), "Error inserting cast vote");
    let database = error
        .downcast_ref::<tokio_postgres::Error>()
        .and_then(tokio_postgres::Error::as_db_error)
        .expect("a database error");
    assert_eq!(database.code(), &SqlState::RAISE_EXCEPTION);
    assert_eq!(database.message(), message);
}

async fn vote_count(tx: &Transaction<'_>, scope: Scope, voter: &str) -> i64 {
    scalar(
        tx,
        "SELECT count(*) FROM sequent_backend.cast_vote
         WHERE tenant_id = $1 AND election_event_id = $2 AND voter_id_string = $3",
        &[&scope.tenant, &scope.event, &voter],
    )
    .await
}

async fn vote_status(tx: &Transaction<'_>, scope: Scope, id: Uuid) -> String {
    scalar(
        tx,
        "SELECT status FROM sequent_backend.cast_vote
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&scope.tenant, &scope.event, &id],
    )
    .await
}

#[tokio::test]
async fn insert_cast_vote_returns_the_vote_and_stores_its_channel_annotations() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let area = f.area(a).await;

    let vote = cast(
        &tx,
        a,
        [election, area],
        Ballot {
            voter: "voter-1",
            status: CastVoteStatus::InProgress,
        },
    )
    .await
    .unwrap();

    let now = Some(now(&tx).await);
    assert_eq!(
        vote,
        CastVote {
            id: vote.id.clone(),
            tenant_id: a.tenant_id(),
            election_id: Some(election.to_string()),
            area_id: Some(area.to_string()),
            created_at: now,
            last_updated_at: now,
            content: Some("ciphertext".to_string()),
            voter_id_string: Some("voter-1".to_string()),
            election_event_id: a.event_id(),
            ballot_id: Some("ballot-hash".to_string()),
            cast_ballot_signature: Some(vec![1, 2, 3]),
            status: CastVoteStatus::InProgress,
        }
    );
    let row = tx
        .query_one(
            "SELECT annotations, status, content FROM sequent_backend.cast_vote
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&a.tenant, &a.event, &Uuid::parse_str(&vote.id).unwrap()],
        )
        .await
        .unwrap();
    assert_eq!(
        row.get::<_, Value>(0),
        json!({"ip": "203.0.113.7", "country": "ES", "voting_channel": "KIOSK"})
    );
    assert_eq!(row.get::<_, String>(1), "in-progress");
    assert_eq!(row.get::<_, String>(2), "ciphertext");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_allows_a_single_vote_when_the_election_sets_no_revote_limit() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let area = f.area(a).await;

    cast(&tx, a, [election, area], valid("voter"))
        .await
        .unwrap();
    let error = rejected(&tx, a, [election, area], valid("voter")).await;

    assert_trigger_rejection(&error, "insert_failed_exceeds_allowed_revotes");
    assert_eq!(vote_count(&tx, a, "voter").await, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_allows_as_many_votes_as_num_allowed_revotes() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election_with_revotes(a, 2).await;
    let area = f.area(a).await;

    cast(&tx, a, [election, area], valid("voter"))
        .await
        .unwrap();
    cast(&tx, a, [election, area], valid("voter"))
        .await
        .unwrap();
    let error = rejected(&tx, a, [election, area], valid("voter")).await;

    assert_trigger_rejection(&error, "insert_failed_exceeds_allowed_revotes");
    assert_eq!(vote_count(&tx, a, "voter").await, 2);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_allows_unlimited_votes_when_num_allowed_revotes_is_zero() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election_with_revotes(a, 0).await;
    let area = f.area(a).await;

    for _ in 0..4 {
        cast(&tx, a, [election, area], valid("voter"))
            .await
            .unwrap();
    }

    assert_eq!(vote_count(&tx, a, "voter").await, 4);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_does_not_count_discarded_votes_against_the_limit() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    f.vote(a, election, area, "voter", "discarded").await;
    f.vote(a, election, area, "voter", "discarded").await;

    cast(&tx, a, [election, area], valid("voter"))
        .await
        .unwrap();

    assert_eq!(vote_count(&tx, a, "voter").await, 3);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_counts_in_progress_votes_against_the_limit() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    f.vote(a, election, area, "voter", "in-progress").await;

    let error = rejected(&tx, a, [election, area], valid("voter")).await;

    assert_trigger_rejection(&error, "insert_failed_exceeds_allowed_revotes");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_limits_each_voter_and_election_separately() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let area = f.area(a).await;
    f.election_as(sibling, first, None).await;
    f.area_as(sibling, area).await;
    f.vote(a, first, area, "voter", "valid").await;

    cast(&tx, a, [first, area], valid("another-voter"))
        .await
        .unwrap();
    cast(&tx, a, [second, area], valid("voter")).await.unwrap();
    cast(&tx, sibling, [first, area], valid("voter"))
        .await
        .unwrap();

    assert_eq!(vote_count(&tx, a, "voter").await, 2);
    assert_eq!(vote_count(&tx, sibling, "voter").await, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_rejects_a_second_area_even_with_unlimited_revotes() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election_with_revotes(a, 0).await;
    let (first_area, second_area) = (f.area(a).await, f.area(a).await);
    cast(&tx, a, [election, first_area], valid("voter"))
        .await
        .unwrap();

    let error = rejected(&tx, a, [election, second_area], valid("voter")).await;

    assert_trigger_rejection(&error, "check_votes_in_other_areas_failed");
    assert_eq!(vote_count(&tx, a, "voter").await, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_reports_a_second_area_before_an_exhausted_revote_limit() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (first_area, second_area) = (f.area(a).await, f.area(a).await);
    cast(&tx, a, [election, first_area], valid("voter"))
        .await
        .unwrap();

    // Both rules reject this vote; the area rule is checked first.
    let error = rejected(&tx, a, [election, second_area], valid("voter")).await;

    assert_trigger_rejection(&error, "check_votes_in_other_areas_failed");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_cast_vote_accepts_another_area_once_the_earlier_votes_are_discarded() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (first_area, second_area) = (f.area(a).await, f.area(a).await);
    f.vote(a, election, first_area, "voter", "discarded").await;

    let vote = cast(&tx, a, [election, second_area], valid("voter"))
        .await
        .unwrap();

    assert_eq!(vote.area_id, Some(second_area.to_string()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn compare_and_set_cast_vote_status_moves_only_a_vote_still_in_the_expected_status() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election_with_revotes(a, 0).await;
    let area = f.area(a).await;
    let target = f.vote(a, election, area, "voter", "in-progress").await;
    let neighbour = f.vote(a, election, area, "voter", "in-progress").await;
    tx.execute(
        "UPDATE sequent_backend.cast_vote SET last_updated_at = $2 WHERE id = $1",
        &[&target, &at(1)],
    )
    .await
    .unwrap();
    let transition = |expected: CastVoteStatus, new: CastVoteStatus| {
        let tx = &tx;
        async move {
            cast_vote::compare_and_set_cast_vote_status(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                &target,
                expected,
                new,
            )
            .await
            .unwrap()
        }
    };

    assert!(transition(CastVoteStatus::InProgress, CastVoteStatus::Valid).await);
    assert!(!transition(CastVoteStatus::InProgress, CastVoteStatus::Discarded).await);

    assert_eq!(vote_status(&tx, a, target).await, "valid");
    assert_eq!(vote_status(&tx, a, neighbour).await, "in-progress");
    let updated: DateTime<Utc> = scalar(
        &tx,
        "SELECT last_updated_at FROM sequent_backend.cast_vote WHERE id = $1",
        &[&target],
    )
    .await;
    assert_eq!(updated, now(&tx).await);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn compare_and_set_cast_vote_status_ignores_votes_of_other_tenants_and_events() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    let [election, area, id] = [f.id(), f.id(), f.id()];
    for scope in [a, sibling, other] {
        f.election_as(scope, election, None).await;
        f.area_as(scope, area).await;
        f.vote_as(scope, id, [election, area], "voter", "in-progress")
            .await;
    }

    let moved = cast_vote::compare_and_set_cast_vote_status(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &id,
        CastVoteStatus::InProgress,
        CastVoteStatus::Discarded,
    )
    .await
    .unwrap();
    let mismatched = cast_vote::compare_and_set_cast_vote_status(
        &tx,
        &other.tenant_id(),
        &sibling.event_id(),
        &id,
        CastVoteStatus::InProgress,
        CastVoteStatus::Discarded,
    )
    .await
    .unwrap();

    assert!(moved);
    assert!(!mismatched);
    assert_eq!(vote_status(&tx, a, id).await, "discarded");
    assert_eq!(vote_status(&tx, sibling, id).await, "in-progress");
    assert_eq!(vote_status(&tx, other, id).await, "in-progress");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_cast_vote_by_id_returns_the_vote_of_the_given_tenant_and_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let area = f.area(a).await;
    let id = f.vote(a, election, area, "voter", "discarded").await;
    let get = |scope: Scope, id: Uuid| {
        let tx = &tx;
        async move {
            cast_vote::get_cast_vote_by_id(tx, &scope.tenant_id(), &scope.event_id(), &id)
                .await
                .unwrap()
        }
    };

    let vote = get(a, id).await.unwrap();

    let now = Some(now(&tx).await);
    assert_eq!(
        vote,
        CastVote {
            id: id.to_string(),
            tenant_id: a.tenant_id(),
            election_id: Some(election.to_string()),
            area_id: Some(area.to_string()),
            created_at: now,
            last_updated_at: now,
            content: Some("ciphertext".to_string()),
            voter_id_string: Some("voter".to_string()),
            election_event_id: a.event_id(),
            ballot_id: None,
            cast_ballot_signature: None,
            status: CastVoteStatus::Discarded,
        }
    );
    assert_eq!(get(sibling, id).await, None);
    assert_eq!(get(a, f.id()).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn has_valid_cast_vote_only_counts_valid_votes_of_the_voter_in_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election_with_revotes(a, 0).await;
    let area = f.area(a).await;
    let sibling_election = f.election(sibling).await;
    let sibling_area = f.area(sibling).await;
    f.vote(a, election, area, "pending", "in-progress").await;
    f.vote(a, election, area, "pending", "discarded").await;
    f.vote(a, election, area, "voted", "discarded").await;
    f.vote(a, election, area, "voted", "valid").await;
    f.vote(
        sibling,
        sibling_election,
        sibling_area,
        "elsewhere",
        "valid",
    )
    .await;
    let has_valid = |voter: &'static str| {
        let tx = &tx;
        async move {
            cast_vote::has_valid_cast_vote(tx, &a.tenant_id(), &a.event_id(), voter)
                .await
                .unwrap()
        }
    };

    assert!(has_valid("voted").await);
    assert!(!has_valid("pending").await);
    assert!(!has_valid("elsewhere").await);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn count_unresolved_cast_votes_counts_in_progress_votes_of_one_election_area() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (election, other_election) = (
        f.election_with_revotes(a, 0).await,
        f.election_with_revotes(a, 0).await,
    );
    let (area, other_area) = (f.area(a).await, f.area(a).await);
    f.vote(a, election, area, "first", "in-progress").await;
    f.vote(a, election, area, "first", "in-progress").await;
    f.vote(a, election, area, "second", "in-progress").await;
    f.vote(a, election, area, "third", "valid").await;
    f.vote(a, election, area, "fourth", "discarded").await;
    f.vote(a, election, other_area, "fifth", "in-progress")
        .await;
    f.vote(a, other_election, area, "first", "in-progress")
        .await;

    let count = cast_vote::count_unresolved_cast_votes(&tx, &a.tenant, &a.event, &election, &area)
        .await
        .unwrap();

    assert_eq!(count, 3);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn discard_voter_cast_votes_discards_the_active_votes_of_the_voter_in_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (
        f.election_with_revotes(a, 0).await,
        f.election_with_revotes(a, 0).await,
    );
    let area = f.area(a).await;
    let sibling_election = f.election(sibling).await;
    let sibling_area = f.area(sibling).await;
    let valid_vote = f.vote(a, first, area, "voter", "valid").await;
    let pending_vote = f.vote(a, second, area, "voter", "in-progress").await;
    let discarded_vote = f.vote(a, first, area, "voter", "discarded").await;
    let other_voter = f.vote(a, first, area, "other-voter", "valid").await;
    let sibling_vote = f
        .vote(sibling, sibling_election, sibling_area, "voter", "valid")
        .await;
    tx.execute(
        "UPDATE sequent_backend.cast_vote SET last_updated_at = $1 WHERE id = $2",
        &[&at(1), &discarded_vote],
    )
    .await
    .unwrap();

    let discarded = cast_vote::discard_voter_cast_votes(&tx, &a.tenant, &a.event, "voter")
        .await
        .unwrap();

    assert_eq!(discarded, 2);
    assert_eq!(vote_status(&tx, a, valid_vote).await, "discarded");
    assert_eq!(vote_status(&tx, a, pending_vote).await, "discarded");
    assert_eq!(vote_status(&tx, a, other_voter).await, "valid");
    assert_eq!(vote_status(&tx, sibling, sibling_vote).await, "valid");
    // Already discarded votes are not rewritten.
    let untouched: DateTime<Utc> = scalar(
        &tx,
        "SELECT last_updated_at FROM sequent_backend.cast_vote WHERE id = $1",
        &[&discarded_vote],
    )
    .await;
    assert_eq!(untouched, at(1));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_voter_cast_vote_state_reports_unresolved_and_valid_votes_of_the_voter() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election_with_revotes(a, 0).await;
    let area = f.area(a).await;
    let sibling_election = f.election(sibling).await;
    let sibling_area = f.area(sibling).await;
    f.vote(a, election, area, "both", "valid").await;
    f.vote(a, election, area, "both", "in-progress").await;
    f.vote(a, election, area, "pending", "in-progress").await;
    f.vote(a, election, area, "voted", "valid").await;
    f.vote(a, election, area, "discarded", "discarded").await;
    f.vote(
        sibling,
        sibling_election,
        sibling_area,
        "elsewhere",
        "valid",
    )
    .await;
    let state = |voter: &'static str| {
        let tx = &tx;
        async move {
            let state = cast_vote::get_voter_cast_vote_state(tx, &a.tenant, &a.event, voter)
                .await
                .unwrap();
            (state.has_unresolved_vote, state.has_valid_vote)
        }
    };

    assert_eq!(state("both").await, (true, true));
    assert_eq!(state("pending").await, (true, false));
    assert_eq!(state("voted").await, (false, true));
    assert_eq!(state("discarded").await, (false, false));
    assert_eq!(state("elsewhere").await, (false, false));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_cast_votes_filters_by_election_voter_and_status() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (election, other_election) = (
        f.election_with_revotes(a, 0).await,
        f.election_with_revotes(a, 0).await,
    );
    let area = f.area(a).await;
    let valid_vote = f.vote(a, election, area, "voter", "valid").await;
    let pending_vote = f.vote(a, election, area, "voter", "in-progress").await;
    f.vote(a, election, area, "voter", "discarded").await;
    f.vote(a, election, area, "other-voter", "valid").await;
    f.vote(a, other_election, area, "voter", "valid").await;
    let votes = |statuses: &'static [CastVoteStatus]| {
        let tx = &tx;
        async move {
            sorted(
                cast_vote::get_cast_votes(tx, &a.tenant, &a.event, &election, "voter", statuses)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|vote| vote.id)
                    .collect(),
            )
        }
    };

    assert_eq!(
        votes(&[CastVoteStatus::Valid, CastVoteStatus::InProgress]).await,
        strings(&[valid_vote, pending_vote])
    );
    assert_eq!(
        votes(&[CastVoteStatus::InProgress]).await,
        strings(&[pending_vote])
    );
    assert!(votes(&[]).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_voter_cast_vote_states_for_event_groups_the_active_votes_by_voter() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election_with_revotes(a, 0).await;
    let area = f.area(a).await;
    let sibling_election = f.election(sibling).await;
    let sibling_area = f.area(sibling).await;
    f.vote(a, election, area, "both", "valid").await;
    f.vote(a, election, area, "both", "in-progress").await;
    f.vote(a, election, area, "pending", "in-progress").await;
    f.vote(a, election, area, "voted", "valid").await;
    f.vote(a, election, area, "voted", "discarded").await;
    f.vote(a, election, area, "discarded", "discarded").await;
    f.vote(
        sibling,
        sibling_election,
        sibling_area,
        "elsewhere",
        "valid",
    )
    .await;
    tx.execute(
        "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, status)
         VALUES ($1, $2, $3, $4, 'valid')",
        &[&a.tenant, &a.event, &election, &area],
    )
    .await
    .unwrap();

    let states =
        cast_vote::get_voter_cast_vote_states_for_event(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap();

    let mut summary: Vec<(String, bool, bool)> = states
        .into_iter()
        .map(|(voter, state)| (voter, state.has_unresolved_vote, state.has_valid_vote))
        .collect();
    summary.sort();
    assert_eq!(
        summary,
        vec![
            ("both".to_string(), true, true),
            ("pending".to_string(), true, false),
            ("voted".to_string(), false, true),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_cast_votes_by_election_id_returns_the_valid_votes_of_the_election() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (election, other_election) = (
        f.election_with_revotes(a, 0).await,
        f.election_with_revotes(a, 0).await,
    );
    let area = f.area(a).await;
    f.election_as(sibling, election, Some(0)).await;
    f.area_as(sibling, area).await;
    let first = f.vote(a, election, area, "first", "valid").await;
    let second = f.vote(a, election, area, "second", "valid").await;
    f.vote(a, election, area, "third", "in-progress").await;
    f.vote(a, election, area, "fourth", "discarded").await;
    f.vote(a, other_election, area, "first", "valid").await;
    f.vote(sibling, election, area, "first", "valid").await;

    let votes = cast_vote::get_cast_votes_by_election_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &election.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(
        sorted(votes.iter().map(|vote| vote.id.clone()).collect()),
        strings(&[first, second])
    );
    assert!(votes.iter().all(|vote| vote.status == CastVoteStatus::Valid
        && vote.content.as_deref() == Some("ciphertext")));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn cast_vote_lookups_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.id();
    let (tenant, event) = (a.tenant_id(), a.event_id());

    for (tenant, event) in [(BAD_UUID, event.as_str()), (tenant.as_str(), BAD_UUID)] {
        assert_invalid_uuid(
            cast_vote::compare_and_set_cast_vote_status(
                &tx,
                tenant,
                event,
                &id,
                CastVoteStatus::InProgress,
                CastVoteStatus::Valid,
            )
            .await,
        );
        assert_invalid_uuid(cast_vote::get_cast_vote_by_id(&tx, tenant, event, &id).await);
        assert_invalid_uuid(cast_vote::has_valid_cast_vote(&tx, tenant, event, "voter").await);
        assert_invalid_uuid(
            cast_vote::get_voter_cast_vote_states_for_event(&tx, tenant, event)
                .await
                .map(|states| states.len()),
        );
        assert_invalid_uuid(
            cast_vote::get_cast_votes_by_election_id(&tx, tenant, event, &id.to_string()).await,
        );
    }
    assert_invalid_uuid(
        cast_vote::get_cast_votes_by_election_id(&tx, &a.tenant_id(), &a.event_id(), BAD_UUID)
            .await,
    );
    tx.rollback().await.unwrap();
}
