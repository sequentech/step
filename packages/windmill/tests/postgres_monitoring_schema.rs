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
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio_postgres::types::ToSql;
use tokio_postgres::{IsolationLevel, Transaction};
use uuid::Uuid;
use windmill::postgres::election_event::delete_election_event;

const TABLES: [&str; 11] = [
    "monitoring_event",
    "monitoring_config",
    "monitoring_config_head",
    "monitoring_election_set",
    "monitoring_snapshot_run",
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
    election_set_key(ids.iter().map(Uuid::to_string))
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
    let result = match savepoint.execute(sql, params).await {
        Ok(rows) => savepoint
            .batch_execute("SET CONSTRAINTS ALL IMMEDIATE")
            .await
            .map(|()| rows),
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
            Err(refusal(&error))
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

const REVISION: &str = "INSERT INTO sequent_backend.monitoring_config
        (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
         origin, preset_id, preset_version, author_id)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";

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

    /// Writes the revision and moves its head to it, as a save does.
    async fn save(&self, tx: &mut Transaction<'_>, s: Scope) -> Result<u64, String> {
        let sql = format!(
            "WITH revision AS ({REVISION}
                 RETURNING tenant_id, election_event_id, kind, key, revision)
             INSERT INTO sequent_backend.monitoring_config_head
                 (tenant_id, election_event_id, kind, key, revision)
             SELECT * FROM revision
             ON CONFLICT (tenant_id, election_event_id, kind, key)
             DO UPDATE SET revision = EXCLUDED.revision"
        );
        attempt(tx, &sql, &self.params(&s)).await
    }

    /// Writes the revision alone.
    async fn write(&self, tx: &mut Transaction<'_>, s: Scope) -> Result<u64, String> {
        attempt(tx, REVISION, &self.params(&s)).await
    }

    fn params<'p>(&'p self, s: &'p Scope) -> [&'p (dyn ToSql + Sync); 12] {
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
            "SELECT dashboard_mode, config_generation, watermarks, live_snapshot_status
             FROM sequent_backend.monitoring_event
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "LEGACY");
    assert_eq!(row.get::<_, i64>(1), 0);
    assert_eq!(row.get::<_, serde_json::Value>(2), serde_json::json!({}));
    assert_eq!(row.get::<_, Option<String>>(3), None);

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
    refused_by(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.monitoring_event SET watermarks = '[]'
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&s.tenant, &s.event],
        )
        .await,
        "monitoring_event_watermarks_check",
        "watermarks that are not an object",
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

    refused_by(
        Revision::edit("w", 1, "id: w\n").save(&mut tx, s).await,
        "monitoring_config_of_its_event",
        "a revision",
    );
    let cases: [(&str, &[&(dyn ToSql + Sync)], &str); 8] = [
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
        (
            Revision::edit("x", 0, yaml),
            "monitoring_config_revision_check",
            "revisions count from 1",
        ),
        (
            Revision::edit("k", 1, yaml),
            "monitoring_config_pkey",
            "a revision number used twice",
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
    // A revision nobody made the head would take the number of the next save.
    refused_by(
        Revision::edit("k", 5, yaml).write(&mut tx, s).await,
        "monitoring_config_revision_is_the_head",
        "a revision past the head",
    );
    refused_by(
        Revision::edit("new", 1, yaml).write(&mut tx, s).await,
        "monitoring_config_revision_is_the_head",
        "a new document without a head",
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

    // The head's updated_at follows each move.
    tx.execute(
        "UPDATE sequent_backend.monitoring_config_head SET updated_at = '2000-01-01T00:00:00Z'
         WHERE election_event_id = $1",
        &[&s.event],
    )
    .await
    .unwrap();
    Revision::edit("w", 2, "id: w\ntitle: T\n")
        .save(&mut tx, s)
        .await
        .unwrap();
    let moved = count(
        &tx,
        "SELECT count(*) FROM sequent_backend.monitoring_config_head
         WHERE election_event_id = $1 AND revision = 2 AND updated_at = now()",
        &[&s.event],
    )
    .await;
    assert_eq!(moved, 1);
    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_config_head
                 (tenant_id, election_event_id, kind, key, revision)
             VALUES ($1, $2, 'theme', 'w', 1)",
            &[&s.tenant, &s.event],
        )
        .await,
        "monitoring_config_head_names_a_revision",
        "a head for a document with no revisions",
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
    // The protocol the head's table describes.
    let bump =
        "UPDATE sequent_backend.monitoring_event SET config_generation = config_generation + 1
                WHERE tenant_id = $1 AND election_event_id = $2";
    let advance = "UPDATE sequent_backend.monitoring_config_head SET revision = revision + 1
                   WHERE tenant_id = $1 AND election_event_id = $2
                     AND kind = 'widget' AND key = $3 AND revision = $4";
    let create = "INSERT INTO sequent_backend.monitoring_config_head
                      (tenant_id, election_event_id, kind, key, revision)
                  SELECT $1, $2, 'widget', $3, 1 WHERE $4::int = 0
                  ON CONFLICT DO NOTHING";

    for (moves, key, expected) in [(advance, "w", 1_i32), (create, "new", 0)] {
        let mut a = first.transaction().await.unwrap();
        let b = second.transaction().await.unwrap();
        let params: [&(dyn ToSql + Sync); 4] = [&s.tenant, &s.event, &key, &expected];
        a.execute(bump, &params[..2]).await.unwrap();
        assert_eq!(a.execute(moves, &params).await.unwrap(), 1);
        let yaml = format!("id: {key}\ntitle: A\n");
        let revision = Revision::edit(key, expected + 1, &yaml);
        a.execute(REVISION, &revision.params(&s)).await.unwrap();
        let moved = {
            let waiting = b.execute(bump, &params[..2]);
            tokio::pin!(waiting);
            assert!(
                tokio::time::timeout(Duration::from_millis(300), &mut waiting)
                    .await
                    .is_err(),
                "the second save waits for the first"
            );
            a.commit().await.unwrap();
            waiting.await.unwrap();
            b.execute(moves, &params).await.unwrap()
        };
        assert_eq!(moved, 0, "and then moves nothing: a conflict to report");
        b.rollback().await.unwrap();
    }

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
    let params: [&(dyn ToSql + Sync); 4] = [&s.tenant, &s.event, &"w", &2_i32];
    a.execute(bump, &params[..2]).await.unwrap();
    a.execute(advance, &params).await.unwrap();
    let revision = Revision::edit("w", 3, "id: w\ntitle: B\n");
    a.execute(REVISION, &revision.params(&s)).await.unwrap();
    let error = {
        let waiting = b.execute(bump, &params[..2]);
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

    // A head that names a revision nobody wrote is refused at commit.
    let lost = first.transaction().await.unwrap();
    lost.execute(
        "UPDATE sequent_backend.monitoring_config_head SET revision = 9
         WHERE tenant_id = $1 AND election_event_id = $2 AND key = 'w'",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    let error = lost.commit().await.unwrap_err();
    assert_eq!(refusal(&error), "monitoring_config_head_names_a_revision");

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

    let show = "UPDATE sequent_backend.monitoring_event SET live_snapshot_revision = $3
                WHERE tenant_id = $1 AND election_event_id = $2";
    refused_by(
        attempt(&mut tx, show, &[&s.tenant, &s.event, &first]).await,
        "monitoring_event_shows_a_complete_run",
        "a running run",
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
    refused_by(
        attempt(&mut tx, show, &[&s.tenant, &s.event, &second]).await,
        "monitoring_event_shows_a_complete_run",
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
    assert_eq!(
        attempt(
            &mut tx,
            "UPDATE sequent_backend.monitoring_snapshot_run SET checked_at = now()
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
            &[&s.tenant, &s.event, &first],
        )
        .await,
        Ok(1),
        "a later pass that found nothing new says so"
    );
    refused_by(
        attempt(
            &mut tx,
            "DELETE FROM sequent_backend.monitoring_snapshot_run
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
            &[&s.tenant, &s.event, &first],
        )
        .await,
        "monitoring_event_shows_a_complete_run",
        "the shown run pruned",
    );
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
    let key = set_key(&[post]);
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_election_set
             (tenant_id, election_event_id, election_set_key, election_ids)
         VALUES ($1, $2, $3, ARRAY[$4::uuid])",
        &[&s.tenant, &s.event, &key, &post],
    )
    .await
    .unwrap();

    let payload = "INSERT INTO sequent_backend.monitoring_snapshot_payload
                       (tenant_id, election_event_id, sha256, payload)
                   VALUES ($1, $2, $3, $4)";
    let object = serde_json::json!({"rows": []});
    let (early, late) = (digest("early"), digest("late"));
    for sha in [&early, &late] {
        assert_eq!(
            attempt(&mut tx, payload, &[&s.tenant, &s.event, sha, &object]).await,
            Ok(1)
        );
    }
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
            attempt(&mut tx, payload, &[&s.tenant, &s.event, sha, value]).await,
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
    let open: Option<i64> = None;
    let write = |source: &'static str,
                 scope_key: &'static str,
                 from: i64,
                 to: Option<i64>,
                 sha: Vec<u8>,
                 set: String| { (source, scope_key, from, to, sha, set) };
    assert_eq!(
        attempt(
            &mut tx,
            figure,
            &[
                &s.tenant,
                &s.event,
                &"voter_turnout",
                &key,
                &"post=e1",
                &10_i64,
                &open,
                &early
            ]
        )
        .await,
        Ok(1)
    );
    let refusals = [
        (
            write(
                "voter_turnout",
                "post=e1",
                12,
                None,
                early.clone(),
                key.clone(),
            ),
            "monitoring_snapshot_figure_is_disjoint",
            "a second showing row",
        ),
        (
            write(
                "voter_turnout",
                "post=e1",
                5,
                None,
                early.clone(),
                key.clone(),
            ),
            "monitoring_snapshot_figure_is_disjoint",
            "an earlier row still showing",
        ),
        (
            write(
                "voter_turnout",
                "post=e1",
                5,
                Some(11),
                early.clone(),
                key.clone(),
            ),
            "monitoring_snapshot_figure_is_disjoint",
            "an earlier row reaching into it",
        ),
        (
            write(
                "voter_turnout",
                "post=e2",
                5,
                Some(5),
                early.clone(),
                key.clone(),
            ),
            "monitoring_snapshot_figure_range_is_forward",
            "an empty range",
        ),
        (
            write("weather", "event", 1, None, early.clone(), key.clone()),
            "monitoring_snapshot_figure_source_check",
            "an unknown source",
        ),
        (
            write(
                "voter_turnout",
                "Post=e1",
                1,
                None,
                early.clone(),
                key.clone(),
            ),
            "monitoring_snapshot_figure_scope_key_check",
            "a scope key out of form",
        ),
        (
            write(
                "voter_turnout",
                "country=ES&post=e1",
                1,
                None,
                early.clone(),
                key.clone(),
            ),
            "monitoring_snapshot_figure_scope_key_check",
            "scope parts out of order",
        ),
        (
            write(
                "voter_turnout",
                "event",
                1,
                None,
                digest("never stored"),
                key.clone(),
            ),
            "monitoring_snapshot_figure_payload_exists",
            "figures never stored",
        ),
        (
            write(
                "voter_turnout",
                "event",
                1,
                None,
                early.clone(),
                set_key(&[]),
            ),
            "monitoring_snapshot_figure_set_is_recorded",
            "a set nobody recorded",
        ),
        (
            write(
                "voter_turnout",
                "event",
                1,
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
    // A pass at revision 20 closes the row and opens the next, in that order.
    let close = "UPDATE sequent_backend.monitoring_snapshot_figure SET to_revision = $3
                 WHERE election_event_id = $1 AND scope_key = 'post=e1' AND from_revision = $2";
    assert_eq!(
        attempt(&mut tx, close, &[&s.event, &10_i64, &20_i64]).await,
        Ok(1)
    );
    assert_eq!(
        attempt(
            &mut tx,
            figure,
            &[
                &s.tenant,
                &s.event,
                &"voter_turnout",
                &key,
                &"post=e1",
                &20_i64,
                &open,
                &late
            ]
        )
        .await,
        Ok(1)
    );
    for (change, why) in [
        ("to_revision = NULL", "a closed row reopened"),
        ("to_revision = 25", "a closed row closed again later"),
        ("payload_sha256 = payload_sha256", "a row rewritten"),
    ] {
        refused_by(
            attempt(
                &mut tx,
                &format!(
                    "UPDATE sequent_backend.monitoring_snapshot_figure SET {change}
                     WHERE election_event_id = $1 AND from_revision = 10"
                ),
                &[&s.event],
            )
            .await,
            "monitoring_snapshot_figure_only_closes",
            why,
        );
    }
    assert_eq!(
        attempt(&mut tx, close, &[&s.event, &20_i64, &30_i64]).await,
        Ok(1)
    );
    refused_by(
        attempt(&mut tx, close, &[&s.event, &20_i64, &31_i64]).await,
        "monitoring_snapshot_figure_only_closes",
        "closed twice",
    );

    // Each revision reads the payload its range holds.
    let read = "SELECT payload_sha256 FROM sequent_backend.monitoring_snapshot_figure
                WHERE tenant_id = $1 AND election_event_id = $2 AND source = 'voter_turnout'
                  AND election_set_key = $3 AND scope_key = 'post=e1' AND from_revision <= $4
                  AND (to_revision IS NULL OR to_revision > $4)
                ORDER BY from_revision DESC LIMIT 1";
    for (revision, expected) in [
        (10_i64, Some(&early)),
        (19, Some(&early)),
        (20, Some(&late)),
        (29, Some(&late)),
        (30, None),
        (9, None),
    ] {
        let shown: Option<Vec<u8>> = tx
            .query_opt(read, &[&s.tenant, &s.event, &key, &revision])
            .await
            .unwrap()
            .map(|row| row.get(0));
        assert_eq!(shown.as_ref(), expected, "revision {revision}");
    }

    // Pruning cannot take what a figure still names.
    for (statement, constraint) in [
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
    // Keeping revisions from 25: the rows closed by then go, then the
    // payloads no row names any more.
    let freed: Vec<Vec<u8>> = tx
        .query(
            "DELETE FROM sequent_backend.monitoring_snapshot_figure
             WHERE tenant_id = $1 AND election_event_id = $2 AND to_revision <= 25
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
        let (key, figures) = (set_key(&Vec::<Uuid>::new()), digest("figures"));
        let statements: [(&str, &[&(dyn ToSql + Sync)]); 3] = [
            (
                "INSERT INTO sequent_backend.monitoring_election_set
                     (tenant_id, election_event_id, election_set_key, election_ids)
                 VALUES ($1, $2, $3, '{}')",
                &[&s.tenant, &s.event, &key],
            ),
            (
                "INSERT INTO sequent_backend.monitoring_snapshot_payload
                     (tenant_id, election_event_id, sha256, payload)
                 VALUES ($1, $2, $3, '{}')",
                &[&s.tenant, &s.event, &figures],
            ),
            (
                "INSERT INTO sequent_backend.monitoring_snapshot_figure
                     (tenant_id, election_event_id, source, election_set_key, scope_key,
                      from_revision, to_revision, payload_sha256)
                 VALUES ($1, $2, 'voter_turnout', $3, 'event', 1, 2, $4)",
                &[&s.tenant, &s.event, &key, &figures],
            ),
        ];
        for (statement, params) in statements {
            setup.execute(statement, params).await.unwrap();
        }
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
    let key = set_key(&[post]);
    let figures = digest("figures");
    let revision: i64 = tx
        .query_one(
            "INSERT INTO sequent_backend.monitoring_snapshot_run
                 (tenant_id, election_event_id, status, as_of, finished_at, settings_revision, config_generation)
             VALUES ($1, $2, 'COMPLETE', now(), now(), 1, 0) RETURNING revision",
            &[&s.tenant, &s.event],
        )
        .await
        .unwrap()
        .get(0);
    let statements: [(&str, &[&(dyn ToSql + Sync)]); 8] = [
        (
            "INSERT INTO sequent_backend.monitoring_election_set
                 (tenant_id, election_event_id, election_set_key, election_ids)
             VALUES ($1, $2, $3, ARRAY[$4::uuid])",
            &[&s.tenant, &s.event, &key, &post],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_payload (tenant_id, election_event_id, sha256, payload)
             VALUES ($1, $2, $3, '{}')",
            &[&s.tenant, &s.event, &figures],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_source
                 (tenant_id, election_event_id, revision, source, election_set_key, producer_status)
             VALUES ($1, $2, $3, 'voter_turnout', $4, 'CONNECTED')",
            &[&s.tenant, &s.event, &revision, &key],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_figure
                 (tenant_id, election_event_id, source, election_set_key, scope_key, from_revision, payload_sha256)
             VALUES ($1, $2, 'voter_turnout', $3, 'event', $4, $5)",
            &[&s.tenant, &s.event, &key, &revision, &figures],
        ),
        (
            "UPDATE sequent_backend.monitoring_event SET live_snapshot_revision = $3
             WHERE tenant_id = $1 AND election_event_id = $2",
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
