// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tables behind the configurable monitoring dashboards: what they
//! refuse, and that they go with their election event.

#[path = "support/schema.rs"]
mod schema;

use sequent_core::monitoring::config::ConfigKind;
use sequent_core::monitoring::sources::DataSourceId;
use strum::IntoEnumIterator;
use tokio_postgres::types::ToSql;
use tokio_postgres::Transaction;
use uuid::Uuid;

const TABLES: [&str; 9] = [
    "monitoring_event",
    "monitoring_config",
    "monitoring_config_head",
    "monitoring_snapshot_run",
    "monitoring_snapshot_manifest",
    "monitoring_snapshot_payload",
    "monitoring_voter",
    "monitoring_login_counter",
    "monitoring_login_counter_receipt",
];

const HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Copy)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
}

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

async fn scope(tx: &Transaction<'_>, seed: u32) -> Scope {
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
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
         VALUES ($1, $2)",
        &[&tenant, &event],
    )
    .await
    .unwrap();
    Scope { tenant, event }
}

/// Runs `sql` in a savepoint and says whether the database took it; a
/// refusal leaves the transaction usable. Only an integrity constraint
/// (SQLSTATE class 23) counts as a refusal, so a mistyped statement fails
/// the test instead of passing for a rule it never reached.
async fn accepted(tx: &mut Transaction<'_>, sql: &str, params: &[&(dyn ToSql + Sync)]) -> bool {
    let savepoint = tx.savepoint("attempt").await.unwrap();
    let result = savepoint.execute(sql, params).await;
    match result {
        Ok(_) => {
            savepoint.commit().await.unwrap();
            true
        }
        Err(error) => {
            let code = error.code().map(|code| code.code().to_owned());
            assert!(
                code.as_deref().is_some_and(|code| code.starts_with("23")),
                "not a constraint refusal: {error:?}"
            );
            savepoint.rollback().await.unwrap();
            false
        }
    }
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

const CONFIG: &str = "INSERT INTO sequent_backend.monitoring_config
    (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256, origin)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";

#[tokio::test]
async fn the_migrations_build_the_monitoring_tables() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    for table in TABLES {
        let exists: bool = tx
            .query_one(
                "SELECT to_regclass('sequent_backend.' || $1) IS NOT NULL",
                &[&table],
            )
            .await
            .unwrap()
            .get(0);
        assert!(exists, "{table}");
    }
}

#[tokio::test]
async fn an_event_is_on_the_standard_dashboard_until_it_is_configured() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let scope = scope(&tx, line!()).await;
    let row = tx
        .query_one(
            "SELECT dashboard_mode, config_generation, watermarks
             FROM sequent_backend.monitoring_event
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&scope.tenant, &scope.event],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "LEGACY");
    assert_eq!(row.get::<_, i64>(1), 0);
    assert_eq!(row.get::<_, serde_json::Value>(2), serde_json::json!({}));
    let update = "UPDATE sequent_backend.monitoring_event SET dashboard_mode = $3
                  WHERE tenant_id = $1 AND election_event_id = $2";
    for (mode, taken) in [("CONFIGURED", true), ("SOMETIMES", false)] {
        assert_eq!(
            accepted(&mut tx, update, &[&scope.tenant, &scope.event, &mode]).await,
            taken,
            "{mode}"
        );
    }
    tx.execute(
        "UPDATE sequent_backend.monitoring_event SET updated_at = '2000-01-01T00:00:00Z'
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&scope.tenant, &scope.event],
    )
    .await
    .unwrap();
    let touched: bool = tx
        .query_one(
            "SELECT updated_at = now() FROM sequent_backend.monitoring_event
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&scope.tenant, &scope.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(touched, "updated_at follows every change");
    // A preset is named with its version, or not at all.
    let preset = "UPDATE sequent_backend.monitoring_event SET preset_id = $3, preset_version = $4
                  WHERE tenant_id = $1 AND election_event_id = $2";
    let none: Option<i32> = None;
    assert!(
        !accepted(
            &mut tx,
            preset,
            &[&scope.tenant, &scope.event, &"comelec", &none]
        )
        .await
    );
    assert!(
        accepted(
            &mut tx,
            preset,
            &[&scope.tenant, &scope.event, &"comelec", &Some(1)]
        )
        .await
    );
}

#[tokio::test]
async fn a_monitoring_row_belongs_to_an_event_of_its_own_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let mine = scope(&tx, line!()).await;
    let other = scope(&tx, line!() + 1_000_000).await;
    tx.execute(
        "DELETE FROM sequent_backend.monitoring_event WHERE election_event_id = $1",
        &[&other.event],
    )
    .await
    .unwrap();
    let insert = "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
                  VALUES ($1, $2)";
    assert!(!accepted(&mut tx, insert, &[&mine.tenant, &other.event]).await);
    assert!(accepted(&mut tx, insert, &[&other.tenant, &other.event]).await);
}

#[tokio::test]
async fn a_revision_is_a_change_of_a_known_kind_and_origin() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let yaml = Some("id: w\n");
    let no_yaml: Option<&str> = None;
    // Every kind the configuration has.
    for (revision, kind) in (1_i32..).zip(ConfigKind::iter()) {
        let kind = kind.to_string();
        assert!(
            accepted(
                &mut tx,
                CONFIG,
                &[&s.tenant, &s.event, &kind, &"k", &revision, &"UPSERT", &yaml, &HASH, &"PRESET"],
            )
            .await,
            "{kind}"
        );
    }
    for (kind, change, yaml, sha, origin, why) in [
        ("report", "UPSERT", yaml, HASH, "EDITOR", "unknown kind"),
        ("widget", "PATCH", yaml, HASH, "EDITOR", "unknown change"),
        ("widget", "UPSERT", yaml, HASH, "IMPORT", "unknown origin"),
        (
            "widget",
            "UPSERT",
            no_yaml,
            HASH,
            "EDITOR",
            "an upsert without its document",
        ),
        (
            "widget",
            "DELETE",
            yaml,
            HASH,
            "EDITOR",
            "a deletion with a document",
        ),
        (
            "widget",
            "UPSERT",
            yaml,
            "abc",
            "EDITOR",
            "a digest that is not sha-256",
        ),
    ] {
        assert!(
            !accepted(
                &mut tx,
                CONFIG,
                &[&s.tenant, &s.event, &kind, &"x", &1_i32, &change, &yaml, &sha, &origin],
            )
            .await,
            "{why}"
        );
    }
    let no_sha: Option<&str> = None;
    assert!(
        accepted(
            &mut tx,
            CONFIG,
            &[
                &s.tenant, &s.event, &"widget", &"gone", &1_i32, &"DELETE", &no_yaml, &no_sha,
                &"EDITOR"
            ],
        )
        .await,
        "a deletion has no document and no digest"
    );
    // One revision number per document.
    let duplicate: [&(dyn ToSql + Sync); 9] = [
        &s.tenant, &s.event, &"widget", &"twice", &1_i32, &"UPSERT", &yaml, &HASH, &"EDITOR",
    ];
    assert!(accepted(&mut tx, CONFIG, &duplicate).await);
    assert!(
        !accepted(&mut tx, CONFIG, &duplicate).await,
        "a revision number used twice"
    );
    for key in ["Turnout", "-lead", "has space", ""] {
        assert!(
            !accepted(
                &mut tx,
                CONFIG,
                &[
                    &s.tenant, &s.event, &"widget", &key, &1_i32, &"UPSERT", &yaml, &HASH,
                    &"EDITOR"
                ],
            )
            .await,
            "key {key:?}"
        );
    }
    assert!(
        !accepted(
            &mut tx,
            CONFIG,
            &[&s.tenant, &s.event, &"widget", &"zero", &0_i32, &"UPSERT", &yaml, &HASH, &"EDITOR"],
        )
        .await,
        "revisions count from 1"
    );
}

#[tokio::test]
async fn a_save_moves_the_head_only_from_the_revision_it_was_based_on() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let yaml = Some("id: w\n");
    for revision in [1_i32, 2] {
        tx.execute(
            CONFIG,
            &[
                &s.tenant, &s.event, &"widget", &"w", &revision, &"UPSERT", &yaml, &HASH, &"EDITOR",
            ],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_config_head
             (tenant_id, election_event_id, kind, key, revision)
         VALUES ($1, $2, 'widget', 'w', 1)",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    let advance = "UPDATE sequent_backend.monitoring_config_head SET revision = $4
                   WHERE tenant_id = $1 AND election_event_id = $2
                     AND kind = 'widget' AND key = 'w' AND revision = $3";
    let moved = tx
        .execute(advance, &[&s.tenant, &s.event, &1_i32, &2_i32])
        .await
        .unwrap();
    assert_eq!(moved, 1);
    // A second editor who also read revision 1 moves nothing.
    let stale = tx
        .execute(advance, &[&s.tenant, &s.event, &1_i32, &2_i32])
        .await
        .unwrap();
    assert_eq!(stale, 0);
    // The head names a revision that exists.
    assert!(!accepted(&mut tx, advance, &[&s.tenant, &s.event, &2_i32, &9_i32]).await);
}

#[tokio::test]
async fn a_snapshot_records_every_source_and_refuses_what_it_does_not_know() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let run = "INSERT INTO sequent_backend.monitoring_snapshot_run
                   (tenant_id, election_event_id, revision, status)
               VALUES ($1, $2, $3, $4)";
    assert!(accepted(&mut tx, run, &[&s.tenant, &s.event, &1_i64, &"RUNNING"]).await);
    assert!(!accepted(&mut tx, run, &[&s.tenant, &s.event, &2_i64, &"PAUSED"]).await);
    let complete = "UPDATE sequent_backend.monitoring_snapshot_run SET status = 'COMPLETE'
                    WHERE tenant_id = $1 AND election_event_id = $2 AND revision = 1";
    assert!(
        !accepted(&mut tx, complete, &[&s.tenant, &s.event]).await,
        "a complete run says what time its figures are from"
    );

    let manifest = "INSERT INTO sequent_backend.monitoring_snapshot_manifest
                        (tenant_id, election_event_id, revision, source, election_set_key,
                         producer_status, reason, scopes)
                    VALUES ($1, $2, 1, $3, '0123456789abcdef', $4, $5, $6)";
    let scopes = serde_json::json!({"": HASH});
    let no_reason: Option<&str> = None;
    for source in DataSourceId::iter() {
        let source = source.to_string();
        assert!(
            accepted(
                &mut tx,
                manifest,
                &[
                    &s.tenant,
                    &s.event,
                    &source,
                    &"CONNECTED",
                    &no_reason,
                    &scopes
                ]
            )
            .await,
            "{source}"
        );
    }
    let empty = serde_json::json!({});
    for (source, status, reason, scopes, why) in [
        (
            "weather",
            "CONNECTED",
            no_reason,
            &empty,
            "an unknown source",
        ),
        (
            "helpdesk",
            "BROKEN",
            no_reason,
            &empty,
            "an unknown producer status",
        ),
    ] {
        tx.execute(
            "DELETE FROM sequent_backend.monitoring_snapshot_manifest WHERE source = 'helpdesk' AND election_event_id = $1",
            &[&s.event],
        )
        .await
        .unwrap();
        assert!(
            !accepted(
                &mut tx,
                manifest,
                &[&s.tenant, &s.event, &source, &status, &reason, scopes]
            )
            .await,
            "{why}"
        );
    }
    assert!(
        !accepted(
            &mut tx,
            manifest,
            &[
                &s.tenant,
                &s.event,
                &"helpdesk",
                &"NOT_CONNECTED",
                &no_reason,
                &empty
            ]
        )
        .await,
        "a source that is not connected says why"
    );
    assert!(
        accepted(
            &mut tx,
            manifest,
            &[
                &s.tenant,
                &s.event,
                &"helpdesk",
                &"NOT_CONNECTED",
                &Some("No producer yet"),
                &empty
            ]
        )
        .await
    );

    let other_set = "INSERT INTO sequent_backend.monitoring_snapshot_manifest
                         (tenant_id, election_event_id, revision, source, election_set_key, producer_status)
                     VALUES ($1, $2, 1, 'voter_turnout', $3, 'CONNECTED')";
    for key in ["all", "0123456789ABCDEF", "0123456789abcde"] {
        assert!(
            !accepted(&mut tx, other_set, &[&s.tenant, &s.event, &key]).await,
            "set key {key:?}"
        );
    }

    // The run viewers are shown cannot be pruned, nor a missing run shown.
    let show = "UPDATE sequent_backend.monitoring_event SET live_snapshot_revision = $3
                WHERE tenant_id = $1 AND election_event_id = $2";
    assert!(!accepted(&mut tx, show, &[&s.tenant, &s.event, &5_i64]).await);
    tx.execute(
        "UPDATE sequent_backend.monitoring_snapshot_run
         SET status = 'COMPLETE', as_of = now(), finished_at = now()
         WHERE tenant_id = $1 AND election_event_id = $2 AND revision = 1",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    assert!(accepted(&mut tx, show, &[&s.tenant, &s.event, &1_i64]).await);
    let prune = "DELETE FROM sequent_backend.monitoring_snapshot_run
                 WHERE tenant_id = $1 AND election_event_id = $2 AND revision = 1";
    assert!(!accepted(&mut tx, prune, &[&s.tenant, &s.event]).await);
    assert!(
        !accepted(
            &mut tx,
            "UPDATE sequent_backend.monitoring_snapshot_run SET status = 'FAILED'
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = 1",
            &[&s.tenant, &s.event],
        )
        .await,
        "a failed run says what went wrong"
    );

    let payload = "INSERT INTO sequent_backend.monitoring_snapshot_payload
                       (tenant_id, election_event_id, sha256, payload)
                   VALUES ($1, $2, $3, '{}'::jsonb)";
    assert!(accepted(&mut tx, payload, &[&s.tenant, &s.event, &HASH]).await);
    assert!(
        !accepted(&mut tx, payload, &[&s.tenant, &s.event, &HASH]).await,
        "content-addressed: stored once"
    );
    assert!(!accepted(&mut tx, payload, &[&s.tenant, &s.event, &"not-a-digest"]).await);
}

#[tokio::test]
async fn login_attempts_are_counted_in_quarter_hours_once_per_delivery() {
    let mut client = schema::pool().await.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let count = "INSERT INTO sequent_backend.monitoring_login_counter
                     (tenant_id, election_event_id, bucket_start, event_type, registration, area_id, attempts)
                 VALUES ($1, $2, $3::text::timestamptz, $4, $5, $6, 1)";
    let no_area: Option<Uuid> = None;
    assert!(
        accepted(
            &mut tx,
            count,
            &[
                &s.tenant,
                &s.event,
                &"2026-05-04T10:15:00Z",
                &"LOGIN",
                &"REGISTERED",
                &no_area
            ]
        )
        .await
    );
    for (bucket, registration, why) in [
        (
            "2026-05-04T10:20:00Z",
            "REGISTERED",
            "a bucket off the quarter hour",
        ),
        (
            "2026-05-04T10:15:30Z",
            "REGISTERED",
            "a bucket with seconds",
        ),
        ("2026-05-04T10:30:00Z", "GUEST", "an unknown registration"),
        (
            "2026-05-04T10:15:00Z",
            "REGISTERED",
            "the same bucket twice, even without an area",
        ),
    ] {
        assert!(
            !accepted(
                &mut tx,
                count,
                &[
                    &s.tenant,
                    &s.event,
                    &bucket,
                    &"LOGIN",
                    &registration,
                    &no_area
                ]
            )
            .await,
            "{why}"
        );
    }
    // An offset zone's quarter hours are UTC quarter hours too.
    assert!(
        accepted(
            &mut tx,
            count,
            &[
                &s.tenant,
                &s.event,
                &"2026-05-04T18:45:00+08:00",
                &"LOGIN",
                &"UNREGISTERED",
                &no_area
            ]
        )
        .await
    );

    // The counter adds up on the same key, a missing area included.
    let add = "INSERT INTO sequent_backend.monitoring_login_counter
                   (tenant_id, election_event_id, bucket_start, event_type, registration, area_id, attempts)
               VALUES ($1, $2, '2026-05-04T10:15:00Z', 'LOGIN', 'REGISTERED', $3, 1)
               ON CONFLICT (tenant_id, election_event_id, bucket_start, event_type, registration, area_id)
               DO UPDATE SET attempts = monitoring_login_counter.attempts + 1";
    for _ in 0..2 {
        tx.execute(add, &[&s.tenant, &s.event, &no_area])
            .await
            .unwrap();
    }
    let attempts: Vec<i64> = tx
        .query(
            "SELECT attempts FROM sequent_backend.monitoring_login_counter
             WHERE election_event_id = $1 AND bucket_start = '2026-05-04T10:15:00Z'",
            &[&s.event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(attempts, [3]);

    let receipt = "INSERT INTO sequent_backend.monitoring_login_counter_receipt
                       (delivery_id, tenant_id, election_event_id)
                   VALUES ($1, $2, $3) ON CONFLICT DO NOTHING";
    let first = tx
        .execute(receipt, &[&"delivery-1", &s.tenant, &s.event])
        .await
        .unwrap();
    let again = tx
        .execute(receipt, &[&"delivery-1", &s.tenant, &s.event])
        .await
        .unwrap();
    assert_eq!((first, again), (1, 0));
}

#[tokio::test]
async fn monitoring_rows_go_with_their_event_and_voters_with_their_election() {
    use windmill::postgres::election_event::delete_election_event;
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx, line!()).await;
    let election = id(line!(), 9);
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id) VALUES ($1, $2, $3)",
        &[&election, &s.tenant, &s.event],
    )
    .await
    .unwrap();
    let voter = "INSERT INTO sequent_backend.monitoring_voter
                     (tenant_id, election_event_id, election_id, voter_id, attributes_hash, settings_revision)
                 VALUES ($1, $2, $3, $4, 'h', 1)";
    tx.execute(voter, &[&s.tenant, &s.event, &election, &"v1"])
        .await
        .unwrap();
    tx.execute(
        CONFIG,
        &[
            &s.tenant,
            &s.event,
            &"widget",
            &"w",
            &1_i32,
            &"UPSERT",
            &Some("id: w\n"),
            &HASH,
            &"PRESET",
        ],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_config_head (tenant_id, election_event_id, kind, key, revision)
         VALUES ($1, $2, 'widget', 'w', 1)",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();
    // A second Post whose voter stays until the event goes.
    let kept = id(line!(), 10);
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id) VALUES ($1, $2, $3)",
        &[&kept, &s.tenant, &s.event],
    )
    .await
    .unwrap();
    tx.execute(voter, &[&s.tenant, &s.event, &kept, &"v1"])
        .await
        .unwrap();
    for statement in [
        "INSERT INTO sequent_backend.monitoring_snapshot_run
             (tenant_id, election_event_id, revision, status, as_of, finished_at)
         VALUES ($1, $2, 1, 'COMPLETE', now(), now())",
        "INSERT INTO sequent_backend.monitoring_snapshot_manifest
             (tenant_id, election_event_id, revision, source, election_set_key, producer_status)
         VALUES ($1, $2, 1, 'voter_turnout', '0123456789abcdef', 'CONNECTED')",
        "INSERT INTO sequent_backend.monitoring_snapshot_payload
             (tenant_id, election_event_id, sha256, payload)
         VALUES ($1, $2, repeat('0', 64), '{}')",
        // The shown run references back to its event.
        "UPDATE sequent_backend.monitoring_event SET live_snapshot_revision = 1
         WHERE tenant_id = $1 AND election_event_id = $2",
        "INSERT INTO sequent_backend.monitoring_login_counter
             (tenant_id, election_event_id, bucket_start, event_type, registration, attempts)
         VALUES ($1, $2, '2026-05-04T10:15:00Z', 'LOGIN', 'UNREGISTERED', 1)",
    ] {
        tx.execute(statement, &[&s.tenant, &s.event]).await.unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_login_counter_receipt (delivery_id, tenant_id, election_event_id)
         VALUES ('d', $1, $2)",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap();

    // An election that is removed takes its voters' figures with it.
    tx.execute(
        "DELETE FROM sequent_backend.election WHERE id = $1",
        &[&election],
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
    assert_eq!(voters, 1);

    // Every table has a row before the event goes, and none after.
    for (table, rows) in row_counts(&tx, s.event).await {
        assert!(
            rows > 0,
            "{table} is empty, so its deletion would prove nothing"
        );
    }
    delete_election_event(&tx, &s.tenant.to_string(), &s.event.to_string())
        .await
        .unwrap();
    for (table, rows) in row_counts(&tx, s.event).await {
        assert_eq!(rows, 0, "{table}");
    }
}
