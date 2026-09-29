// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tables behind the configurable monitoring dashboards: what they
//! refuse, and that they go with their election event.

#[path = "support/schema.rs"]
mod schema;

use sequent_core::monitoring::config::ConfigKind;
use sequent_core::monitoring::sources::DataSourceId;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio_postgres::types::ToSql;
use tokio_postgres::Transaction;
use uuid::Uuid;
use windmill::postgres::election_event::delete_election_event;

const TABLES: [&str; 12] = [
    "monitoring_event",
    "monitoring_config",
    "monitoring_config_head",
    "monitoring_election_set",
    "monitoring_snapshot_run",
    "monitoring_snapshot_source",
    "monitoring_snapshot_manifest",
    "monitoring_snapshot_manifest_scope",
    "monitoring_snapshot_payload",
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

/// The key of a set of elections, as the snapshot job derives it.
fn set_key(ids: &[Uuid]) -> String {
    let mut ids = ids.to_vec();
    ids.sort();
    let joined: Vec<String> = ids.iter().map(Uuid::to_string).collect();
    sha256(&joined.join(","))[..16].to_owned()
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
/// refused it. Anything other than an integrity constraint (SQLSTATE class
/// 23) fails the test, so a mistyped statement cannot pass as a refusal.
async fn attempt(
    tx: &mut Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64, String> {
    let savepoint = tx.savepoint("attempt").await.unwrap();
    // Deferred constraints are checked here too, not at a commit that never
    // comes.
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

async fn row_counts(tx: &Transaction<'_>, event: Uuid) -> Vec<(&'static str, i64)> {
    let mut counts = Vec::new();
    for table in TABLES {
        let rows: i64 = tx
            .query_one(
                &format!(
                    "SELECT count(*) FROM sequent_backend.{table} WHERE election_event_id = $1"
                ),
                &[&event],
            )
            .await
            .unwrap()
            .get(0);
        counts.push((table, rows));
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
    preset: Option<(&'a str, i32)>,
    author: Option<&'a str>,
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
            preset: None,
            author: Some("admin-1"),
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

    async fn write(&self, tx: &mut Transaction<'_>, s: Scope) -> Result<u64, String> {
        let (preset_id, preset_version) = self.preset.unzip();
        attempt(
            tx,
            "INSERT INTO sequent_backend.monitoring_config
                 (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
                  origin, preset_id, preset_version, author_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            &[
                &s.tenant,
                &s.event,
                &self.kind,
                &self.key,
                &self.revision,
                &self.change,
                &self.yaml,
                &self.sha256,
                &self.origin,
                &preset_id,
                &preset_version,
                &self.author,
            ],
        )
        .await
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

    tx.execute(
        "UPDATE sequent_backend.monitoring_event SET updated_at = '2000-01-01T00:00:00Z'
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    let touched: bool = tx
        .query_one(
            "SELECT updated_at = now() FROM sequent_backend.monitoring_event
             WHERE tenant_id = $1 AND election_event_id = $2",
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

    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
             VALUES ($1, $2)",
            &[&mine.tenant, &other.event],
        )
        .await,
        "monitoring_event_of_its_tenant",
        "another tenant's event",
    );
    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_login_counter
                 (tenant_id, election_event_id, bucket_start, event_type, registration, attempts)
             VALUES ($1, $2, '2026-05-04T10:15:00Z', 'LOGIN', 'REGISTERED', 1)",
            &[&mine.tenant, &other.event],
        )
        .await,
        "monitoring_login_counter_of_its_tenant",
        "sign-ins counted for another tenant's event",
    );
    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_login_counter_receipt
                 (delivery_id, tenant_id, election_event_id)
             VALUES (repeat('a', 64), $1, $2)",
            &[&mine.tenant, &other.event],
        )
        .await,
        "monitoring_login_counter_receipt_of_its_tenant",
        "a receipt for another tenant's event",
    );
    refused_by(
        attempt(
            &mut tx,
            "INSERT INTO sequent_backend.monitoring_voter
                 (tenant_id, election_event_id, election_id, voter_id, attributes_hash, settings_revision)
             VALUES ($1, $2, $3, 'v1', 'h', 1)",
            &[&mine.tenant, &mine.event, &theirs],
        )
        .await,
        "monitoring_voter_of_an_election_of_the_event",
        "a voter of another event's election",
    );
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
        assert_eq!(revision.write(&mut tx, s).await, Ok(1), "{kind}");
    }
    let reset = Revision {
        key: "from-preset",
        origin: "PRESET",
        preset: Some(("comelec", 1)),
        ..Revision::edit("-", 1, "id: from-preset\n")
    };
    assert_eq!(reset.write(&mut tx, s).await, Ok(1));
    assert_eq!(Revision::removal("gone", 2).write(&mut tx, s).await, Ok(1));
    let multibyte = "title: Participación · 投票\n";
    assert_eq!(
        Revision::edit("unicode", 1, multibyte)
            .write(&mut tx, s)
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
                preset: Some(("comelec", 1)),
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_preset_follows_origin",
            "an editor revision that names a preset",
        ),
        (
            Revision {
                author: None,
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_editor_is_named",
            "an edit without its author",
        ),
        (
            Revision {
                author: Some(""),
                ..Revision::edit("x", 1, yaml)
            },
            "monitoring_config_editor_is_named",
            "an edit by a blank author",
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
        refused_by(revision.write(&mut tx, s).await, constraint, why);
    }
    for key in ["Turnout", "-lead", "has space", "", &"a".repeat(65)] {
        refused_by(
            Revision::edit(key, 1, yaml).write(&mut tx, s).await,
            "monitoring_config_key_check",
            key,
        );
    }
}

#[tokio::test]
async fn a_revision_is_never_changed_or_removed_but_with_its_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    Revision::edit("w", 1, "id: w\n")
        .write(&mut tx, s)
        .await
        .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_config_head
             (tenant_id, election_event_id, kind, key, revision)
         VALUES ($1, $2, 'widget', 'w', 1)",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    for (statement, why) in [
        (
            "UPDATE sequent_backend.monitoring_config SET yaml = 'id: x' WHERE election_event_id = $1",
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
    let heads: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_config_head WHERE election_event_id = $1",
            &[&s.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(heads, 1);

    // The head's updated_at follows each move.
    Revision::edit("w", 2, "id: w\ntitle: T\n")
        .write(&mut tx, s)
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.monitoring_config_head SET updated_at = '2000-01-01T00:00:00Z'
         WHERE election_event_id = $1",
        &[&s.event],
    )
    .await
    .unwrap();
    let moved: bool = tx
        .query_one(
            "UPDATE sequent_backend.monitoring_config_head SET revision = 2
             WHERE election_event_id = $1 RETURNING updated_at = now()",
            &[&s.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(moved);
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
        let setup = first.transaction().await.unwrap();
        let s = scope(&setup, line!()).await;
        setup
            .execute(
                "INSERT INTO sequent_backend.monitoring_config
                     (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
                      origin, author_id)
                 VALUES ($1, $2, 'widget', 'w', 1, 'UPSERT', 'id: w', $3, 'EDITOR', 'admin-1')",
                &[&s.tenant, &s.event, &sha256("id: w")],
            )
            .await
            .unwrap();
        setup
            .execute(
                "INSERT INTO sequent_backend.monitoring_config_head
                     (tenant_id, election_event_id, kind, key, revision)
                 VALUES ($1, $2, 'widget', 'w', 1)",
                &[&s.tenant, &s.event],
            )
            .await
            .unwrap();
        setup.commit().await.unwrap();
        s
    };
    let advance = "UPDATE sequent_backend.monitoring_config_head SET revision = 2
                   WHERE tenant_id = $1 AND election_event_id = $2
                     AND kind = 'widget' AND key = 'w' AND revision = 1";
    let revise = "INSERT INTO sequent_backend.monitoring_config
                      (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
                       origin, author_id)
                  VALUES ($1, $2, 'widget', $3, $4, 'UPSERT', $5, $6, 'EDITOR', $7)";
    let create = "INSERT INTO sequent_backend.monitoring_config_head
                      (tenant_id, election_event_id, kind, key, revision)
                  VALUES ($1, $2, 'widget', 'new', 1) ON CONFLICT DO NOTHING";

    for (statement, key, revision) in [(advance, "w", 2_i32), (create, "new", 1)] {
        let a = first.transaction().await.unwrap();
        let b = second.transaction().await.unwrap();
        // The head moves first; the revision it names follows.
        assert_eq!(
            a.execute(statement, &[&s.tenant, &s.event]).await.unwrap(),
            1
        );
        let yaml = format!("id: {key}\ntitle: A\n");
        a.execute(
            revise,
            &[
                &s.tenant,
                &s.event,
                &key,
                &revision,
                &yaml,
                &sha256(&yaml),
                &"admin-a",
            ],
        )
        .await
        .unwrap();
        let params: [&(dyn ToSql + Sync); 2] = [&s.tenant, &s.event];
        let moved = {
            let waiting = b.execute(statement, &params);
            tokio::pin!(waiting);
            assert!(
                tokio::time::timeout(Duration::from_millis(300), &mut waiting)
                    .await
                    .is_err(),
                "the second save waits for the first"
            );
            a.commit().await.unwrap();
            waiting.await.unwrap()
        };
        assert_eq!(moved, 0, "and then moves nothing: a conflict");
        b.rollback().await.unwrap();
    }

    // A head that names a revision nobody wrote is refused at commit.
    let lost = first.transaction().await.unwrap();
    lost.execute(
        "UPDATE sequent_backend.monitoring_config_head SET revision = 3
         WHERE tenant_id = $1 AND election_event_id = $2 AND key = 'w'",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    let error = lost.commit().await.unwrap_err();
    assert_eq!(refusal(&error), "monitoring_config_head_names_a_revision");

    // Deleting the event removes the committed rows, deferred check included.
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
async fn viewers_are_shown_only_a_complete_run_and_it_is_kept() {
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
    let (revision, generation) = (Some(1_i32), Some(0_i64));
    let (no_revision, no_generation): (Option<i32>, Option<i64>) = (None, None);
    let cases: [(
        &str,
        Option<&str>,
        Option<&str>,
        Option<&str>,
        Option<i32>,
        Option<i64>,
        &str,
    ); 7] = [
        (
            "PAUSED",
            at,
            none,
            at,
            revision,
            generation,
            "monitoring_snapshot_run_status_check",
        ),
        (
            "RUNNING",
            at,
            none,
            none,
            no_revision,
            no_generation,
            "monitoring_snapshot_run_running_is_open",
        ),
        (
            "RUNNING",
            none,
            Some("x"),
            none,
            no_revision,
            no_generation,
            "monitoring_snapshot_run_running_is_open",
        ),
        (
            "COMPLETE",
            at,
            none,
            none,
            revision,
            generation,
            "monitoring_snapshot_run_complete_is_counted",
        ),
        (
            "COMPLETE",
            at,
            none,
            at,
            no_revision,
            generation,
            "monitoring_snapshot_run_complete_is_counted",
        ),
        (
            "COMPLETE",
            at,
            none,
            at,
            revision,
            no_generation,
            "monitoring_snapshot_run_complete_is_counted",
        ),
        (
            "FAILED",
            at,
            Some(""),
            none,
            no_revision,
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
    for revision in [first, second] {
        refused_by(
            attempt(&mut tx, show, &[&s.tenant, &s.event, &revision]).await,
            "monitoring_event_shows_a_complete_run",
            "a running run, or another event's",
        );
    }
    assert_eq!(
        attempt(
            &mut tx,
            run,
            &[
                &s.tenant,
                &s.event,
                &first,
                &"COMPLETE",
                &at,
                &none,
                &at,
                &revision,
                &generation
            ],
        )
        .await,
        Ok(1)
    );
    assert_eq!(
        attempt(&mut tx, show, &[&s.tenant, &s.event, &first]).await,
        Ok(1)
    );
    refused_by(
        attempt(
            &mut tx,
            run,
            &[
                &s.tenant,
                &s.event,
                &first,
                &"FAILED",
                &at,
                &Some("late"),
                &at,
                &revision,
                &generation,
            ],
        )
        .await,
        "monitoring_event_shows_a_complete_run",
        "the shown run marked failed",
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
async fn a_snapshot_names_its_figures_by_content_and_keeps_what_it_names() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let (a, b) = (id(line!(), 3), id(line!(), 4));

    let set = "INSERT INTO sequent_backend.monitoring_election_set
                   (tenant_id, election_event_id, election_set_key, election_ids)
               VALUES ($1, $2, $3, $4)";
    let key = set_key(&[b, a]);
    for (ids, constraint, why) in [
        (
            vec![a, b],
            "monitoring_election_set_key_is_of_ids",
            "a key of other elections",
        ),
        (
            vec![b, a],
            "monitoring_election_set_ids_ascend",
            "ids out of order",
        ),
        (
            vec![a, a],
            "monitoring_election_set_ids_ascend",
            "an id twice",
        ),
        (vec![], "monitoring_election_set_ids_ascend", "no elections"),
    ] {
        let key = if why == "a key of other elections" {
            set_key(&[a])
        } else {
            key.clone()
        };
        refused_by(
            attempt(&mut tx, set, &[&s.tenant, &s.event, &key, &ids]).await,
            constraint,
            why,
        );
    }
    let with_null = "INSERT INTO sequent_backend.monitoring_election_set
                         (tenant_id, election_event_id, election_set_key, election_ids)
                     VALUES ($1, $2, $3, ARRAY[$4, NULL]::uuid[])";
    refused_by(
        attempt(&mut tx, with_null, &[&s.tenant, &s.event, &key, &a]).await,
        "monitoring_election_set_ids_ascend",
        "a missing id",
    );
    assert_eq!(
        attempt(&mut tx, set, &[&s.tenant, &s.event, &key, &vec![a, b]]).await,
        Ok(1),
        "the key the snapshot job derives"
    );

    let payload = "INSERT INTO sequent_backend.monitoring_snapshot_payload
                       (tenant_id, election_event_id, sha256, payload)
                   VALUES ($1, $2, $3, '{}'::jsonb)";
    let figures = sha256("figures");
    assert_eq!(
        attempt(&mut tx, payload, &[&s.tenant, &s.event, &figures]).await,
        Ok(1)
    );
    refused_by(
        attempt(&mut tx, payload, &[&s.tenant, &s.event, &figures]).await,
        "monitoring_snapshot_payload_pkey",
        "the same figures stored twice",
    );
    refused_by(
        attempt(&mut tx, payload, &[&s.tenant, &s.event, &"not-a-digest"]).await,
        "monitoring_snapshot_payload_sha256_check",
        "a payload not named by its digest",
    );

    let manifest = "INSERT INTO sequent_backend.monitoring_snapshot_manifest
                        (tenant_id, election_event_id, sha256, source, producer_status, reason)
                    VALUES ($1, $2, $3, $4, $5, $6)";
    let no_reason: Option<&str> = None;
    for source in DataSourceId::iter() {
        let source = source.to_string();
        assert_eq!(
            attempt(
                &mut tx,
                manifest,
                &[
                    &s.tenant,
                    &s.event,
                    &sha256(&source),
                    &source,
                    &"CONNECTED",
                    &no_reason
                ],
            )
            .await,
            Ok(1),
            "{source}"
        );
    }
    let other = sha256("other");
    for (source, status, reason, constraint, why) in [
        (
            "weather",
            "CONNECTED",
            None,
            "monitoring_snapshot_manifest_source_check",
            "an unknown source",
        ),
        (
            "helpdesk",
            "BROKEN",
            None,
            "monitoring_snapshot_manifest_producer_status_check",
            "an unknown status",
        ),
        (
            "helpdesk",
            "NOT_CONNECTED",
            None,
            "monitoring_snapshot_manifest_not_connected_says_why",
            "no reason",
        ),
        (
            "helpdesk",
            "NOT_CONNECTED",
            Some(""),
            "monitoring_snapshot_manifest_not_connected_says_why",
            "a blank reason",
        ),
        (
            "helpdesk",
            "CONNECTED",
            Some("why"),
            "monitoring_snapshot_manifest_not_connected_says_why",
            "a reason while connected",
        ),
    ] {
        refused_by(
            attempt(
                &mut tx,
                manifest,
                &[&s.tenant, &s.event, &other, &source, &status, &reason],
            )
            .await,
            constraint,
            why,
        );
    }

    let scope_row = "INSERT INTO sequent_backend.monitoring_snapshot_manifest_scope
                         (tenant_id, election_event_id, manifest_sha256, scope_key, payload_sha256)
                     VALUES ($1, $2, $3, $4, $5)";
    let turnout = sha256("voter_turnout");
    for scope_key in ["", "post=e1"] {
        assert_eq!(
            attempt(
                &mut tx,
                scope_row,
                &[&s.tenant, &s.event, &turnout, &scope_key, &figures]
            )
            .await,
            Ok(1)
        );
    }
    refused_by(
        attempt(
            &mut tx,
            scope_row,
            &[&s.tenant, &s.event, &turnout, &"region=r", &other],
        )
        .await,
        "monitoring_snapshot_manifest_scope_payload_exists",
        "a scope whose figures were never stored",
    );

    let run: i64 = tx
        .query_one(
            "INSERT INTO sequent_backend.monitoring_snapshot_run
                 (tenant_id, election_event_id, status)
             VALUES ($1, $2, 'RUNNING') RETURNING revision",
            &[&s.tenant, &s.event],
        )
        .await
        .unwrap()
        .get(0);
    let source_row = "INSERT INTO sequent_backend.monitoring_snapshot_source
                          (tenant_id, election_event_id, revision, source, election_set_key, manifest_sha256)
                      VALUES ($1, $2, $3, $4, $5, $6)";
    refused_by(
        attempt(
            &mut tx,
            source_row,
            &[&s.tenant, &s.event, &run, &"poll_status", &key, &turnout],
        )
        .await,
        "monitoring_snapshot_source_manifest_of_the_source",
        "one source's manifest filed under another",
    );
    refused_by(
        attempt(
            &mut tx,
            source_row,
            &[
                &s.tenant,
                &s.event,
                &run,
                &"voter_turnout",
                &set_key(&[a]),
                &turnout,
            ],
        )
        .await,
        "monitoring_snapshot_source_set_is_recorded",
        "figures for a set nobody recorded",
    );
    assert_eq!(
        attempt(
            &mut tx,
            source_row,
            &[&s.tenant, &s.event, &run, &"voter_turnout", &key, &turnout]
        )
        .await,
        Ok(1)
    );

    // Pruning cannot take what a run or manifest still names.
    for (statement, constraint) in [
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_payload WHERE election_event_id = $1",
            "monitoring_snapshot_manifest_scope_payload_exists",
        ),
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_manifest WHERE election_event_id = $1",
            "monitoring_snapshot_source_manifest_of_the_source",
        ),
        (
            "DELETE FROM sequent_backend.monitoring_election_set WHERE election_event_id = $1",
            "monitoring_snapshot_source_set_is_recorded",
        ),
    ] {
        refused_by(
            attempt(&mut tx, statement, &[&s.event]).await,
            constraint,
            statement,
        );
    }
    // Once the run goes, so may the rest, in that order.
    for (statement, rows) in [
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_run WHERE election_event_id = $1",
            1,
        ),
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_manifest WHERE election_event_id = $1",
            11,
        ),
        (
            "DELETE FROM sequent_backend.monitoring_snapshot_payload WHERE election_event_id = $1",
            1,
        ),
        (
            "DELETE FROM sequent_backend.monitoring_election_set WHERE election_event_id = $1",
            1,
        ),
    ] {
        assert_eq!(
            attempt(&mut tx, statement, &[&s.event]).await,
            Ok(rows),
            "{statement}"
        );
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
    assert_eq!(
        listed(&tx, "monitoring_snapshot_manifest_source_check").await,
        sources
    );
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
    let cases: [(
        &str,
        Option<&str>,
        &serde_json::Value,
        Option<&str>,
        Option<&str>,
        &str,
        i32,
        &str,
    ); 9] = [
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
    let voters: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_voter WHERE election_event_id = $1",
            &[&s.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(voters, 0, "a removed Post takes its voters' rows with it");
}

#[tokio::test]
async fn login_attempts_are_counted_in_quarter_hours_once_per_delivery() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    // Counted before the event is configured.
    let s = bare_scope(&tx, line!()).await;
    let count = "INSERT INTO sequent_backend.monitoring_login_counter
                     (tenant_id, election_event_id, bucket_start, event_type, registration, area_id, attempts)
                 VALUES ($1, $2, $3::text::timestamptz, $4, $5, $6, $7)";
    let no_area: Option<Uuid> = None;
    let bucket = "2026-05-04T10:15:00Z";
    assert_eq!(
        attempt(
            &mut tx,
            count,
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
    for (bucket, event_type, registration, attempts, constraint, why) in [
        (
            "2026-05-04T10:20:00Z",
            "LOGIN",
            "REGISTERED",
            1_i64,
            "monitoring_login_counter_bucket_start_check",
            "off the quarter hour",
        ),
        (
            "2026-05-04T10:15:30Z",
            "LOGIN",
            "REGISTERED",
            1,
            "monitoring_login_counter_bucket_start_check",
            "with seconds",
        ),
        (
            "2026-05-04T10:15:00.000001Z",
            "LOGIN",
            "REGISTERED",
            1,
            "monitoring_login_counter_bucket_start_check",
            "with microseconds",
        ),
        (
            "2026-05-04T10:30:00Z",
            "LOGIN",
            "GUEST",
            1,
            "monitoring_login_counter_registration_check",
            "an unknown registration",
        ),
        (
            "2026-05-04T10:30:00Z",
            "null",
            "REGISTERED",
            1,
            "monitoring_login_counter_event_type_check",
            "the listener's missing type",
        ),
        (
            "2026-05-04T10:30:00Z",
            "LOGIN",
            "REGISTERED",
            0,
            "monitoring_login_counter_attempts_check",
            "no attempts",
        ),
        (
            bucket,
            "LOGIN",
            "REGISTERED",
            1,
            "monitoring_login_counter_pkey",
            "the same bucket twice without an area",
        ),
    ] {
        refused_by(
            attempt(
                &mut tx,
                count,
                &[
                    &s.tenant,
                    &s.event,
                    &bucket,
                    &event_type,
                    &registration,
                    &no_area,
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
            count,
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
        .write(&mut tx, s)
        .await
        .unwrap();
    let key = set_key(&[post]);
    let figures = sha256("figures");
    let manifest = sha256("manifest");
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
    let statements: [(&str, &[&(dyn ToSql + Sync)]); 10] = [
        (
            "INSERT INTO sequent_backend.monitoring_config_head (tenant_id, election_event_id, kind, key, revision)
             VALUES ($1, $2, 'widget', 'w', 1)",
            &[&s.tenant, &s.event],
        ),
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
            "INSERT INTO sequent_backend.monitoring_snapshot_manifest
                 (tenant_id, election_event_id, sha256, source, producer_status)
             VALUES ($1, $2, $3, 'voter_turnout', 'CONNECTED')",
            &[&s.tenant, &s.event, &manifest],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_manifest_scope
                 (tenant_id, election_event_id, manifest_sha256, scope_key, payload_sha256)
             VALUES ($1, $2, $3, '', $4)",
            &[&s.tenant, &s.event, &manifest, &figures],
        ),
        (
            "INSERT INTO sequent_backend.monitoring_snapshot_source
                 (tenant_id, election_event_id, revision, source, election_set_key, manifest_sha256)
             VALUES ($1, $2, $3, 'voter_turnout', $4, $5)",
            &[&s.tenant, &s.event, &revision, &key, &manifest],
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
