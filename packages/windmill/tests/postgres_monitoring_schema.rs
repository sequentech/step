// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tables behind the configurable monitoring dashboards: what they
//! refuse, and that they go with their election event.

#[path = "support/schema.rs"]
mod schema;

use sequent_core::monitoring::config::ConfigKind;
use sequent_core::monitoring::scope::election_set_key;
use sequent_core::monitoring::sources::DataSourceId;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::future::Future;
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio_postgres::types::ToSql;
use tokio_postgres::{IsolationLevel, Transaction};
use uuid::Uuid;
use windmill::postgres::election_event::delete_election_event;

const TABLES: [&str; 12] = [
    "monitoring_event",
    "monitoring_config",
    "monitoring_config_head",
    "monitoring_election_set",
    "monitoring_snapshot_run",
    "monitoring_snapshot_state",
    "monitoring_snapshot_source",
    "monitoring_snapshot_payload",
    "monitoring_snapshot_figure",
    "monitoring_voter",
    "monitoring_login_counter",
    "monitoring_login_counter_receipt",
];

#[derive(Clone, Copy)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
}

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

fn sha256(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// A payload digest, as the snapshot tables store it.
fn digest(text: &str) -> Vec<u8> {
    Sha256::digest(text.as_bytes()).to_vec()
}

fn set_key(ids: &[Uuid]) -> String {
    election_set_key(ids.iter().map(Uuid::to_string)).unwrap()
}

/// A tenant with an election event on the configurable dashboard.
async fn scope(tx: &Transaction<'_>, seed: u32) -> Scope {
    let s = bare_scope(tx, seed).await;
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
         VALUES ($1, $2)",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    s
}

/// A tenant with an election event and no monitoring row.
async fn bare_scope(tx: &Transaction<'_>, seed: u32) -> Scope {
    let tenant = id(seed, 1);
    let event = id(seed, 2);
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    Scope { tenant, event }
}

async fn election(tx: &Transaction<'_>, s: Scope, id: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&id, &s.tenant, &s.event],
    )
    .await
    .unwrap();
}

async fn election_set(tx: &Transaction<'_>, s: Scope, ids: &[Uuid]) -> String {
    let key = set_key(ids);
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_election_set
             (tenant_id, election_event_id, election_set_key, election_ids)
         VALUES ($1, $2, $3, $4)",
        &[&s.tenant, &s.event, &key, &ids],
    )
    .await
    .unwrap();
    key
}

async fn payload(tx: &Transaction<'_>, s: Scope, text: &str) -> Vec<u8> {
    let sha = digest(text);
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_snapshot_payload
             (tenant_id, election_event_id, sha256, payload)
         VALUES ($1, $2, $3, '{}')",
        &[&s.tenant, &s.event, &sha],
    )
    .await
    .unwrap();
    sha
}

/// A finished snapshot run of the event, `COMPLETE` or `FAILED`: its
/// revision.
async fn finished_run(tx: &Transaction<'_>, s: Scope, status: &str) -> i64 {
    tx.query_one(
        "INSERT INTO sequent_backend.monitoring_snapshot_run
             (tenant_id, election_event_id, status, finished_at, as_of, settings_revision,
              config_generation, error)
         SELECT $1, $2, $3, now(), complete.as_of, complete.settings, complete.generation,
                CASE WHEN $3 = 'FAILED' THEN 'the pass failed' END
         FROM (SELECT CASE WHEN $3 = 'COMPLETE' THEN now() END AS as_of,
                      CASE WHEN $3 = 'COMPLETE' THEN 1 END AS settings,
                      CASE WHEN $3 = 'COMPLETE' THEN 0::bigint END AS generation) AS complete
         RETURNING revision",
        &[&s.tenant, &s.event, &status],
    )
    .await
    .unwrap()
    .get(0)
}

/// A run recorded as RUNNING, as the job starts a pass: its revision.
async fn start_run(tx: &Transaction<'_>, s: Scope) -> i64 {
    tx.query_one(
        "INSERT INTO sequent_backend.monitoring_snapshot_run
             (tenant_id, election_event_id, status)
         VALUES ($1, $2, 'RUNNING') RETURNING revision",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap()
    .get(0)
}

/// Ends a running pass, `COMPLETE` or `FAILED`.
async fn end_run(
    tx: &Transaction<'_>,
    s: Scope,
    revision: i64,
    status: &str,
) -> Result<u64, tokio_postgres::Error> {
    tx.execute(
        "UPDATE sequent_backend.monitoring_snapshot_run
         SET status = $4, finished_at = now(),
             as_of = CASE WHEN $4 = 'COMPLETE' THEN now() END,
             settings_revision = CASE WHEN $4 = 'COMPLETE' THEN 1 END,
             config_generation = CASE WHEN $4 = 'COMPLETE' THEN 0 END,
             error = CASE WHEN $4 = 'FAILED' THEN 'the pass failed' END
         WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
        &[&s.tenant, &s.event, &revision, &status],
    )
    .await
}

/// Closes a scope's showing row at the running pass `at`.
async fn close_figure(
    tx: &Transaction<'_>,
    s: Scope,
    scope_key: &str,
    at: i64,
) -> Result<u64, tokio_postgres::Error> {
    tx.execute(
        "UPDATE sequent_backend.monitoring_snapshot_figure SET to_revision = $4
         WHERE tenant_id = $1 AND election_event_id = $2 AND scope_key = $3
           AND to_revision IS NULL",
        &[&s.tenant, &s.event, &scope_key, &at],
    )
    .await
}

/// Runs `sql` in a savepoint: the rows it touched, or the constraint that
/// refused it. Deferred constraints are checked before the savepoint ends,
/// as the snapshot job's pruning does. Anything other than an integrity
/// constraint (SQLSTATE class 23) fails the test, so a mistyped statement
/// cannot pass as a refusal.
async fn attempt(
    tx: &mut Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64, String> {
    let savepoint = tx.savepoint("attempt").await.unwrap();
    let result = savepoint.execute(sql, params).await.map_err(refused);
    settle(savepoint, result).await
}

/// Checks the deferred constraints of what `savepoint` did so far, then
/// keeps it, or rolls it back on a refusal.
async fn settle(savepoint: Transaction<'_>, result: Result<u64, String>) -> Result<u64, String> {
    let result = match result {
        Ok(rows) => savepoint
            .batch_execute("SET CONSTRAINTS ALL IMMEDIATE")
            .await
            .map(|()| rows)
            .map_err(refused),
        Err(error) => Err(error),
    };
    match result {
        Ok(rows) => {
            savepoint
                .batch_execute("SET CONSTRAINTS ALL DEFERRED")
                .await
                .unwrap();
            savepoint.commit().await.unwrap();
            Ok(rows)
        }
        Err(error) => {
            savepoint.rollback().await.unwrap();
            Err(error)
        }
    }
}

/// The constraint an integrity error names; panics on any other error.
fn refusal(error: &tokio_postgres::Error) -> String {
    let db = error
        .as_db_error()
        .unwrap_or_else(|| panic!("not a database error: {error:?}"));
    assert!(
        db.code().code().starts_with("23"),
        "not a constraint refusal: {db:?}"
    );
    db.constraint()
        .unwrap_or_else(|| panic!("a refusal without its constraint: {db:?}"))
        .to_owned()
}

fn refused(error: tokio_postgres::Error) -> String {
    refusal(&error)
}

fn refused_by(result: Result<u64, String>, constraint: &str, why: &str) {
    assert_eq!(result, Err(constraint.to_owned()), "{why}");
}

async fn count(tx: &Transaction<'_>, sql: &str, params: &[&(dyn ToSql + Sync)]) -> i64 {
    tx.query_one(sql, params).await.unwrap().get(0)
}

async fn row_counts(tx: &Transaction<'_>, event: Uuid) -> Vec<(&'static str, i64)> {
    let mut counts = Vec::new();
    for table in TABLES {
        let sql =
            format!("SELECT count(*) FROM sequent_backend.{table} WHERE election_event_id = $1");
        counts.push((table, count(tx, &sql, &[&event]).await));
    }
    counts
}

/// The event's configuration generation.
async fn generation(tx: &Transaction<'_>, s: Scope) -> i64 {
    count(
        tx,
        "SELECT config_generation FROM sequent_backend.monitoring_event
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&s.tenant, &s.event],
    )
    .await
}

/// A figure row as a snapshot pass opens it, still showing.
async fn open_figure(
    tx: &Transaction<'_>,
    s: Scope,
    (set, scope_key): (&str, &str),
    from: i64,
    sha: &[u8],
) -> Result<u64, tokio_postgres::Error> {
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_snapshot_figure
             (tenant_id, election_event_id, source, election_set_key, scope_key,
              from_revision, payload_sha256)
         VALUES ($1, $2, 'voter_turnout', $3, $4, $5, $6)",
        &[&s.tenant, &s.event, &set, &scope_key, &from, &sha],
    )
    .await
}

// A save, by the protocol monitoring_config_head describes.
const BUMP: &str =
    "UPDATE sequent_backend.monitoring_event SET config_generation = config_generation + 1
     WHERE tenant_id = $1 AND election_event_id = $2
     RETURNING config_generation";
const ADVANCE_HEAD: &str =
    "UPDATE sequent_backend.monitoring_config_head SET revision = revision + 1
     WHERE tenant_id = $1 AND election_event_id = $2 AND kind = $3 AND key = $4
       AND revision = $5";
const CREATE_HEAD: &str = "INSERT INTO sequent_backend.monitoring_config_head
         (tenant_id, election_event_id, kind, key, revision)
     VALUES ($1, $2, $3, $4, 1)
     ON CONFLICT DO NOTHING";
const REVISION: &str = "INSERT INTO sequent_backend.monitoring_config
        (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
         origin, preset_id, preset_version, author_id, config_generation)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)";

/// What a save reports when the head is not where it expected.
const CONFLICT: &str = "conflict";
/// What a save reports for an event with no monitoring row.
const NOT_CONFIGURED: &str = "not configured";

/// Moves the head of a document on to `revision`: the rows it moved, none
/// on a conflict.
async fn move_head(
    tx: &Transaction<'_>,
    s: Scope,
    kind: &str,
    key: &str,
    revision: i32,
) -> Result<u64, tokio_postgres::Error> {
    if revision == 1 {
        tx.execute(CREATE_HEAD, &[&s.tenant, &s.event, &kind, &key])
            .await
    } else {
        tx.execute(
            ADVANCE_HEAD,
            &[&s.tenant, &s.event, &kind, &key, &(revision - 1)],
        )
        .await
    }
}

/// One saved change, written by the protocol: the event's generation
/// raised once, then each document's head moved and its revision written.
async fn save_steps(
    tx: &Transaction<'_>,
    s: Scope,
    revisions: &[Revision<'_>],
) -> Result<u64, String> {
    let generation: i64 = tx
        .query_opt(BUMP, &[&s.tenant, &s.event])
        .await
        .map_err(refused)?
        .ok_or_else(|| NOT_CONFIGURED.to_owned())?
        .get(0);
    let mut rows = 0;
    for revision in revisions {
        let moved = move_head(tx, s, revision.kind, revision.key, revision.revision)
            .await
            .map_err(refused)?;
        if moved == 0 {
            return Err(CONFLICT.to_owned());
        }
        rows += tx
            .execute(REVISION, &revision.params(&s, &generation))
            .await
            .map_err(refused)?;
    }
    Ok(rows)
}

/// A saved change in a savepoint, as [`attempt`] runs a statement.
async fn save_all(
    tx: &mut Transaction<'_>,
    s: Scope,
    revisions: &[Revision<'_>],
) -> Result<u64, String> {
    let savepoint = tx.savepoint("save").await.unwrap();
    let result = save_steps(&savepoint, s, revisions).await;
    settle(savepoint, result).await
}

/// Waits for `future`, failing the test if it had to wait on a lock.
async fn without_waiting<F: Future>(future: F, what: &str) -> F::Output {
    tokio::time::timeout(Duration::from_secs(2), future)
        .await
        .unwrap_or_else(|_| panic!("{what} waited for the other transaction"))
}

/// A configuration revision, as a save or a reset would write it.
struct Revision<'a> {
    kind: &'a str,
    key: &'a str,
    revision: i32,
    change: &'a str,
    yaml: Option<&'a str>,
    sha256: Option<String>,
    origin: &'a str,
    preset_id: Option<&'a str>,
    preset_version: Option<i32>,
    author: &'a str,
}

impl<'a> Revision<'a> {
    fn edit(key: &'a str, revision: i32, yaml: &'a str) -> Self {
        Revision {
            kind: "widget",
            key,
            revision,
            change: "UPSERT",
            yaml: Some(yaml),
            sha256: Some(sha256(yaml)),
            origin: "EDITOR",
            preset_id: None,
            preset_version: None,
            author: "admin-1",
        }
    }

    fn removal(key: &'a str, revision: i32) -> Self {
        Revision {
            change: "DELETE",
            yaml: None,
            sha256: None,
            ..Revision::edit(key, revision, "-")
        }
    }

    fn reset(key: &'a str, revision: i32, yaml: &'a str) -> Self {
        Revision {
            origin: "PRESET",
            preset_id: Some("comelec"),
            preset_version: Some(1),
            ..Revision::edit(key, revision, yaml)
        }
    }

    /// Saves the revision as a change of its own.
    async fn save(&self, tx: &mut Transaction<'_>, s: Scope) -> Result<u64, String> {
        save_all(tx, s, std::slice::from_ref(self)).await
    }

    /// Writes the revision alone, of `generation`, moving no head.
    async fn write(
        &self,
        tx: &mut Transaction<'_>,
        s: Scope,
        generation: i64,
    ) -> Result<u64, String> {
        attempt(tx, REVISION, &self.params(&s, &generation)).await
    }

    fn params<'p>(&'p self, s: &'p Scope, generation: &'p i64) -> [&'p (dyn ToSql + Sync); 13] {
        [
            &s.tenant,
            &s.event,
            &self.kind,
            &self.key,
            &self.revision,
            &self.change,
            &self.yaml,
            &self.sha256,
            &self.origin,
            &self.preset_id,
            &self.preset_version,
            &self.author,
            generation,
        ]
    }
}

#[tokio::test]
async fn an_event_is_on_the_standard_dashboard_until_it_is_configured() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let row = tx
        .query_one(
            "SELECT dashboard_mode, config_generation
             FROM sequent_backend.monitoring_event
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "LEGACY");
    assert_eq!(row.get::<_, i64>(1), 0);

    let mode = "UPDATE sequent_backend.monitoring_event SET dashboard_mode = $3
                WHERE tenant_id = $1 AND election_event_id = $2";
    assert_eq!(
        attempt(&mut tx, mode, &[&s.tenant, &s.event, &"CONFIGURED"]).await,
        Ok(1)
    );
    refused_by(
        attempt(&mut tx, mode, &[&s.tenant, &s.event, &"SOMETIMES"]).await,
        "monitoring_event_dashboard_mode_check",
        "an unknown mode",
    );
    let preset = "UPDATE sequent_backend.monitoring_event SET preset_id = $3, preset_version = $4
                  WHERE tenant_id = $1 AND election_event_id = $2";
    let none: Option<i32> = None;
    refused_by(
        attempt(&mut tx, preset, &[&s.tenant, &s.event, &"comelec", &none]).await,
        "monitoring_event_preset_named_with_version",
        "a preset without its version",
    );
    assert_eq!(
        attempt(
            &mut tx,
            preset,
            &[&s.tenant, &s.event, &"comelec", &Some(1)]
        )
        .await,
        Ok(1)
    );
    let raise = "UPDATE sequent_backend.monitoring_event SET config_generation = $3
                 WHERE tenant_id = $1 AND election_event_id = $2";
    for (to, why) in [
        (-1_i64, "a generation below the first"),
        (2, "a generation skipped"),
    ] {
        refused_by(
            attempt(&mut tx, raise, &[&s.tenant, &s.event, &to]).await,
            "monitoring_event_generation_moves_by_one",
            why,
        );
    }
    assert_eq!(
        attempt(&mut tx, raise, &[&s.tenant, &s.event, &1_i64]).await,
        Ok(1)
    );
    refused_by(
        attempt(&mut tx, raise, &[&s.tenant, &s.event, &0_i64]).await,
        "monitoring_event_generation_moves_by_one",
        "a generation taken back",
    );
    // Which transaction raised it is the trigger's to say.
    assert_eq!(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.monitoring_event SET config_generation_xact = '7'
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event],
        )
        .await,
        Ok(1)
    );
    let raised_here = count(
        &tx,
        "SELECT count(*) FROM sequent_backend.monitoring_event
         WHERE tenant_id = $1 AND election_event_id = $2
           AND config_generation_xact = pg_current_xact_id()",
        &[&s.tenant, &s.event],
    )
    .await;
    assert_eq!(raised_here, 1);
    let fresh = bare_scope(&tx, line!()).await;
    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_event
                 (tenant_id, election_event_id, config_generation)
             VALUES ($1, $2, 3)",
            &[&fresh.tenant, &fresh.event],
        )
        .await,
        "monitoring_event_generation_moves_by_one",
        "a new event past the first generation",
    );
    refused_by(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.monitoring_event SET election_event_id = $3
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event, &fresh.event],
        )
        .await,
        "monitoring_event_is_kept",
        "a monitoring history moved to another event",
    );
    refused_by(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_event
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event],
        )
        .await,
        "monitoring_event_is_kept",
        "back to the standard dashboard by deleting the history",
    );

    tx.execute(
        "UPDATE sequent_backend.monitoring_event SET updated_at = '2000-01-01T00:00:00Z'
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    let touched: bool = tx
        .query_one(
            "UPDATE sequent_backend.monitoring_event SET dashboard_mode = 'LEGACY'
             WHERE tenant_id = $1 AND election_event_id = $2
             RETURNING updated_at = now()",
            &[&s.tenant, &s.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(touched, "updated_at follows every change");
    let state = count(
        &tx,
        "SELECT count(*) FROM sequent_backend.monitoring_snapshot_state WHERE election_event_id = $1",
        &[&s.event],
    )
    .await;
    assert_eq!(state, 0, "nothing is shown before the snapshot job runs");
}

#[tokio::test]
async fn a_monitoring_row_belongs_to_an_event_of_its_own_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let mine = scope(&tx, line!()).await;
    let other = bare_scope(&tx, line!()).await;
    let theirs = id(line!(), 3);
    election(&tx, other, theirs).await;

    let cases: [(&str, &[&(dyn ToSql + Sync)], &str); 4] = [
        (
            "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
             VALUES ($1, $2)",
            &[&mine.tenant, &other.event],
            "monitoring_event_of_its_tenant",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_login_counter
                 (tenant_id, election_event_id, bucket_start, event_type, registration, attempts)
             VALUES ($1, $2, '2026-05-04T10:15:00Z', 'LOGIN', 'REGISTERED', 1)",
            &[&mine.tenant, &other.event],
            "monitoring_login_counter_of_its_tenant",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_login_counter_receipt
                 (delivery_id, tenant_id, election_event_id)
             VALUES (repeat('a', 64), $1, $2)",
            &[&mine.tenant, &other.event],
            "monitoring_login_counter_receipt_of_its_tenant",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_voter
                 (tenant_id, election_event_id, election_id, voter_id, attributes_hash, settings_revision)
             VALUES ($1, $2, $3, 'v1', 'h', 1)",
            &[&mine.tenant, &mine.event, &theirs],
            "monitoring_voter_of_an_election_of_the_event",
        ),
    ];
    for (statement, params, constraint) in cases {
        refused_by(
            attempt(&mut tx, statement, params).await,
            constraint,
            statement,
        );
    }
}

#[tokio::test]
async fn configuration_and_figures_need_their_event_configured() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = bare_scope(&tx, line!()).await;
    let post = id(line!(), 3);
    election(&tx, s, post).await;
    let (key, figures) = (set_key(&[post]), digest("figures"));

    assert_eq!(
        Revision::edit("w", 1, "id: w\n").save(&mut tx, s).await,
        Err(NOT_CONFIGURED.to_owned()),
        "a save finds no event to raise the generation of"
    );
    refused_by(
        Revision::edit("w", 1, "id: w\n").write(&mut tx, s, 1).await,
        "monitoring_config_of_its_event",
        "a revision",
    );
    let cases: [(&str, &[&(dyn ToSql + Sync)], &str); 9] = [
        (
            "INSERT INTO sequent_backend.monitoring_config_head
                 (tenant_id, election_event_id, kind, key, revision)
             VALUES ($1, $2, 'widget', 'w', 1)",
            &[&s.tenant, &s.event],
            "monitoring_config_head_of_its_event",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_election_set
                 (tenant_id, election_event_id, election_set_key, election_ids)
             VALUES ($1, $2, $3, ARRAY[$4::uuid])",
            &[&s.tenant, &s.event, &key, &post],
            "monitoring_election_set_of_its_event",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_run
                 (tenant_id, election_event_id, status)
             VALUES ($1, $2, 'RUNNING')",
            &[&s.tenant, &s.event],
            "monitoring_snapshot_run_of_its_event",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_state
                 (tenant_id, election_event_id)
             VALUES ($1, $2)",
            &[&s.tenant, &s.event],
            "monitoring_snapshot_state_of_its_event",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_source
                 (tenant_id, election_event_id, revision, source, election_set_key, producer_status)
             VALUES ($1, $2, 1, 'voter_turnout', $3, 'CONNECTED')",
            &[&s.tenant, &s.event, &key],
            "monitoring_snapshot_source_of_its_run",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_payload
                 (tenant_id, election_event_id, sha256, payload)
             VALUES ($1, $2, $3, '{}')",
            &[&s.tenant, &s.event, &figures],
            "monitoring_snapshot_payload_of_its_event",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_figure
                 (tenant_id, election_event_id, source, election_set_key, scope_key,
                  from_revision, payload_sha256)
             VALUES ($1, $2, 'voter_turnout', $3, 'event', 1, $4)",
            &[&s.tenant, &s.event, &key, &figures],
            "monitoring_snapshot_figure_of_its_event",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_voter
                 (tenant_id, election_event_id, election_id, voter_id, attributes_hash, settings_revision)
             VALUES ($1, $2, $3, 'v1', 'h', 1)",
            &[&s.tenant, &s.event, &post],
            "monitoring_voter_of_its_event",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_run
                 (tenant_id, election_event_id, revision, status)
             VALUES ($1, $2, 0, 'RUNNING')",
            &[&s.tenant, &s.event],
            "monitoring_snapshot_run_revision_check",
        ),
    ];
    for (statement, params, constraint) in cases {
        refused_by(
            attempt(&mut tx, statement, params).await,
            constraint,
            statement,
        );
    }
}

#[tokio::test]
async fn a_revision_is_a_change_of_a_known_kind_and_origin() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;

    for kind in ConfigKind::iter() {
        let kind = kind.to_string();
        let revision = Revision {
            kind: &kind,
            ..Revision::edit("k", 1, "id: k\n")
        };
        assert_eq!(revision.save(&mut tx, s).await, Ok(1), "{kind}");
    }
    assert_eq!(
        Revision::reset("from-preset", 1, "id: from-preset\n")
            .save(&mut tx, s)
            .await,
        Ok(1)
    );
    assert_eq!(Revision::removal("gone", 1).save(&mut tx, s).await, Ok(1));
    assert_eq!(
        Revision::edit("unicode", 1, "title: Participación · 投票\n")
            .save(&mut tx, s)
            .await,
        Ok(1),
        "the digest is of the UTF-8 text"
    );

    let yaml = "id: x\n";
    let cases = [
        (
            Revision {
                kind: "report",
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_kind_check",
            "an unknown kind",
        ),
        (
            Revision {
                change: "PATCH",
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_change_check",
            "an unknown change",
        ),
        (
            Revision {
                origin: "IMPORT",
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_origin_check",
            "an unknown origin",
        ),
        (
            Revision {
                yaml: None,
                sha256: None,
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_document_follows_change",
            "an upsert without its document",
        ),
        (
            Revision {
                change: "DELETE",
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_document_follows_change",
            "a removal with a document",
        ),
        (
            Revision {
                sha256: Some(sha256("id: y\n")),
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_digest_is_of_document",
            "a digest of another document",
        ),
        (
            Revision {
                sha256: None,
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_digest_is_of_document",
            "a document without its digest",
        ),
        (
            Revision {
                sha256: Some(sha256("")),
                ..Revision::removal("x", 1)
            },
            "monitoring_config_digest_is_of_document",
            "a digest without a document",
        ),
        (
            Revision {
                sha256: Some(sha256("")),
                ..Revision::edit("x", 1, "")
            },
            "monitoring_config_yaml_check",
            "an empty document",
        ),
        (
            Revision {
                origin: "PRESET",
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_preset_follows_origin",
            "a preset revision that names no preset",
        ),
        (
            Revision {
                origin: "EDITOR",
                ..Revision::reset("x", 1, yaml)
            },
            "monitoring_config_preset_follows_origin",
            "an editor revision that names a preset",
        ),
        (
            Revision {
                preset_version: None,
                ..Revision::reset("x", 1, yaml)
            },
            "monitoring_config_preset_named_with_version",
            "a preset without its version",
        ),
        (
            Revision {
                author: "",
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_author_is_named",
            "an edit by a blank author",
        ),
        (
            Revision {
                author: "",
                ..Revision::reset("x", 1, yaml)
            },
            "monitoring_config_author_is_named",
            "a reset by a blank author",
        ),
    ];
    for (revision, constraint, why) in cases {
        refused_by(revision.save(&mut tx, s).await, constraint, why);
    }
    for key in ["Turnout", "-lead", "has space", "", &"a".repeat(65)] {
        refused_by(
            Revision::edit(key, 1, yaml).save(&mut tx, s).await,
            "monitoring_config_key_check",
            key,
        );
    }
    assert_eq!(
        Revision::edit("k", 1, yaml).save(&mut tx, s).await,
        Err(CONFLICT.to_owned()),
        "a second first revision finds the head taken"
    );

    // Revisions written past the protocol.
    let current = generation(&tx, s).await;
    assert_eq!(current, 7, "one generation per saved change");
    for (revision, generation, constraint, why) in [
        (
            Revision::edit("x", 0, yaml),
            current,
            "monitoring_config_revision_check",
            "revisions count from 1",
        ),
        (
            Revision::edit("k", 1, yaml),
            current,
            "monitoring_config_pkey",
            "a revision number used twice",
        ),
        // A revision nobody made the head would take the number of the next
        // save.
        (
            Revision::edit("k", 2, yaml),
            current,
            "monitoring_config_revision_is_the_head",
            "a revision past the head",
        ),
        (
            Revision::edit("new", 1, yaml),
            current,
            "monitoring_config_revision_is_the_head",
            "a new document without a head",
        ),
        (
            Revision::edit("k", 2, yaml),
            current + 1,
            "monitoring_config_generation_is_current",
            "a generation not yet reached",
        ),
        (
            Revision::edit("k", 2, yaml),
            current - 1,
            "monitoring_config_generation_is_current",
            "an earlier generation",
        ),
    ] {
        refused_by(
            revision.write(&mut tx, s, generation).await,
            constraint,
            why,
        );
    }
    let unsaved = scope(&tx, line!()).await;
    refused_by(
        Revision::edit("k", 1, yaml)
            .write(&mut tx, unsaved, 0)
            .await,
        "monitoring_config_generation_is_current",
        "a revision of the generation before any save",
    );
}

#[tokio::test]
async fn a_revision_is_never_changed_or_removed_but_with_its_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    Revision::edit("w", 1, "id: w\n")
        .save(&mut tx, s)
        .await
        .unwrap();
    for (statement, why) in [
        (
            "UPDATE sequent_backend.monitoring_config SET author_name = 'x' WHERE election_event_id = $1",
            "a logged revision rewritten",
        ),
        (
            "DELETE FROM sequent_backend.monitoring_config WHERE election_event_id = $1",
            "a revision removed, the live one with it",
        ),
    ] {
        refused_by(
            attempt(&mut tx, statement, &[&s.event]).await,
            "monitoring_config_is_append_only",
            why,
        );
    }
    // Nor from inside another table's trigger.
    tx.batch_execute(&format!(
        "CREATE TEMP TABLE poke (x int);
         CREATE FUNCTION pg_temp.poke() RETURNS trigger AS $$
         BEGIN
             DELETE FROM sequent_backend.monitoring_config WHERE election_event_id = '{}';
             RETURN NULL;
         END;
         $$ LANGUAGE plpgsql;
         CREATE TRIGGER poke AFTER INSERT ON pg_temp.poke
             FOR EACH ROW EXECUTE FUNCTION pg_temp.poke();",
        s.event
    ))
    .await
    .unwrap();
    refused_by(
        attempt(&mut tx, "INSERT INTO pg_temp.poke VALUES (1)", &[]).await,
        "monitoring_config_is_append_only",
        "a deletion from a trigger",
    );
    // TRUNCATE takes the whole table at once; its guard is a statement
    // trigger, checked here rather than by locking the shared table.
    let guarded = count(
        &tx,
        "SELECT count(*) FROM pg_trigger
         WHERE tgrelid = 'sequent_backend.monitoring_config'::regclass
           AND tgname = 'monitoring_config_is_not_truncated'
           AND tgtype & 32 <> 0 AND tgtype & 2 <> 0 AND tgenabled = 'O'",
        &[],
    )
    .await;
    assert_eq!(guarded, 1, "TRUNCATE is refused before it runs");
    let heads = count(
        &tx,
        "SELECT count(*) FROM sequent_backend.monitoring_config_head WHERE election_event_id = $1",
        &[&s.event],
    )
    .await;
    assert_eq!(heads, 1);

    Revision::edit("w", 2, "id: w\ntitle: T\n")
        .save(&mut tx, s)
        .await
        .unwrap();
    let moved = count(
        &tx,
        "SELECT count(*) FROM sequent_backend.monitoring_config_head
         WHERE election_event_id = $1 AND revision = 2",
        &[&s.event],
    )
    .await;
    assert_eq!(moved, 1);

    // A head only moves on by one, and each move is a revision of its own.
    for (statement, constraint, why) in [
        (
            "INSERT INTO sequent_backend.monitoring_config_head
                 (tenant_id, election_event_id, kind, key, revision)
             VALUES ($1, $2, 'theme', 'w', 1)",
            "monitoring_config_head_names_a_revision",
            "a head for a document with no revisions",
        ),
        (
            "INSERT INTO sequent_backend.monitoring_config_head
                 (tenant_id, election_event_id, kind, key, revision)
             VALUES ($1, $2, 'theme', 'w', 2)",
            "monitoring_config_head_moves_by_one",
            "a new document's head past its first revision",
        ),
        (
            "UPDATE sequent_backend.monitoring_config_head SET revision = revision + 2
             WHERE tenant_id = $1 AND election_event_id = $2",
            "monitoring_config_head_moves_by_one",
            "a head skipping a revision",
        ),
        (
            "UPDATE sequent_backend.monitoring_config_head SET revision = revision - 1
             WHERE tenant_id = $1 AND election_event_id = $2",
            "monitoring_config_head_moves_by_one",
            "a head moved back",
        ),
        (
            "UPDATE sequent_backend.monitoring_config_head SET revision = revision
             WHERE tenant_id = $1 AND election_event_id = $2",
            "monitoring_config_head_moves_by_one",
            "a head rewritten where it stands",
        ),
        (
            "UPDATE sequent_backend.monitoring_config_head
             SET key = 'v', revision = revision + 1
             WHERE tenant_id = $1 AND election_event_id = $2",
            "monitoring_config_head_moves_by_one",
            "a head moved to another document",
        ),
        (
            "UPDATE sequent_backend.monitoring_config_head SET revision = revision + 1
             WHERE tenant_id = $1 AND election_event_id = $2",
            "monitoring_config_head_names_a_revision",
            "a head moved on to a revision nobody wrote",
        ),
    ] {
        refused_by(
            attempt(&mut tx, statement, &[&s.tenant, &s.event]).await,
            constraint,
            why,
        );
    }
    refused_by(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_config_head
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event],
        )
        .await,
        "monitoring_config_head_is_kept",
        "a document's head removed instead of a DELETE revision saved",
    );
    refused_by(
        save_all(
            &mut tx,
            s,
            &[
                Revision::edit("w", 3, "id: w\ntitle: 3\n"),
                Revision::edit("w", 4, "id: w\ntitle: 4\n"),
            ],
        )
        .await,
        "monitoring_config_revision_is_the_head",
        "one change moving a document twice: its first revision never took effect",
    );
}

#[tokio::test]
async fn of_two_saves_from_one_revision_the_second_moves_nothing() {
    let pool = schema::pool().await;
    let mut first = pool.get().await.unwrap();
    let mut second = pool.get().await.unwrap();
    let s = {
        let mut setup = first.transaction().await.unwrap();
        let s = scope(&setup, line!()).await;
        Revision::edit("w", 1, "id: w\n")
            .save(&mut setup, s)
            .await
            .unwrap();
        setup.commit().await.unwrap();
        s
    };
    let event: [&(dyn ToSql + Sync); 2] = [&s.tenant, &s.event];

    for (key, revision) in [("w", 2_i32), ("new", 1)] {
        let a = first.transaction().await.unwrap();
        let b = second.transaction().await.unwrap();
        let raised_to: i64 = a.query_one(BUMP, &event).await.unwrap().get(0);
        assert_eq!(move_head(&a, s, "widget", key, revision).await.unwrap(), 1);
        let yaml = format!("id: {key}\ntitle: A\n");
        let written = Revision::edit(key, revision, &yaml);
        a.execute(REVISION, &written.params(&s, &raised_to))
            .await
            .unwrap();
        let moved = {
            let waiting = b.query_one(BUMP, &event);
            tokio::pin!(waiting);
            assert!(
                tokio::time::timeout(Duration::from_millis(300), &mut waiting)
                    .await
                    .is_err(),
                "the second save waits for the first"
            );
            a.commit().await.unwrap();
            let raised: i64 = waiting.await.unwrap().get(0);
            assert_eq!(raised, raised_to + 1, "and raises the generation past it");
            move_head(&b, s, "widget", key, revision).await.unwrap()
        };
        assert_eq!(moved, 0, "and then moves nothing: a conflict to report");
        b.rollback().await.unwrap();
    }
    // The head's updated_at is when it last moved.
    let check = first.transaction().await.unwrap();
    let moved_when_saved: bool = check
        .query_one(
            "SELECT head.updated_at = saved.created_at AND head.updated_at > created.created_at
             FROM sequent_backend.monitoring_config_head AS head
             JOIN sequent_backend.monitoring_config AS saved USING
                 (tenant_id, election_event_id, kind, key, revision)
             JOIN sequent_backend.monitoring_config AS created USING
                 (tenant_id, election_event_id, kind, key)
             WHERE head.tenant_id = $1 AND head.election_event_id = $2 AND head.key = 'w'
               AND head.revision = 2 AND created.revision = 1",
            &event,
        )
        .await
        .unwrap()
        .get(0);
    assert!(moved_when_saved);
    check.rollback().await.unwrap();

    // Under REPEATABLE READ the second save fails instead, which is why
    // saves run under READ COMMITTED.
    let a = first.transaction().await.unwrap();
    let b = second
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .start()
        .await
        .unwrap();
    b.batch_execute("SELECT 1").await.unwrap();
    let raised_to: i64 = a.query_one(BUMP, &event).await.unwrap().get(0);
    move_head(&a, s, "widget", "w", 3).await.unwrap();
    let written = Revision::edit("w", 3, "id: w\ntitle: B\n");
    a.execute(REVISION, &written.params(&s, &raised_to))
        .await
        .unwrap();
    let error = {
        let waiting = b.query_one(BUMP, &event);
        tokio::pin!(waiting);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), &mut waiting)
                .await
                .is_err()
        );
        a.commit().await.unwrap();
        waiting.await.unwrap_err()
    };
    assert_eq!(error.as_db_error().unwrap().code().code(), "40001");
    b.rollback().await.unwrap();

    // What only the end of the transaction can tell is refused at commit:
    // a head moved on to a revision nobody wrote,
    let lost = first.transaction().await.unwrap();
    lost.execute(
        "UPDATE sequent_backend.monitoring_config_head SET revision = revision + 1
         WHERE tenant_id = $1 AND election_event_id = $2 AND key = 'w'",
        &event,
    )
    .await
    .unwrap();
    let error = lost.commit().await.unwrap_err();
    assert_eq!(refusal(&error), "monitoring_config_head_names_a_revision");
    // and a revision written with no head moved to it.
    let orphan = first.transaction().await.unwrap();
    let raised_to: i64 = orphan.query_one(BUMP, &event).await.unwrap().get(0);
    let written = Revision::edit("w", 4, "id: w\ntitle: C\n");
    orphan
        .execute(REVISION, &written.params(&s, &raised_to))
        .await
        .unwrap();
    let error = orphan.commit().await.unwrap_err();
    assert_eq!(refusal(&error), "monitoring_config_revision_is_the_head");
    // A change that did not raise the generation writes no revision, even of
    // the current one: runs already counted for it would not match it.
    let unraised = first.transaction().await.unwrap();
    let current = generation(&unraised, s).await;
    assert_eq!(move_head(&unraised, s, "widget", "w", 4).await.unwrap(), 1);
    let error = unraised
        .execute(REVISION, &written.params(&s, &current))
        .await
        .unwrap_err();
    assert_eq!(refusal(&error), "monitoring_config_generation_is_current");
    unraised.rollback().await.unwrap();

    // Deleting the event removes the committed rows, deferred checks included.
    let cleanup = first.transaction().await.unwrap();
    delete_election_event(&cleanup, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
    let check = first.transaction().await.unwrap();
    for (table, rows) in row_counts(&check, s.event).await {
        assert_eq!(rows, 0, "{table}");
    }
}

#[tokio::test]
async fn the_configuration_of_a_generation_is_each_documents_latest_revision_up_to_it() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let changes: [&[Revision]; 4] = [
        &[Revision::edit("w", 1, "id: w\n")],
        &[Revision::edit("w", 2, "id: w\ntitle: T\n")],
        // A reset writes several documents in one change.
        &[
            Revision::reset("v", 1, "id: v\n"),
            Revision {
                kind: "theme",
                ..Revision::reset("t", 1, "id: t\n")
            },
        ],
        &[Revision::removal("w", 3)],
    ];
    for change in changes {
        assert_eq!(save_all(&mut tx, s, change).await, Ok(change.len() as u64));
    }
    // A mode switch raises the generation and writes no revision.
    tx.execute(
        "UPDATE sequent_backend.monitoring_event
         SET config_generation = config_generation + 1, dashboard_mode = 'CONFIGURED'
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();

    let as_of = "SELECT kind, key, revision FROM (
                     SELECT DISTINCT ON (kind, key) kind, key, revision, change
                     FROM sequent_backend.monitoring_config
                     WHERE tenant_id = $1 AND election_event_id = $2
                       AND config_generation <= $3
                     ORDER BY kind, key, config_generation DESC
                 ) AS latest
                 WHERE change = 'UPSERT'
                 ORDER BY kind, key";
    let w = |revision| ("widget", "w", revision);
    let expected: [(i64, Vec<(&str, &str, i32)>); 6] = [
        (0, vec![]),
        (1, vec![w(1)]),
        (2, vec![w(2)]),
        (3, vec![("theme", "t", 1), ("widget", "v", 1), w(2)]),
        (4, vec![("theme", "t", 1), ("widget", "v", 1)]),
        (5, vec![("theme", "t", 1), ("widget", "v", 1)]),
    ];
    for (generation, documents) in expected {
        let shown: Vec<(String, String, i32)> = tx
            .query(as_of, &[&s.tenant, &s.event, &generation])
            .await
            .unwrap()
            .iter()
            .map(|row| (row.get(0), row.get(1), row.get(2)))
            .collect();
        let documents: Vec<(String, String, i32)> = documents
            .into_iter()
            .map(|(kind, key, revision)| (kind.to_owned(), key.to_owned(), revision))
            .collect();
        assert_eq!(shown, documents, "generation {generation}");
    }
}

#[tokio::test]
async fn viewers_are_shown_only_a_complete_run_and_it_is_final() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let start = "INSERT INTO sequent_backend.monitoring_snapshot_run
                     (tenant_id, election_event_id, status)
                 VALUES ($1, $2, 'RUNNING') RETURNING revision";
    let first: i64 = tx
        .query_one(start, &[&s.tenant, &s.event])
        .await
        .unwrap()
        .get(0);
    let later = scope(&tx, line!()).await;
    let second: i64 = tx
        .query_one(start, &[&later.tenant, &later.event])
        .await
        .unwrap()
        .get(0);
    assert!(second > first, "revisions only grow, across events too");

    let run = "UPDATE sequent_backend.monitoring_snapshot_run
               SET status = $4, finished_at = $5::text::timestamptz, error = $6,
                   as_of = $7::text::timestamptz, settings_revision = $8, config_generation = $9
               WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3";
    let at = Some("2026-05-04T10:15:00Z");
    let none: Option<&str> = None;
    let (settings, generation) = (Some(1_i32), Some(0_i64));
    let (no_settings, no_generation): (Option<i32>, Option<i64>) = (None, None);
    let cases = [
        (
            "PAUSED",
            at,
            none,
            at,
            settings,
            generation,
            "monitoring_snapshot_run_status_check",
        ),
        (
            "RUNNING",
            at,
            none,
            none,
            no_settings,
            no_generation,
            "monitoring_snapshot_run_running_is_open",
        ),
        (
            "RUNNING",
            none,
            Some("x"),
            none,
            no_settings,
            no_generation,
            "monitoring_snapshot_run_running_is_open",
        ),
        (
            "COMPLETE",
            at,
            none,
            none,
            settings,
            generation,
            "monitoring_snapshot_run_complete_is_counted",
        ),
        (
            "COMPLETE",
            at,
            none,
            at,
            no_settings,
            generation,
            "monitoring_snapshot_run_complete_is_counted",
        ),
        (
            "COMPLETE",
            at,
            none,
            at,
            settings,
            no_generation,
            "monitoring_snapshot_run_complete_is_counted",
        ),
        (
            "COMPLETE",
            at,
            Some("x"),
            at,
            settings,
            generation,
            "monitoring_snapshot_run_complete_is_counted",
        ),
        (
            "FAILED",
            at,
            Some(""),
            none,
            no_settings,
            no_generation,
            "monitoring_snapshot_run_failure_says_why",
        ),
        (
            "FAILED",
            none,
            Some("x"),
            none,
            no_settings,
            no_generation,
            "monitoring_snapshot_run_failure_says_why",
        ),
    ];
    for (status, finished, error, as_of, settings, config, constraint) in cases {
        refused_by(
            attempt(
                &mut tx,
                run,
                &[
                    &s.tenant, &s.event, &first, &status, &finished, &error, &as_of, &settings,
                    &config,
                ],
            )
            .await,
            constraint,
            status,
        );
    }
    let check =
        "UPDATE sequent_backend.monitoring_snapshot_run SET checked_at = $4::text::timestamptz
                 WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3";
    refused_by(
        attempt(&mut tx, check, &[&s.tenant, &s.event, &first, &at]).await,
        "monitoring_snapshot_run_checked_when_complete",
        "a running run checked",
    );

    let state = "INSERT INTO sequent_backend.monitoring_snapshot_state
                     (tenant_id, election_event_id, live_snapshot_revision, watermarks)
                 VALUES ($1, $2, $3, $4)";
    let (no_run, object): (Option<i64>, serde_json::Value) = (None, serde_json::json!({}));
    refused_by(
        attempt(
            &mut tx,
            state,
            &[&s.tenant, &s.event, &no_run, &serde_json::json!([])],
        )
        .await,
        "monitoring_snapshot_state_watermarks_check",
        "watermarks that are not an object",
    );
    refused_by(
        attempt(
            &mut tx,
            state,
            &[&s.tenant, &s.event, &Some(first), &object],
        )
        .await,
        "monitoring_snapshot_state_shows_a_complete_run",
        "a running run",
    );
    assert_eq!(
        attempt(&mut tx, state, &[&s.tenant, &s.event, &no_run, &object]).await,
        Ok(1),
        "a state that shows nothing yet"
    );
    for (scope, revision) in [(s, first), (later, second)] {
        assert_eq!(
            attempt(
                &mut tx,
                run,
                &[
                    &scope.tenant,
                    &scope.event,
                    &revision,
                    &"COMPLETE",
                    &at,
                    &none,
                    &at,
                    &settings,
                    &generation
                ],
            )
            .await,
            Ok(1)
        );
    }
    let show = "UPDATE sequent_backend.monitoring_snapshot_state SET live_snapshot_revision = $3
                WHERE tenant_id = $1 AND election_event_id = $2";
    refused_by(
        attempt(&mut tx, show, &[&s.tenant, &s.event, &second]).await,
        "monitoring_snapshot_state_shows_a_complete_run",
        "another event's complete run",
    );
    assert_eq!(
        attempt(&mut tx, show, &[&s.tenant, &s.event, &first]).await,
        Ok(1)
    );

    for (change, why) in [
        (
            "status = 'FAILED', error = 'late'",
            "the shown run marked failed",
        ),
        (
            "as_of = as_of - interval '1 hour'",
            "its figures dated otherwise",
        ),
        ("config_generation = 7", "its configuration changed"),
    ] {
        refused_by(
            attempt(
                &mut tx,
                &format!(
                    "UPDATE sequent_backend.monitoring_snapshot_run SET {change}
                     WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3"
                ),
                &[&s.tenant, &s.event, &first],
            )
            .await,
            "monitoring_snapshot_run_is_final",
            why,
        );
    }
    let before = Some("2026-05-04T10:14:59Z");
    refused_by(
        attempt(&mut tx, check, &[&s.tenant, &s.event, &first, &before]).await,
        "monitoring_snapshot_run_checked_when_complete",
        "checked before the moment its figures are from",
    );
    let (checked, earlier) = (Some("2026-05-04T10:30:00Z"), Some("2026-05-04T10:20:00Z"));
    assert_eq!(
        attempt(&mut tx, check, &[&s.tenant, &s.event, &first, &checked]).await,
        Ok(1),
        "a later pass that found nothing new says so"
    );
    for (when, why) in [
        (earlier, "checked earlier than it was"),
        (none, "never checked after all"),
    ] {
        refused_by(
            attempt(&mut tx, check, &[&s.tenant, &s.event, &first, &when]).await,
            "monitoring_snapshot_run_is_final",
            why,
        );
    }

    // What viewers are shown only moves on.
    let third: i64 = tx
        .query_one(start, &[&s.tenant, &s.event])
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        attempt(
            &mut tx,
            run,
            &[
                &s.tenant,
                &s.event,
                &third,
                &"COMPLETE",
                &at,
                &none,
                &at,
                &settings,
                &generation
            ],
        )
        .await,
        Ok(1)
    );
    assert_eq!(
        attempt(&mut tx, show, &[&s.tenant, &s.event, &third]).await,
        Ok(1)
    );
    for (revision, why) in [
        (Some(first), "back to an older run"),
        (no_run, "back to no run"),
    ] {
        refused_by(
            attempt(&mut tx, show, &[&s.tenant, &s.event, &revision]).await,
            "monitoring_snapshot_state_moves_forward",
            why,
        );
    }
    refused_by(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_snapshot_state
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event],
        )
        .await,
        "monitoring_snapshot_state_moves_forward",
        "the state removed but with its event",
    );
    let prune = "DELETE FROM sequent_backend.monitoring_snapshot_run
                 WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3";
    refused_by(
        attempt(&mut tx, prune, &[&s.tenant, &s.event, &third]).await,
        "monitoring_snapshot_state_shows_a_complete_run",
        "the shown run pruned",
    );
    assert_eq!(
        attempt(&mut tx, prune, &[&s.tenant, &s.event, &first]).await,
        Ok(1),
        "an older run pruned"
    );

    tx.execute(
        "UPDATE sequent_backend.monitoring_snapshot_state SET updated_at = '2000-01-01T00:00:00Z'
         WHERE election_event_id = $1",
        &[&s.event],
    )
    .await
    .unwrap();
    let touched: bool = tx
        .query_one(
            "UPDATE sequent_backend.monitoring_snapshot_state
             SET watermarks = '{\"voters\": \"2026-05-04T10:15:00Z\"}'
             WHERE election_event_id = $1 RETURNING updated_at = now()",
            &[&s.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(touched, "updated_at follows every change");
}

#[tokio::test]
async fn a_set_of_elections_is_recorded_under_the_key_of_its_ids() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let (a, b) = (id(line!(), 3), id(line!(), 4));
    let set = "INSERT INTO sequent_backend.monitoring_election_set
                   (tenant_id, election_event_id, election_set_key, election_ids)
               VALUES ($1, $2, $3, $4)";
    let both = set_key(&[b, a]);
    for (key, ids, constraint, why) in [
        (
            set_key(&[a]),
            vec![a, b],
            "monitoring_election_set_key_is_of_ids",
            "a key of other elections",
        ),
        (
            both.clone(),
            vec![b, a],
            "monitoring_election_set_ids_ascend",
            "ids out of order",
        ),
        (
            set_key(&[a]),
            vec![a, a],
            "monitoring_election_set_ids_ascend",
            "an id twice",
        ),
    ] {
        refused_by(
            attempt(&mut tx, set, &[&s.tenant, &s.event, &key, &ids]).await,
            constraint,
            why,
        );
    }
    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_election_set
                 (tenant_id, election_event_id, election_set_key, election_ids)
             VALUES ($1, $2, $3, ARRAY[$4, NULL]::uuid[])",
            &[&s.tenant, &s.event, &both, &a],
        )
        .await,
        "monitoring_election_set_ids_ascend",
        "a missing id",
    );
    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_election_set
                 (tenant_id, election_event_id, election_set_key, election_ids)
             VALUES ($1, $2, $3, ARRAY[[$4, $5]]::uuid[])",
            &[&s.tenant, &s.event, &both, &a, &b],
        )
        .await,
        "monitoring_election_set_ids_ascend",
        "a nested list",
    );
    let nothing: Vec<Uuid> = Vec::new();
    for (key, ids) in [(both, vec![a, b]), (set_key(&nothing), nothing.clone())] {
        assert_eq!(
            attempt(&mut tx, set, &[&s.tenant, &s.event, &key, &ids]).await,
            Ok(1),
            "the key sequent-core derives, the empty set included"
        );
    }
}

#[tokio::test]
async fn a_scope_shows_one_stored_payload_over_a_range_of_runs() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let post = id(line!(), 3);
    let key = election_set(&tx, s, &[post]).await;

    let payload_row = "INSERT INTO sequent_backend.monitoring_snapshot_payload
                           (tenant_id, election_event_id, sha256, payload)
                       VALUES ($1, $2, $3, $4)";
    let object = serde_json::json!({"rows": []});
    let (early, late) = (digest("early"), digest("late"));
    for sha in [&early, &late] {
        assert_eq!(
            attempt(&mut tx, payload_row, &[&s.tenant, &s.event, sha, &object]).await,
            Ok(1)
        );
    }
    let north = payload(&tx, s, "north").await;
    let short = early[..16].to_vec();
    for (sha, value, constraint, why) in [
        (
            &early,
            &object,
            "monitoring_snapshot_payload_pkey",
            "the same figures stored twice",
        ),
        (
            &short,
            &object,
            "monitoring_snapshot_payload_sha256_check",
            "not a SHA-256",
        ),
        (
            &digest("list"),
            &serde_json::json!([]),
            "monitoring_snapshot_payload_payload_check",
            "figures that are not an object",
        ),
    ] {
        refused_by(
            attempt(&mut tx, payload_row, &[&s.tenant, &s.event, sha, value]).await,
            constraint,
            why,
        );
    }
    refused_by(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.monitoring_snapshot_payload SET payload = '{\"rows\": [1]}'
             WHERE election_event_id = $1",
            &[&s.event],
        )
        .await,
        "monitoring_snapshot_payload_is_immutable",
        "figures changed under their hash",
    );

    let figure = "INSERT INTO sequent_backend.monitoring_snapshot_figure
                      (tenant_id, election_event_id, source, election_set_key, scope_key,
                       from_revision, to_revision, payload_sha256)
                  VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
    let other = scope(&tx, line!()).await;
    let foreign = start_run(&tx, other).await;
    let nowhere = i64::MAX;
    // A pass that crashed and was never marked failed.
    let crashed = start_run(&tx, s).await;

    // The pass at r1 opens the scope.
    let r1 = start_run(&tx, s).await;
    let row = |source: &'static str, scope_key: &'static str, from: i64, to: Option<i64>| {
        (source, scope_key, from, to, early.clone(), key.clone())
    };
    let refusals = [
        (
            row("voter_turnout", "post=e1", r1, Some(nowhere)),
            "monitoring_snapshot_figure_only_closes",
            "a row opened already closed",
        ),
        (
            row("voter_turnout", "post=e1", nowhere, None),
            "monitoring_snapshot_figure_of_a_running_pass",
            "a row from no run",
        ),
        (
            row("voter_turnout", "post=e1", foreign, None),
            "monitoring_snapshot_figure_of_a_running_pass",
            "a row from another event's run",
        ),
        (
            row("weather", "event", r1, None),
            "monitoring_snapshot_figure_source_check",
            "an unknown source",
        ),
        (
            row("voter_turnout", "Post=e1", r1, None),
            "monitoring_snapshot_figure_scope_key_check",
            "a scope key out of form",
        ),
        (
            row("voter_turnout", "country=ES&post=e1", r1, None),
            "monitoring_snapshot_figure_scope_key_check",
            "scope parts out of order",
        ),
        (
            (
                "voter_turnout",
                "event",
                r1,
                None,
                digest("never stored"),
                key.clone(),
            ),
            "monitoring_snapshot_figure_payload_exists",
            "figures never stored",
        ),
        (
            (
                "voter_turnout",
                "event",
                r1,
                None,
                early.clone(),
                set_key(&[]),
            ),
            "monitoring_snapshot_figure_set_is_recorded",
            "a set nobody recorded",
        ),
        (
            (
                "voter_turnout",
                "event",
                r1,
                None,
                short.clone(),
                key.clone(),
            ),
            "monitoring_snapshot_figure_payload_sha256_check",
            "not a SHA-256",
        ),
    ];
    for ((source, scope_key, from, to, sha, set), constraint, why) in refusals {
        refused_by(
            attempt(
                &mut tx,
                figure,
                &[
                    &s.tenant, &s.event, &source, &set, &scope_key, &from, &to, &sha,
                ],
            )
            .await,
            constraint,
            why,
        );
    }
    // A pass that writes figures completes: neither a failed pass's nor an
    // unfinished one's figures are committed.
    for status in ["FAILED", "RUNNING"] {
        let savepoint = Transaction::savepoint(&mut tx, "pass").await.unwrap();
        let pass = start_run(&savepoint, s).await;
        open_figure(&savepoint, s, (key.as_str(), "event"), pass, &early)
            .await
            .unwrap();
        if status == "FAILED" {
            end_run(&savepoint, s, pass, status).await.unwrap();
        }
        refused_by(
            settle(savepoint, Ok(1)).await,
            "monitoring_snapshot_figure_completes_its_run",
            status,
        );
    }
    for (scope_key, sha) in [("post=e1", &early), ("region=north", &north)] {
        open_figure(&tx, s, (key.as_str(), scope_key), r1, sha)
            .await
            .unwrap();
    }
    end_run(&tx, s, r1, "COMPLETE").await.unwrap();

    // The pass at r2 closes the row and opens the next, in that order.
    let r2 = start_run(&tx, s).await;
    let open = |scope_key: &'static str, from: i64| {
        (
            figure,
            vec![
                Box::new(s.tenant) as Box<dyn ToSql + Sync>,
                Box::new(s.event),
                Box::new("voter_turnout"),
                Box::new(key.clone()),
                Box::new(scope_key),
                Box::new(from),
                Box::new(None::<i64>),
                Box::new(late.clone()),
            ],
        )
    };
    let close = |scope_key: &'static str, at: i64| {
        (
            "UPDATE sequent_backend.monitoring_snapshot_figure SET to_revision = $4
             WHERE tenant_id = $1 AND election_event_id = $2 AND scope_key = $3
               AND to_revision IS NULL",
            vec![
                Box::new(s.tenant) as Box<dyn ToSql + Sync>,
                Box::new(s.event),
                Box::new(scope_key),
                Box::new(at),
            ],
        )
    };
    for ((statement, params), constraint, why) in [
        (
            open("post=e1", r2),
            "monitoring_snapshot_figure_is_disjoint",
            "a second showing row",
        ),
        (
            open("post=e3", r1),
            "monitoring_snapshot_figure_of_a_running_pass",
            "a scope added to a complete run",
        ),
        (
            open("post=e3", crashed),
            "monitoring_snapshot_figure_is_read",
            "a crashed pass's scope under a complete run",
        ),
        (
            close("post=e1", crashed),
            "monitoring_snapshot_figure_is_read",
            "a scope closed at a crashed pass, taken from a complete run",
        ),
        (
            close("post=e1", nowhere),
            "monitoring_snapshot_figure_of_a_running_pass",
            "closed at no run",
        ),
    ] {
        let params: Vec<&(dyn ToSql + Sync)> = params.iter().map(|param| param.as_ref()).collect();
        refused_by(attempt(&mut tx, statement, &params).await, constraint, why);
    }
    assert_eq!(close_figure(&tx, s, "post=e1", r2).await.unwrap(), 1);
    open_figure(&tx, s, (key.as_str(), "post=e1"), r2, &late)
        .await
        .unwrap();
    end_run(&tx, s, r2, "COMPLETE").await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_snapshot_state
             (tenant_id, election_event_id, live_snapshot_revision)
         VALUES ($1, $2, $3)",
        &[&s.tenant, &s.event, &r2],
    )
    .await
    .unwrap();

    for (change, why) in [
        ("to_revision = NULL", "a closed row reopened"),
        ("to_revision = $2", "a closed row closed again"),
        ("payload_sha256 = payload_sha256", "a row rewritten"),
    ] {
        refused_by(
            attempt(
                &mut tx,
                &format!(
                    "UPDATE sequent_backend.monitoring_snapshot_figure SET {change}
                     WHERE election_event_id = $1 AND from_revision = $3 AND $2::bigint > 0
                       AND scope_key = 'post=e1'"
                ),
                &[&s.event, &r2, &r1],
            )
            .await,
            "monitoring_snapshot_figure_only_closes",
            why,
        );
    }
    refused_by(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.monitoring_snapshot_figure SET to_revision = $2
             WHERE election_event_id = $1 AND scope_key = 'region=north'",
            &[&s.event, &r2],
        )
        .await,
        "monitoring_snapshot_figure_of_a_running_pass",
        "a scope taken from the shown run",
    );

    // A failed pass writes nothing that later passes carry forward.
    let failed = start_run(&tx, s).await;
    end_run(&tx, s, failed, "FAILED").await.unwrap();
    for (result, why) in [
        (
            attempt(
                &mut tx,
                "UPDATE sequent_backend.monitoring_snapshot_figure SET to_revision = $2
                 WHERE election_event_id = $1 AND scope_key = 'post=e1' AND to_revision IS NULL",
                &[&s.event, &failed],
            )
            .await,
            "a scope closed at a failed pass",
        ),
        (
            attempt(
                &mut tx,
                figure,
                &[
                    &s.tenant,
                    &s.event,
                    &"voter_turnout",
                    &key,
                    &"post=e3",
                    &failed,
                    &None::<i64>,
                    &late,
                ],
            )
            .await,
            "a scope opened by a failed pass",
        ),
    ] {
        refused_by(result, "monitoring_snapshot_figure_of_a_running_pass", why);
    }

    // The pass at r3 finds post=e1 gone.
    let r3 = start_run(&tx, s).await;
    assert_eq!(close_figure(&tx, s, "post=e1", r3).await.unwrap(), 1);
    end_run(&tx, s, r3, "COMPLETE").await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.monitoring_snapshot_state SET live_snapshot_revision = $3
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&s.tenant, &s.event, &r3],
    )
    .await
    .unwrap();

    // The passes' checks at commit, which the refusals above left pending
    // again as each rolled back; pruning runs after them.
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE; SET CONSTRAINTS ALL DEFERRED")
        .await
        .unwrap();

    // Each revision reads the one payload its range holds.
    let read = "SELECT payload_sha256 FROM sequent_backend.monitoring_snapshot_figure
                WHERE tenant_id = $1 AND election_event_id = $2 AND source = 'voter_turnout'
                  AND election_set_key = $3 AND scope_key = $4
                  AND int8range(from_revision, to_revision) @> $5::bigint";
    for (scope_key, revision, expected) in [
        ("post=e1", crashed, None),
        ("post=e1", r1, Some(&early)),
        ("post=e1", r2, Some(&late)),
        ("post=e1", r3, None),
        ("region=north", crashed, None),
        ("region=north", r1, Some(&north)),
        ("region=north", r2, Some(&north)),
        ("region=north", r3, Some(&north)),
    ] {
        let shown: Option<Vec<u8>> = tx
            .query_opt(read, &[&s.tenant, &s.event, &key, &scope_key, &revision])
            .await
            .unwrap()
            .map(|row| row.get(0));
        assert_eq!(shown.as_ref(), expected, "{scope_key} at {revision}");
    }

    // Pruning cannot take what a kept run still reads.
    for (statement, constraint) in [
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_figure WHERE election_event_id = $1",
            "monitoring_snapshot_figure_is_read",
        ),
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_payload WHERE election_event_id = $1",
            "monitoring_snapshot_figure_payload_exists",
        ),
        (
            "DELETE FROM sequent_backend.monitoring_election_set WHERE election_event_id = $1",
            "monitoring_snapshot_figure_set_is_recorded",
        ),
    ] {
        refused_by(
            attempt(&mut tx, statement, &[&s.event]).await,
            constraint,
            statement,
        );
    }

    // Keeping the runs from r2: the runs before it go, then the rows that
    // hold none of the runs left, then the payloads no row names any more.
    assert_eq!(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_snapshot_run
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision < $3",
            &[&s.tenant, &s.event, &r2],
        )
        .await,
        Ok(2),
        "r1 and the crashed pass"
    );
    let freed: Vec<Vec<u8>> = tx
        .query(
            "DELETE FROM sequent_backend.monitoring_snapshot_figure AS figure
             WHERE tenant_id = $1 AND election_event_id = $2
               AND NOT EXISTS (
                   SELECT 1 FROM sequent_backend.monitoring_snapshot_run AS run
                   WHERE run.tenant_id = figure.tenant_id
                     AND run.election_event_id = figure.election_event_id
                     AND run.revision >= figure.from_revision
                     AND (figure.to_revision IS NULL OR run.revision < figure.to_revision)
                     AND run.status = 'COMPLETE'
               )
             RETURNING payload_sha256",
            &[&s.tenant, &s.event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(freed, [early.clone()]);
    assert_eq!(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_snapshot_payload AS payload
             WHERE tenant_id = $1 AND election_event_id = $2 AND sha256 = ANY($3)
               AND NOT EXISTS (
                   SELECT 1 FROM sequent_backend.monitoring_snapshot_figure AS figure
                   WHERE figure.tenant_id = payload.tenant_id
                     AND figure.election_event_id = payload.election_event_id
                     AND figure.payload_sha256 = payload.sha256
               )",
            &[&s.tenant, &s.event, &freed],
        )
        .await,
        Ok(1)
    );
    refused_by(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_snapshot_figure
             WHERE election_event_id = $1 AND from_revision = $2 AND scope_key = 'post=e1'",
            &[&s.event, &r2],
        )
        .await,
        "monitoring_snapshot_figure_is_read",
        "a row holding a kept run",
    );
}

#[tokio::test]
async fn of_two_passes_opening_one_scope_the_second_is_refused() {
    let pool = schema::pool().await;
    let mut first = pool.get().await.unwrap();
    let mut second = pool.get().await.unwrap();
    let (s, key, sha, earlier, later) = {
        let setup = first.transaction().await.unwrap();
        let s = scope(&setup, line!()).await;
        let key = election_set(&setup, s, &[]).await;
        let sha = payload(&setup, s, "figures").await;
        let earlier = start_run(&setup, s).await;
        let later = start_run(&setup, s).await;
        setup.commit().await.unwrap();
        (s, key, sha, earlier, later)
    };
    let a = first.transaction().await.unwrap();
    let b = second.transaction().await.unwrap();
    assert_eq!(
        open_figure(&a, s, (key.as_str(), "event"), earlier, &sha)
            .await
            .unwrap(),
        1
    );
    end_run(&a, s, earlier, "COMPLETE").await.unwrap();
    let error = {
        let waiting = open_figure(&b, s, (key.as_str(), "event"), later, &sha);
        tokio::pin!(waiting);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), &mut waiting)
                .await
                .is_err(),
            "the second waits to see whether the first commits"
        );
        a.commit().await.unwrap();
        waiting.await.unwrap_err()
    };
    assert_eq!(refusal(&error), "monitoring_snapshot_figure_is_disjoint");
    b.rollback().await.unwrap();

    let cleanup = first.transaction().await.unwrap();
    delete_election_event(&cleanup, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

#[tokio::test]
async fn a_reader_of_a_run_sees_all_its_figures_or_none_while_it_is_pruned() {
    let pool = schema::pool().await;
    let mut reader = pool.get().await.unwrap();
    let mut pruner = pool.get().await.unwrap();
    let (s, old, live) = {
        let setup = pruner.transaction().await.unwrap();
        let s = scope(&setup, line!()).await;
        let key = election_set(&setup, s, &[]).await;
        let (a, b) = (payload(&setup, s, "a").await, payload(&setup, s, "b").await);
        let old = start_run(&setup, s).await;
        for scope_key in ["event", "region=north"] {
            open_figure(&setup, s, (key.as_str(), scope_key), old, &a)
                .await
                .unwrap();
        }
        end_run(&setup, s, old, "COMPLETE").await.unwrap();
        let live = start_run(&setup, s).await;
        close_figure(&setup, s, "event", live).await.unwrap();
        open_figure(&setup, s, (key.as_str(), "event"), live, &b)
            .await
            .unwrap();
        end_run(&setup, s, live, "COMPLETE").await.unwrap();
        setup
            .execute(
                "INSERT INTO sequent_backend.monitoring_snapshot_state
                     (tenant_id, election_event_id, live_snapshot_revision)
                 VALUES ($1, $2, $3)",
                &[&s.tenant, &s.event, &live],
            )
            .await
            .unwrap();
        setup.commit().await.unwrap();
        (s, old, live)
    };
    let found = "SELECT count(*) FROM sequent_backend.monitoring_snapshot_run
                 WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3
                   AND status = 'COMPLETE'";
    let figures = "SELECT count(*) FROM sequent_backend.monitoring_snapshot_figure
                   WHERE tenant_id = $1 AND election_event_id = $2
                     AND int8range(from_revision, to_revision) @> $3::bigint";

    // An export of the old run finds it, then the job prunes it.
    let export = reader
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .start()
        .await
        .unwrap();
    assert_eq!(count(&export, found, &[&s.tenant, &s.event, &old]).await, 1);
    let prune = pruner.transaction().await.unwrap();
    prune
        .batch_execute("SET CONSTRAINTS ALL IMMEDIATE")
        .await
        .unwrap();
    let prunes: [(&str, u64); 2] = [
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_run
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
            1,
        ),
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_figure AS figure
             WHERE tenant_id = $1 AND election_event_id = $2 AND $3::bigint > 0
               AND NOT EXISTS (
                   SELECT 1 FROM sequent_backend.monitoring_snapshot_run AS run
                   WHERE run.tenant_id = figure.tenant_id
                     AND run.election_event_id = figure.election_event_id
                     AND run.revision >= figure.from_revision
                     AND (figure.to_revision IS NULL OR run.revision < figure.to_revision)
                     AND run.status = 'COMPLETE'
               )",
            1,
        ),
    ];
    for (statement, rows) in prunes {
        let pruned = without_waiting(
            prune.execute(statement, &[&s.tenant, &s.event, &old]),
            "pruning",
        )
        .await
        .unwrap();
        assert_eq!(pruned, rows, "{statement}");
    }
    prune.commit().await.unwrap();
    assert_eq!(
        count(&export, figures, &[&s.tenant, &s.event, &old]).await,
        2,
        "the export still reads every scope of the run it found"
    );
    export.commit().await.unwrap();
    // One that starts later finds the run gone, rather than part of it.
    let late = reader.transaction().await.unwrap();
    assert_eq!(count(&late, found, &[&s.tenant, &s.event, &old]).await, 0);
    assert_eq!(
        count(&late, figures, &[&s.tenant, &s.event, &live]).await,
        2
    );
    late.rollback().await.unwrap();

    let cleanup = pruner.transaction().await.unwrap();
    delete_election_event(&cleanup, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

#[tokio::test]
async fn a_run_records_whether_each_source_was_counted() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let key = set_key(&Vec::<Uuid>::new());
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_election_set
             (tenant_id, election_event_id, election_set_key, election_ids)
         VALUES ($1, $2, $3, '{}')",
        &[&s.tenant, &s.event, &key],
    )
    .await
    .unwrap();
    let revision: i64 = tx
        .query_one(
            "INSERT INTO sequent_backend.monitoring_snapshot_run (tenant_id, election_event_id, status)
             VALUES ($1, $2, 'RUNNING') RETURNING revision",
            &[&s.tenant, &s.event],
        )
        .await
        .unwrap()
        .get(0);
    let source = "INSERT INTO sequent_backend.monitoring_snapshot_source
                      (tenant_id, election_event_id, revision, source, election_set_key,
                       producer_status, reason)
                  VALUES ($1, $2, $3, $4, $5, $6, $7)";
    let no_reason: Option<&str> = None;
    for id in DataSourceId::iter().filter(|id| *id != DataSourceId::Helpdesk) {
        let id = id.to_string();
        assert_eq!(
            attempt(
                &mut tx,
                source,
                &[
                    &s.tenant,
                    &s.event,
                    &revision,
                    &id,
                    &key,
                    &"CONNECTED",
                    &no_reason
                ]
            )
            .await,
            Ok(1),
            "{id}"
        );
    }
    let missing = set_key(&[id(line!(), 3)]);
    for (id, set, status, reason, constraint) in [
        (
            "weather",
            &key,
            "CONNECTED",
            None,
            "monitoring_snapshot_source_source_check",
        ),
        (
            "helpdesk",
            &key,
            "BROKEN",
            None,
            "monitoring_snapshot_source_producer_status_check",
        ),
        (
            "helpdesk",
            &key,
            "NOT_CONNECTED",
            None,
            "monitoring_snapshot_source_not_connected_says_why",
        ),
        (
            "helpdesk",
            &key,
            "NOT_CONNECTED",
            Some(""),
            "monitoring_snapshot_source_not_connected_says_why",
        ),
        (
            "helpdesk",
            &key,
            "CONNECTED",
            Some("why"),
            "monitoring_snapshot_source_not_connected_says_why",
        ),
        (
            "helpdesk",
            &missing,
            "CONNECTED",
            None,
            "monitoring_snapshot_source_set_is_recorded",
        ),
        (
            "voter_turnout",
            &key,
            "CONNECTED",
            None,
            "monitoring_snapshot_source_pkey",
        ),
    ] {
        refused_by(
            attempt(
                &mut tx,
                source,
                &[&s.tenant, &s.event, &revision, &id, set, &status, &reason],
            )
            .await,
            constraint,
            constraint,
        );
    }
    assert_eq!(
        attempt(
            &mut tx,
            source,
            &[
                &s.tenant,
                &s.event,
                &revision,
                &"helpdesk",
                &key,
                &"NOT_CONNECTED",
                &Some("No producer yet")
            ],
        )
        .await,
        Ok(1)
    );
    refused_by(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.monitoring_snapshot_source SET producer_status = 'CONNECTED', reason = NULL
             WHERE election_event_id = $1 AND source = 'helpdesk'",
            &[&s.event],
        )
        .await,
        "monitoring_snapshot_source_is_immutable",
        "a run's record rewritten",
    );
    refused_by(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_election_set WHERE election_event_id = $1",
            &[&s.event],
        )
        .await,
        "monitoring_snapshot_source_set_is_recorded",
        "a set a run names, pruned",
    );
    // With its run, a run's record goes.
    let gone = attempt(
        &mut tx,
        "DELETE FROM sequent_backend.monitoring_snapshot_run WHERE election_event_id = $1",
        &[&s.event],
    )
    .await;
    assert_eq!(gone, Ok(1));
    let left = count(
        &tx,
        "SELECT count(*) FROM sequent_backend.monitoring_snapshot_source WHERE election_event_id = $1",
        &[&s.event],
    )
    .await;
    assert_eq!(left, 0);
}

#[tokio::test]
async fn pruning_out_of_order_fails_at_commit_unless_checked_at_once() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let s = {
        let setup = client.transaction().await.unwrap();
        let s = scope(&setup, line!()).await;
        let key = election_set(&setup, s, &[]).await;
        let figures = payload(&setup, s, "figures").await;
        // A scope the pass at `from` opened and the pass at `to` closed,
        // with the run at `from` pruned: no run left reads the row.
        let from = start_run(&setup, s).await;
        open_figure(&setup, s, (key.as_str(), "event"), from, &figures)
            .await
            .unwrap();
        end_run(&setup, s, from, "COMPLETE").await.unwrap();
        let to = start_run(&setup, s).await;
        close_figure(&setup, s, "event", to).await.unwrap();
        end_run(&setup, s, to, "COMPLETE").await.unwrap();
        setup.commit().await.unwrap();
        let setup = client.transaction().await.unwrap();
        setup
            .execute(
                "DELETE FROM sequent_backend.monitoring_snapshot_run
                 WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
                &[&s.tenant, &s.event, &from],
            )
            .await
            .unwrap();
        setup.commit().await.unwrap();
        s
    };
    let payloads =
        "DELETE FROM sequent_backend.monitoring_snapshot_payload WHERE election_event_id = $1";
    let figures =
        "DELETE FROM sequent_backend.monitoring_snapshot_figure WHERE election_event_id = $1";

    // Deferred: the statement succeeds and the whole transaction is lost.
    let late = client.transaction().await.unwrap();
    assert_eq!(late.execute(payloads, &[&s.event]).await.unwrap(), 1);
    let error = late.commit().await.unwrap_err();
    assert_eq!(refusal(&error), "monitoring_snapshot_figure_payload_exists");
    // Checked at once, as the job prunes: the statement that errs fails.
    let at_once = client.transaction().await.unwrap();
    at_once
        .batch_execute("SET CONSTRAINTS ALL IMMEDIATE")
        .await
        .unwrap();
    let error = at_once.execute(payloads, &[&s.event]).await.unwrap_err();
    assert_eq!(refusal(&error), "monitoring_snapshot_figure_payload_exists");
    at_once.rollback().await.unwrap();
    // Both in one transaction, in either order, commit.
    let both = client.transaction().await.unwrap();
    both.execute(payloads, &[&s.event]).await.unwrap();
    both.execute(figures, &[&s.event]).await.unwrap();
    both.commit().await.unwrap();

    let cleanup = client.transaction().await.unwrap();
    delete_election_event(&cleanup, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

#[tokio::test]
async fn no_monitoring_table_can_be_truncated() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // Checked in the catalogue rather than by truncating the shared tables.
    for table in TABLES {
        let guards = count(
            &tx,
            "SELECT count(*) FROM pg_trigger
             WHERE tgrelid = ('sequent_backend.' || $1)::regclass
               AND tgtype & 32 <> 0 AND tgtype & 2 <> 0 AND tgenabled = 'O'
               AND tgfoid = 'sequent_backend.monitoring_refuse_change'::regproc",
            &[&table],
        )
        .await;
        assert_eq!(guards, 1, "{table}");
    }
}

/// The values a CHECK constraint lists.
async fn listed(tx: &Transaction<'_>, constraint: &str) -> BTreeSet<String> {
    let definition: String = tx
        .query_one(
            "SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conname = $1",
            &[&constraint],
        )
        .await
        .unwrap()
        .get(0);
    definition
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn the_tables_list_exactly_the_kinds_and_sources_the_code_has() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let kinds: BTreeSet<String> = ConfigKind::iter().map(|kind| kind.to_string()).collect();
    assert_eq!(listed(&tx, "monitoring_config_kind_check").await, kinds);
    let sources: BTreeSet<String> = DataSourceId::iter()
        .map(|source| source.to_string())
        .collect();
    for constraint in [
        "monitoring_snapshot_source_source_check",
        "monitoring_snapshot_figure_source_check",
    ] {
        assert_eq!(listed(&tx, constraint).await, sources, "{constraint}");
    }
}

#[tokio::test]
async fn a_voter_row_holds_derived_values_only_and_goes_with_its_election() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let post = id(line!(), 3);
    election(&tx, s, post).await;
    let voter = "INSERT INTO sequent_backend.monitoring_voter
                     (tenant_id, election_event_id, election_id, voter_id, region, dims,
                      enrollment_state, enrollment_reason, attributes_hash, settings_revision)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
    let object = serde_json::json!({"age_band": "18-24"});
    let array = serde_json::json!(["18-24"]);
    let none: Option<&str> = None;
    let cases = [
        (
            "",
            none,
            &object,
            none,
            none,
            "h",
            1,
            "monitoring_voter_voter_id_check",
        ),
        (
            "v",
            Some(""),
            &object,
            none,
            none,
            "h",
            1,
            "monitoring_voter_region_check",
        ),
        (
            "v",
            none,
            &array,
            none,
            none,
            "h",
            1,
            "monitoring_voter_dims_check",
        ),
        (
            "v",
            none,
            &object,
            Some("WAITING"),
            none,
            "h",
            1,
            "monitoring_voter_enrollment_state_check",
        ),
        (
            "v",
            none,
            &object,
            Some("ACCEPTED"),
            Some("wrong-area"),
            "h",
            1,
            "monitoring_voter_reason_is_for_rejection",
        ),
        (
            "v",
            none,
            &object,
            none,
            Some("wrong-area"),
            "h",
            1,
            "monitoring_voter_reason_is_for_rejection",
        ),
        (
            "v",
            none,
            &object,
            Some("REJECTED"),
            Some(""),
            "h",
            1,
            "monitoring_voter_enrollment_reason_check",
        ),
        (
            "v",
            none,
            &object,
            none,
            none,
            "",
            1,
            "monitoring_voter_attributes_hash_check",
        ),
        (
            "v",
            none,
            &object,
            none,
            none,
            "h",
            0,
            "monitoring_voter_settings_revision_check",
        ),
    ];
    for (voter_id, region, dims, state, reason, hash, settings, constraint) in cases {
        refused_by(
            attempt(
                &mut tx,
                voter,
                &[
                    &s.tenant, &s.event, &post, &voter_id, &region, dims, &state, &reason, &hash,
                    &settings,
                ],
            )
            .await,
            constraint,
            constraint,
        );
    }
    assert_eq!(
        attempt(
            &mut tx,
            voter,
            &[
                &s.tenant,
                &s.event,
                &post,
                &"v",
                &none,
                &object,
                &Some("REJECTED"),
                &Some("wrong-area"),
                &"h",
                &1_i32
            ],
        )
        .await,
        Ok(1)
    );

    tx.execute(
        "UPDATE sequent_backend.monitoring_voter SET updated_at = '2000-01-01T00:00:00Z'
         WHERE election_event_id = $1",
        &[&s.event],
    )
    .await
    .unwrap();
    let touched: bool = tx
        .query_one(
            "UPDATE sequent_backend.monitoring_voter SET first_voted_at = now()
             WHERE election_event_id = $1 RETURNING updated_at = now()",
            &[&s.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(touched);

    tx.execute(
        "DELETE FROM sequent_backend.election WHERE id = $1",
        &[&post],
    )
    .await
    .unwrap();
    let voters = count(
        &tx,
        "SELECT count(*) FROM sequent_backend.monitoring_voter WHERE election_event_id = $1",
        &[&s.event],
    )
    .await;
    assert_eq!(voters, 0, "a removed Post takes its voters' rows with it");
}

#[tokio::test]
async fn login_attempts_are_counted_in_quarter_hours_once_per_delivery() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    // Counted before the event is configured.
    let s = bare_scope(&tx, line!()).await;
    let count_one = "INSERT INTO sequent_backend.monitoring_login_counter
                         (tenant_id, election_event_id, bucket_start, event_type, registration, area_id, attempts)
                     VALUES ($1, $2, $3::text::timestamptz, $4, $5, $6, $7)";
    let no_area: Option<Uuid> = None;
    let nil = Some(Uuid::nil());
    let bucket = "2026-05-04T10:15:00Z";
    assert_eq!(
        attempt(
            &mut tx,
            count_one,
            &[
                &s.tenant,
                &s.event,
                &bucket,
                &"LOGIN",
                &"REGISTERED",
                &no_area,
                &1_i64
            ]
        )
        .await,
        Ok(1)
    );
    let cases = [
        (
            "2026-05-04T10:20:00Z",
            "LOGIN",
            "REGISTERED",
            no_area,
            1_i64,
            "monitoring_login_counter_bucket_start_check",
            "off the quarter hour",
        ),
        (
            "2026-05-04T10:15:30Z",
            "LOGIN",
            "REGISTERED",
            no_area,
            1,
            "monitoring_login_counter_bucket_start_check",
            "with seconds",
        ),
        (
            "2026-05-04T10:15:00.000001Z",
            "LOGIN",
            "REGISTERED",
            no_area,
            1,
            "monitoring_login_counter_bucket_start_check",
            "with microseconds",
        ),
        (
            "infinity",
            "LOGIN",
            "REGISTERED",
            no_area,
            1,
            "monitoring_login_counter_bucket_start_check",
            "no time at all",
        ),
        (
            "2026-05-04T10:30:00Z",
            "LOGIN",
            "GUEST",
            no_area,
            1,
            "monitoring_login_counter_registration_check",
            "an unknown registration",
        ),
        (
            "2026-05-04T10:30:00Z",
            "null",
            "REGISTERED",
            no_area,
            1,
            "monitoring_login_counter_event_type_check",
            "the listener's missing type",
        ),
        (
            "2026-05-04T10:30:00Z",
            "LOGIN",
            "REGISTERED",
            no_area,
            0,
            "monitoring_login_counter_attempts_check",
            "no attempts",
        ),
        (
            "2026-05-04T10:30:00Z",
            "LOGIN",
            "REGISTERED",
            nil,
            1,
            "monitoring_login_counter_area_id_check",
            "the stand-in as an area",
        ),
        (
            bucket,
            "LOGIN",
            "REGISTERED",
            no_area,
            1,
            "monitoring_login_counter_pkey",
            "the same bucket twice without an area",
        ),
    ];
    for (bucket, event_type, registration, area, attempts, constraint, why) in cases {
        refused_by(
            attempt(
                &mut tx,
                count_one,
                &[
                    &s.tenant,
                    &s.event,
                    &bucket,
                    &event_type,
                    &registration,
                    &area,
                    &attempts,
                ],
            )
            .await,
            constraint,
            why,
        );
    }
    // An offset zone's quarter hours are UTC quarter hours too.
    assert_eq!(
        attempt(
            &mut tx,
            count_one,
            &[
                &s.tenant,
                &s.event,
                &"2026-05-04T18:45:00+05:45",
                &"LOGIN_ERROR",
                &"UNREGISTERED",
                &no_area,
                &1_i64
            ],
        )
        .await,
        Ok(1)
    );

    // The counter adds up on the same key, a missing area included, and
    // keeps each area apart.
    let add = "INSERT INTO sequent_backend.monitoring_login_counter
                   (tenant_id, election_event_id, bucket_start, event_type, registration, area_id, attempts)
               VALUES ($1, $2, '2026-05-04T10:15:00Z', 'LOGIN', 'REGISTERED', $3, 1)
               ON CONFLICT (tenant_id, election_event_id, bucket_start, event_type, registration, area_key)
               DO UPDATE SET attempts = monitoring_login_counter.attempts + EXCLUDED.attempts";
    let area = Some(id(line!(), 3));
    for area in [no_area, no_area, area] {
        tx.execute(add, &[&s.tenant, &s.event, &area])
            .await
            .unwrap();
    }
    let attempts: Vec<(Option<Uuid>, i64)> = tx
        .query(
            "SELECT area_id, attempts FROM sequent_backend.monitoring_login_counter
             WHERE election_event_id = $1 AND bucket_start = '2026-05-04T10:15:00Z'
             ORDER BY area_id NULLS FIRST",
            &[&s.event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    assert_eq!(attempts, [(None, 3), (area, 1)]);

    let receipt = "INSERT INTO sequent_backend.monitoring_login_counter_receipt
                       (delivery_id, tenant_id, election_event_id)
                   VALUES ($1, $2, $3) ON CONFLICT DO NOTHING";
    let delivery = sha256("event:1");
    let first = tx
        .execute(receipt, &[&delivery, &s.tenant, &s.event])
        .await
        .unwrap();
    let again = tx
        .execute(receipt, &[&delivery, &s.tenant, &s.event])
        .await
        .unwrap();
    assert_eq!((first, again), (1, 0));
    refused_by(
        attempt(&mut tx, receipt, &[&"delivery-1", &s.tenant, &s.event]).await,
        "monitoring_login_counter_receipt_delivery_id_check",
        "a delivery id that is not the log's digest",
    );
}

#[tokio::test]
async fn a_snapshot_pass_and_a_save_never_wait_for_one_another() {
    let pool = schema::pool().await;
    let mut jobs = pool.get().await.unwrap();
    let mut editor = pool.get().await.unwrap();
    let (s, key, sha) = {
        let mut setup = jobs.transaction().await.unwrap();
        let s = scope(&setup, line!()).await;
        Revision::edit("w", 1, "id: w\n")
            .save(&mut setup, s)
            .await
            .unwrap();
        let key = election_set(&setup, s, &[]).await;
        let sha = payload(&setup, s, "figures").await;
        setup
            .execute(
                "INSERT INTO sequent_backend.monitoring_snapshot_state
                     (tenant_id, election_event_id)
                 VALUES ($1, $2)",
                &[&s.tenant, &s.event],
            )
            .await
            .unwrap();
        setup.commit().await.unwrap();
        (s, key, sha)
    };
    // As the job does: a run recorded as RUNNING on its own, then one
    // REPEATABLE READ transaction that counts, writes and shows it.
    let start = "INSERT INTO sequent_backend.monitoring_snapshot_run
                     (tenant_id, election_event_id, status)
                 VALUES ($1, $2, 'RUNNING') RETURNING revision";
    let complete = "UPDATE sequent_backend.monitoring_snapshot_run
                    SET status = 'COMPLETE', finished_at = now(), as_of = now(),
                        settings_revision = 1, config_generation = $4
                    WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3";
    let show = "UPDATE sequent_backend.monitoring_snapshot_state
                SET live_snapshot_revision = $3, watermarks = $4
                WHERE tenant_id = $1 AND election_event_id = $2";
    let generation_now = "SELECT config_generation FROM sequent_backend.monitoring_event
                          WHERE tenant_id = $1 AND election_event_id = $2";
    let event: [&(dyn ToSql + Sync); 2] = [&s.tenant, &s.event];
    let marks = serde_json::json!({"voters": "2026-05-04T10:15:00Z"});

    // A save holds its rows while a pass runs from start to finish.
    let run: i64 = jobs.query_one(start, &event).await.unwrap().get(0);
    let pass = jobs
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .start()
        .await
        .unwrap();
    let counted_for: i64 = pass.query_one(generation_now, &event).await.unwrap().get(0);
    let save = editor.transaction().await.unwrap();
    assert_eq!(
        save_steps(&save, s, &[Revision::edit("w", 2, "id: w\ntitle: T\n")]).await,
        Ok(1)
    );
    without_waiting(
        open_figure(&pass, s, (key.as_str(), "event"), run, &sha),
        "a figure",
    )
    .await
    .unwrap();
    without_waiting(
        pass.execute(
            "UPDATE sequent_backend.monitoring_snapshot_state SET watermarks = $3
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event, &marks],
        ),
        "moving the watermarks",
    )
    .await
    .unwrap();
    save.commit().await.unwrap();
    // The save committed under the pass's snapshot; the pass still writes,
    // shows its run and commits, counted for the configuration it read.
    without_waiting(
        open_figure(&pass, s, (key.as_str(), "region=north"), run, &sha),
        "a figure after the save",
    )
    .await
    .unwrap();
    without_waiting(
        pass.execute(
            "INSERT INTO sequent_backend.monitoring_snapshot_payload
                 (tenant_id, election_event_id, sha256, payload)
             VALUES ($1, $2, $3, '{}')",
            &[&s.tenant, &s.event, &digest("more figures")],
        ),
        "a payload after the save",
    )
    .await
    .unwrap();
    without_waiting(
        pass.execute(complete, &[&s.tenant, &s.event, &run, &counted_for]),
        "completing the run",
    )
    .await
    .unwrap();
    without_waiting(
        pass.execute(show, &[&s.tenant, &s.event, &run, &marks]),
        "showing the run",
    )
    .await
    .unwrap();
    let counted_again: i64 = pass.query_one(generation_now, &event).await.unwrap().get(0);
    assert_eq!(
        counted_again, counted_for,
        "one snapshot of the configuration"
    );
    pass.commit().await.unwrap();

    // A pass holds its rows while a save runs from start to finish.
    let run: i64 = jobs.query_one(start, &event).await.unwrap().get(0);
    let pass = jobs
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .start()
        .await
        .unwrap();
    let counted_for: i64 = pass.query_one(generation_now, &event).await.unwrap().get(0);
    pass.execute(complete, &[&s.tenant, &s.event, &run, &counted_for])
        .await
        .unwrap();
    pass.execute(show, &[&s.tenant, &s.event, &run, &marks])
        .await
        .unwrap();
    let save = editor.transaction().await.unwrap();
    let saved = without_waiting(
        save_steps(&save, s, &[Revision::edit("w", 3, "id: w\ntitle: U\n")]),
        "a save",
    )
    .await;
    assert_eq!(saved, Ok(1));
    save.commit().await.unwrap();
    pass.commit().await.unwrap();

    let check = jobs.transaction().await.unwrap();
    let shown: i64 = check
        .query_one(
            "SELECT live_snapshot_revision FROM sequent_backend.monitoring_snapshot_state
             WHERE tenant_id = $1 AND election_event_id = $2",
            &event,
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!((shown, generation(&check, s).await), (run, 3));
    check.rollback().await.unwrap();

    let cleanup = jobs.transaction().await.unwrap();
    delete_election_event(&cleanup, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

#[tokio::test]
async fn every_monitoring_row_goes_with_its_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let post = id(line!(), 3);
    election(&tx, s, post).await;
    Revision::edit("w", 1, "id: w\n")
        .save(&mut tx, s)
        .await
        .unwrap();
    let key = election_set(&tx, s, &[post]).await;
    let figures = payload(&tx, s, "figures").await;
    let revision = start_run(&tx, s).await;
    open_figure(&tx, s, (key.as_str(), "event"), revision, &figures)
        .await
        .unwrap();
    end_run(&tx, s, revision, "COMPLETE").await.unwrap();
    let statements: [(&str, &[&(dyn ToSql + Sync)]); 5] = [
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_source
                 (tenant_id, election_event_id, revision, source, election_set_key, producer_status)
             VALUES ($1, $2, $3, 'voter_turnout', $4, 'CONNECTED')",
            &[&s.tenant, &s.event, &revision, &key],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_state
                 (tenant_id, election_event_id, live_snapshot_revision)
             VALUES ($1, $2, $3)",
            &[&s.tenant, &s.event, &revision],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_voter
                 (tenant_id, election_event_id, election_id, voter_id, attributes_hash, settings_revision)
             VALUES ($1, $2, $3, 'v1', 'h', 1)",
            &[&s.tenant, &s.event, &post],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_login_counter
                 (tenant_id, election_event_id, bucket_start, event_type, registration, attempts)
             VALUES ($1, $2, '2026-05-04T10:15:00Z', 'LOGIN', 'UNREGISTERED', 1)",
            &[&s.tenant, &s.event],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_login_counter_receipt (delivery_id, tenant_id, election_event_id)
             VALUES ($3, $1, $2)",
            &[&s.tenant, &s.event, &sha256("event:1")],
        ),
    ];
    for (statement, params) in statements {
        assert_eq!(
            attempt(&mut tx, statement, params).await,
            Ok(1),
            "{statement}"
        );
    }

    for (table, rows) in row_counts(&tx, s.event).await {
        assert!(
            rows > 0,
            "{table} is empty, so its deletion would prove nothing"
        );
    }
    delete_election_event(&tx, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")
        .await
        .unwrap();
    for (table, rows) in row_counts(&tx, s.event).await {
        assert_eq!(rows, 0, "{table}");
    }
}
