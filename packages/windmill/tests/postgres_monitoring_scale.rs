// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The snapshot pipeline at the sizes an election has: thousands to tens of
//! thousands of voters over a hundred Posts and a hundred and fifty
//! countries, with votes over a week, applications and sign-in counters.
//!
//! Each size seeds a synthetic event in bulk, including a Keycloak-shaped
//! realm (the tables and indexes the projection's query reads), then runs
//! the snapshot job's own pass over it: a first pass, an unchanged one, one
//! after 1% of the voters voted, a full pass over the accounts with nothing
//! changed, pruning, and a CSV and SQL export of every dashboard. It prints
//! what each step took and wrote (`[scale]` lines) and holds each step to a
//! budget that grows with the voters.
//!
//! `MONITORING_SCALE_VOTERS` names the sizes, comma separated; by default
//! 1000, which runs with the other database tests. Larger sizes run on
//! demand; see the monitoring scale guide in the developer documentation.

#[path = "support/schema.rs"]
mod schema;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use deadpool_postgres::{Client, Transaction};
use indexmap::IndexMap;
use sequent_core::monitoring::payload::ScopePayload;
use sequent_core::monitoring::presets;
use sequent_core::monitoring::revision::DashboardMode;
use sequent_core::monitoring::sources::DataSourceId;
use std::collections::BTreeMap;
use std::time::{Duration as Elapsed, Instant};
use tokio_postgres::IsolationLevel;
use uuid::Uuid;
use windmill::services::monitoring::config_store::{
    reset_to_preset, Author, EventRef, MonitoringConfigAudit, RecordedChange, ResetOutcome,
};
use windmill::services::monitoring::export::{
    build_file, collect_export, MonitoringExportFormat, MonitoringExportRequest,
};
use windmill::services::monitoring::projection::refresh_voter_activity;
use windmill::services::monitoring::snapshot::{
    live_snapshot, prune_snapshots, read_scope, refresh_event_snapshot, request_election_set,
    PassOptions, PassOutcome, ScopeRead,
};

const POSTS: i32 = 100;
/// Areas voting in two Posts, after one area per Post.
const SHARED_AREAS: i32 = 5;
const REGIONS: i32 = 10;
const COUNTRIES: i32 = 150;
const VOTER_GROUP: &str = "voter";
/// The schema standing in for the Keycloak database.
const KEYCLOAK: &str = "monitoring_scale_keycloak";
/// Voters whose ballots one statement inserts.
const VOTE_CHUNK: usize = 2_000;

struct NoAudit;

#[async_trait]
impl MonitoringConfigAudit for NoAudit {
    async fn prepare(&self, _: &mut Client, _: EventRef, _: &Author) -> anyhow::Result<()> {
        Ok(())
    }
    async fn record(&self, _: &Transaction<'_>, _: &RecordedChange) -> anyhow::Result<()> {
        Ok(())
    }
}

fn sizes() -> Vec<i32> {
    std::env::var("MONITORING_SCALE_VOTERS")
        .unwrap_or_else(|_| "1000".to_string())
        .split(',')
        .map(|size| size.trim().replace('_', ""))
        .filter(|size| !size.is_empty())
        .map(|size| {
            size.parse()
                .expect("MONITORING_SCALE_VOTERS: comma-separated counts")
        })
        .collect()
}

/// What a step may take for `voters`: `base` plus `per_10k` for every ten
/// thousand voters, in a release build. Set from runs on a development
/// machine with about twice the time measured; an unoptimized build (the
/// default `cargo test`) gets [`DEBUG_FACTOR`] times as long, and
/// `MONITORING_SCALE_BUDGET_FACTOR` stretches them all on a slower machine.
fn budget(voters: i32, base_ms: u64, per_10k_ms: u64) -> Elapsed {
    let factor: f64 = std::env::var("MONITORING_SCALE_BUDGET_FACTOR")
        .ok()
        .and_then(|factor| factor.parse().ok())
        .unwrap_or(1.0);
    let build = if cfg!(debug_assertions) {
        DEBUG_FACTOR
    } else {
        1.0
    };
    let ms = base_ms as f64 + per_10k_ms as f64 * f64::from(voters) / 10_000.0;
    Elapsed::from_millis((ms * factor * build) as u64)
}

/// How much slower an unoptimized build is: 4 to 14 times over these steps,
/// most where serializing and hashing every payload dominates a pass.
const DEBUG_FACTOR: f64 = 16.0;

fn check(voters: i32, step: &str, took: Elapsed, limit: Elapsed) {
    println!(
        "[scale] voters={voters} step={step} ms={} budget_ms={}",
        took.as_millis(),
        limit.as_millis()
    );
    assert!(
        took <= limit,
        "{step} at {voters} voters took {took:?}, over its budget of {limit:?}"
    );
}

async fn timed<T>(future: impl std::future::Future<Output = T>) -> (T, Elapsed) {
    let start = Instant::now();
    let value = future.await;
    (value, start.elapsed())
}

/// The realm tables, columns and indexes `fetch_voter_accounts` reads, as
/// Keycloak defines them.
async fn keycloak_schema(client: &Client) {
    client
        .batch_execute(&format!(
            "CREATE SCHEMA IF NOT EXISTS {KEYCLOAK};
             SET search_path TO {KEYCLOAK};
             CREATE TABLE IF NOT EXISTS realm (
                 id varchar(36) PRIMARY KEY, name varchar(255) UNIQUE);
             CREATE TABLE IF NOT EXISTS user_entity (
                 id varchar(36) PRIMARY KEY, realm_id varchar(255) NOT NULL,
                 username varchar(255), enabled boolean NOT NULL DEFAULT true,
                 UNIQUE (realm_id, username));
             CREATE TABLE IF NOT EXISTS user_attribute (
                 name varchar(255) NOT NULL, value varchar(255),
                 user_id varchar(36) NOT NULL, id varchar(36) PRIMARY KEY);
             CREATE INDEX IF NOT EXISTS idx_user_attribute ON user_attribute (user_id);
             CREATE INDEX IF NOT EXISTS idx_user_attribute_name ON user_attribute (name, value);
             CREATE TABLE IF NOT EXISTS credential (
                 id varchar(36) PRIMARY KEY, user_id varchar(36), type varchar(255),
                 created_date bigint);
             CREATE INDEX IF NOT EXISTS idx_user_credential ON credential (user_id);
             CREATE TABLE IF NOT EXISTS keycloak_group (
                 id varchar(36) PRIMARY KEY, name varchar(255), realm_id varchar(36));
             CREATE TABLE IF NOT EXISTS user_group_membership (
                 group_id varchar(36) NOT NULL, user_id varchar(36) NOT NULL,
                 PRIMARY KEY (group_id, user_id));
             CREATE INDEX IF NOT EXISTS idx_user_group_mapping ON user_group_membership (user_id);"
        ))
        .await
        .unwrap();
}

/// A synthetic event of `voters` voters, seeded in bulk. Every choice is a
/// hash of the voter's number, so a size seeds the same event every time.
async fn seed(client: &mut Client, keycloak: &Client, voters: i32) -> EventRef {
    let event = EventRef {
        tenant_id: Uuid::new_v4(),
        election_event_id: Uuid::new_v4(),
    };
    let (tenant, id) = (event.tenant_id, event.election_event_id);
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&id, &tenant],
    )
    .await
    .unwrap();
    // Posts: a region each, most open, some closed or not started.
    tx.execute(
        "INSERT INTO sequent_backend.election
             (id, tenant_id, election_event_id, presentation, annotations, status,
              num_allowed_revotes)
         SELECT md5($2::uuid::text || '/post/' || p)::uuid, $1, $2,
                jsonb_build_object('i18n', jsonb_build_object('en',
                    jsonb_build_object('name', format('Post %s', lpad(p::text, 3, '0'))))),
                jsonb_build_object('miru:geographical-region', format('Region %s', p % $3::int)),
                jsonb_build_object('voting_status',
                    CASE WHEN p % 10 = 0 THEN 'CLOSED' WHEN p % 17 = 0 THEN 'NOT_STARTED'
                         ELSE 'OPEN' END),
                0
         FROM generate_series(0, $4::int - 1) AS p",
        &[&tenant, &id, &REGIONS, &POSTS],
    )
    .await
    .unwrap();
    // One area per Post, then a few voting in two Posts; a contest per
    // area and Post.
    tx.execute(
        "WITH areas AS (
             SELECT a, md5($2::uuid::text || '/area/' || a)::uuid AS area,
                    CASE WHEN a < $3::int THEN ARRAY[a] ELSE ARRAY[a - $3::int, a - $3::int + 1] END AS posts
             FROM generate_series(0, $3::int + $4::int - 1) AS a
         ),
         inserted AS (
             INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             SELECT area, $1, $2, format('Area %s', a) FROM areas
         ),
         pairs AS (SELECT a, area, unnest(posts) AS p FROM areas),
         contests AS (
             INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
             SELECT md5($2::uuid::text || '/contest/' || a || '/' || p)::uuid, $1, $2,
                    md5($2::uuid::text || '/post/' || p)::uuid
             FROM pairs
         )
         INSERT INTO sequent_backend.area_contest
             (id, tenant_id, election_event_id, area_id, contest_id)
         SELECT md5($2::uuid::text || '/area_contest/' || a || '/' || p)::uuid, $1, $2, area,
                md5($2::uuid::text || '/contest/' || a || '/' || p)::uuid
         FROM pairs",
        &[&tenant, &id, &POSTS, &SHARED_AREAS],
    )
    .await
    .unwrap();
    // A third of the voters applied: most approved, some disapproved for
    // one of six reasons, the rest pending; a few applied twice.
    tx.execute(
        "WITH voters AS (
             SELECT i, md5($2::uuid::text || '/voter/' || i)::uuid::text AS voter,
                    md5($2::uuid::text || '/area/' || ((hashint4(i) & 2147483647) % ($3::int + $5::int)))::uuid AS area,
                    (hashint4(i * 11 + 3) & 2147483647) % 100 AS roll,
                    (hashint4(i * 11 + 4) & 2147483647) % (10 * 24 * 3600) AS second
             FROM generate_series(0, $4::int - 1) AS i
         ),
         applied AS (
             SELECT voter, area, roll, second, 0 AS attempt FROM voters WHERE roll < 33
             UNION ALL
             SELECT voter, area, roll, second + 3600, 1 FROM voters WHERE roll < 3
         )
         INSERT INTO sequent_backend.applications
             (tenant_id, election_event_id, area_id, applicant_id, status, verification_type,
              applicant_data, annotations, created_at, updated_at)
         SELECT $1, $2, area, voter,
                CASE WHEN roll < 20 THEN 'ACCEPTED' WHEN roll < 28 THEN 'REJECTED'
                     ELSE 'PENDING' END,
                'MANUAL', '{}',
                CASE WHEN roll >= 20 AND roll < 28 THEN jsonb_build_object('rejection_reason',
                    (ARRAY['Blurred ID', 'Expired passport', 'Name mismatch',
                           'Underage', 'Duplicate record', 'Not a resident'])[1 + roll % 6])
                     ELSE '{}'::jsonb END,
                now() - interval '10 days' + make_interval(secs => second),
                now() - interval '10 days' + make_interval(secs => second + attempt)
         FROM applied",
        &[&tenant, &id, &POSTS, &voters, &SHARED_AREAS],
    )
    .await
    .unwrap();
    // Sign-ins every 15 minutes over the week, in as many areas as a
    // thousand voters fill, plus failures and unknown usernames.
    tx.execute(
        "WITH buckets AS (
             SELECT date_bin('15 minutes', now() - interval '7 days', '2000-01-01')
                    + b * interval '15 minutes' AS bucket_start, b
             FROM generate_series(0, 7 * 96 - 1) AS b
         ),
         areas AS (
             SELECT a, md5($2::uuid::text || '/area/' || a)::uuid AS area
             FROM generate_series(0, least($3::int + $5::int, greatest(1, $4::int / 1000)) - 1) AS a
         ),
         rows AS (
             SELECT bucket_start, 'LOGIN' AS event_type, 'REGISTERED' AS registration,
                    area, 1 + (hashint4(b * 131 + a) & 2147483647) % 20 AS attempts
             FROM buckets, areas
             UNION ALL
             SELECT bucket_start, 'LOGIN_ERROR', 'REGISTERED', area,
                    1 + (hashint4(b * 137 + a) & 2147483647) % 3
             FROM buckets, areas WHERE (hashint4(b * 139 + a) & 2147483647) % 4 = 0
             UNION ALL
             SELECT bucket_start, 'LOGIN_ERROR', 'UNREGISTERED', NULL,
                    1 + (hashint4(b) & 2147483647) % 5
             FROM buckets
             UNION ALL
             SELECT bucket_start, 'UPDATE_PASSWORD', 'REGISTERED', area, 1
             FROM buckets, areas WHERE (hashint4(b * 149 + a) & 2147483647) % 20 = 0
         )
         INSERT INTO sequent_backend.monitoring_login_counter
             (tenant_id, election_event_id, bucket_start, event_type, registration, area_id,
              attempts)
         SELECT $1, $2, bucket_start, event_type, registration, area, attempts FROM rows",
        &[&tenant, &id, &POSTS, &voters, &SHARED_AREAS],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    // 60% of voters vote, over seven days ending an hour ago; one in six of
    // them votes again later; a few leave a discarded or unfinished ballot.
    // In chunks: the revote limit's trigger takes a lock per voter, which
    // one transaction cannot hold for tens of thousands.
    for from in (0..voters).step_by(VOTE_CHUNK) {
        client
            .execute(
                "WITH voters AS (
             SELECT i, md5($2::uuid::text || '/voter/' || i)::uuid::text AS voter,
                    ((hashint4(i) & 2147483647) % ($3::int + $5::int)) AS a,
                    (hashint4(i * 7 + 1) & 2147483647) % 100 AS roll,
                    (hashint4(i * 7 + 2) & 2147483647) % (7 * 24 * 3600) AS second
             FROM generate_series($6::int, least($4::int, $6::int + $7::int) - 1) AS i
         ),
         placed AS (
             SELECT *, md5($2::uuid::text || '/area/' || a)::uuid AS area,
                    CASE WHEN a < $3::int THEN a ELSE a - $3::int END AS p
             FROM voters
         ),
         ballots AS (
             SELECT area, p, voter, 'valid' AS status, second FROM placed WHERE roll < 60
             UNION ALL
             SELECT area, p, voter, 'valid', second + (7 * 24 * 3600 - second) / 2
             FROM placed WHERE roll < 10
             UNION ALL
             SELECT area, p, voter, CASE WHEN roll < 63 THEN 'discarded' ELSE 'in-progress' END,
                    second
             FROM placed WHERE roll >= 60 AND roll < 65
         )
         INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string, status,
              created_at)
         SELECT $1, $2, md5($2::uuid::text || '/post/' || p)::uuid, area, voter, status,
                now() - interval '7 days 1 hour' + make_interval(secs => second)
         FROM ballots",
                &[
                    &tenant,
                    &id,
                    &POSTS,
                    &voters,
                    &SHARED_AREAS,
                    &from,
                    &(VOTE_CHUNK as i32),
                ],
            )
            .await
            .unwrap();
    }

    let ResetOutcome::Reset { .. } = reset_to_preset(
        client,
        &NoAudit,
        event,
        &Author {
            id: "admin-1".to_string(),
            name: None,
        },
        "comelec",
        DashboardMode::Configured,
    )
    .await
    .unwrap() else {
        panic!("the reset writes the preset");
    };

    // The realm: every voter in the voter group with the attributes the
    // COMELEC settings read, most with a credential.
    let realm = format!("tenant-{tenant}-event-{id}");
    keycloak
        .batch_execute(&format!(
            "SET search_path TO {KEYCLOAK};
             INSERT INTO realm (id, name) VALUES ('{id}', '{realm}');
             INSERT INTO keycloak_group (id, name, realm_id)
             VALUES (md5('{id}/group')::uuid::text, '{VOTER_GROUP}', '{id}');
             INSERT INTO user_entity (id, realm_id, username)
             SELECT md5('{id}/voter/' || i)::uuid::text, '{id}', 'voter-' || i
             FROM generate_series(0, {voters} - 1) AS i;
             INSERT INTO user_group_membership (group_id, user_id)
             SELECT md5('{id}/group')::uuid::text, md5('{id}/voter/' || i)::uuid::text
             FROM generate_series(0, {voters} - 1) AS i;
             INSERT INTO credential (id, user_id, type, created_date)
             SELECT md5('{id}/credential/' || i)::uuid::text, md5('{id}/voter/' || i)::uuid::text,
                    'password',
                    (extract(epoch FROM now() - interval '20 days') * 1000)::bigint
                        + (hashint4(i * 13 + 5) & 2147483647) % (14 * 86400000)
             FROM generate_series(0, {voters} - 1) AS i
             WHERE (hashint4(i * 13 + 6) & 2147483647) % 100 < 70;
             WITH voters AS (
                 SELECT i, md5('{id}/voter/' || i)::uuid::text AS voter,
                        (hashint4(i) & 2147483647) % ({POSTS} + {SHARED_AREAS}) AS a,
                        (hashint4(i * 17 + 7) & 2147483647) % 100 AS roll,
                        (hashint4(i * 17 + 8) & 2147483647) % 100 AS abroad
                 FROM generate_series(0, {voters} - 1) AS i
             ),
             attributes AS (
                 SELECT voter, i, 'area-id' AS name, md5('{id}/area/' || a)::uuid::text AS value
                 FROM voters
                 UNION ALL
                 -- Most live in their Post's country; the rest anywhere.
                 SELECT voter, i, 'country',
                        format('Country %s/Embassy %s',
                               lpad((CASE WHEN abroad < 80 THEN a % {COUNTRIES}
                                          ELSE (hashint4(i * 17 + 9) & 2147483647) % {COUNTRIES}
                                     END)::text, 3, '0'),
                               a)
                 FROM voters
                 UNION ALL
                 SELECT voter, i, 'sex', CASE WHEN roll < 48 THEN 'M' WHEN roll < 97 THEN 'F' END
                 FROM voters WHERE roll < 97
                 UNION ALL
                 SELECT voter, i, 'dateOfBirth',
                        to_char(date '1940-01-01' + (hashint4(i * 17 + 10) & 2147483647) % 24000,
                                'YYYY-MM-DD')
                 FROM voters
                 UNION ALL
                 SELECT voter, i, 'landBasedOrSeafarer',
                        CASE WHEN roll % 5 = 0 THEN 'Seafarer' ELSE 'Land-based' END
                 FROM voters
                 UNION ALL
                 SELECT voter, i, 'sequent.read-only.id-card-number-validated', 'VERIFIED'
                 FROM voters WHERE roll % 5 < 2
             )
             INSERT INTO user_attribute (id, user_id, name, value)
             SELECT md5('{id}/attribute/' || i || '/' || name)::uuid::text, voter, name, value
             FROM attributes;
             ANALYZE user_entity; ANALYZE user_attribute; ANALYZE credential;
             ANALYZE user_group_membership;"
        ))
        .await
        .unwrap();
    event
}

async fn pass(
    hasura: &mut Client,
    keycloak: &mut Client,
    event: EventRef,
    now: chrono::DateTime<Utc>,
) -> PassOutcome {
    refresh_event_snapshot(
        hasura,
        keycloak,
        event,
        PassOptions {
            voter_group: VOTER_GROUP,
            full_pass_every: Duration::minutes(5),
            now,
        },
    )
    .await
    .unwrap()
}

/// How many payloads the event keeps, their bytes and the largest.
async fn payload_sizes(client: &Client, event: EventRef) -> (i64, i64, i64) {
    let row = client
        .query_one(
            "SELECT count(*), COALESCE(sum(octet_length(payload)), 0)::bigint,
                    COALESCE(max(octet_length(payload)), 0)::bigint
             FROM sequent_backend.monitoring_snapshot_payload
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}

/// The figures showing, by source and kind of scope.
async fn figures_by_kind(client: &Client, event: EventRef) -> BTreeMap<(String, String), i64> {
    client
        .query(
            "SELECT source, regexp_replace(scope_key, '=[^&]*', '', 'g') AS kind, count(*)
             FROM sequent_backend.monitoring_snapshot_figure
             WHERE tenant_id = $1 AND election_event_id = $2 AND to_revision IS NULL
             GROUP BY 1, 2 ORDER BY 1, 2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| ((row.get(0), row.get(1)), row.get(2)))
        .collect()
}

async fn event_payload(
    client: &mut Client,
    event: EventRef,
    revision: i64,
    set: &str,
    source: DataSourceId,
) -> ScopePayload {
    let tx = client.transaction().await.unwrap();
    let read = read_scope(&tx, event, revision, source, set, "event")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    match read {
        ScopeRead::Payload { text, .. } => serde_json::from_str(&text).unwrap(),
        other => panic!("no {source} payload for the event: {other:?}"),
    }
}

fn peak_rows(voters: i32, source: DataSourceId, payload: &ScopePayload) {
    let groups: Vec<String> = payload
        .groups
        .iter()
        .map(|(dimension, rows)| format!("{dimension}={}", rows.len()))
        .collect();
    println!(
        "[scale] voters={voters} source={source} scope=event groups=[{}] cube_cells={} \
         posts={} series_buckets={}",
        groups.join(" "),
        payload.cube.as_ref().map_or(0, |cube| cube.cells.len()),
        payload.posts.len(),
        payload.series.len(),
    );
}

async fn export(
    client: &mut Client,
    event: EventRef,
    posts: &[Uuid],
    revision: i64,
    dashboard: &str,
    format: MonitoringExportFormat,
) -> (usize, usize) {
    let request = MonitoringExportRequest {
        tenant_id: event.tenant_id.to_string(),
        election_event_id: event.election_event_id.to_string(),
        dashboard_id: dashboard.to_string(),
        widget_id: None,
        election_ids: posts.iter().map(Uuid::to_string).collect(),
        pinned_post: None,
        scope: Default::default(),
        selector_values: IndexMap::new(),
        widget_selector_values: IndexMap::new(),
        snapshot_revision: revision,
        config_generation: None,
        format,
        from: None,
        to: None,
        document_id: Uuid::new_v4().to_string(),
    };
    let tx = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .unwrap();
    let data = collect_export(&tx, &request)
        .await
        .unwrap_or_else(|error| panic!("exporting {dashboard}: {error}"));
    tx.commit().await.unwrap();
    let rows = data.widgets.len();
    let bytes = build_file(&data, format).unwrap();
    (rows, bytes.len())
}

/// The process's peak resident memory, in MiB.
fn peak_rss_mib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("VmHWM:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|kib| kib.parse::<u64>().ok())
        })
        .map_or(0, |kib| kib / 1024)
}

async fn at_size(voters: i32) {
    let pool = schema::pool().await;
    let mut hasura = pool.get().await.unwrap();
    let mut keycloak = pool.get().await.unwrap();
    keycloak_schema(&keycloak).await;

    let (event, took) = timed(seed(&mut hasura, &keycloak, voters)).await;
    println!("[scale] voters={voters} step=seed ms={}", took.as_millis());
    let posts: Vec<Uuid> = hasura
        .query(
            "SELECT id FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    let now = Utc::now();

    // The first pass: every account read, every scope written.
    let (first, took) = timed(pass(&mut hasura, &mut keycloak, event, now)).await;
    let PassOutcome::Completed {
        revision,
        figures_written,
    } = first
    else {
        panic!("the first pass completes: {first:?}");
    };
    check(voters, "first_pass", took, budget(voters, 3_000, 5_000));
    let projected: i64 = hasura
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_voter
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .unwrap()
        .get(0);
    assert!(
        projected >= i64::from(voters),
        "every voter is projected into their Posts"
    );
    let (payloads, bytes, largest) = payload_sizes(&hasura, event).await;
    println!(
        "[scale] voters={voters} projected_rows={projected} figures_written={figures_written} \
         payloads={payloads} payload_bytes={bytes} largest_payload_bytes={largest}"
    );
    for ((source, kind), count) in figures_by_kind(&hasura, event).await {
        println!("[scale] voters={voters} figures source={source} scope={kind} count={count}");
    }

    // Bringing enrollment and first votes up to date, on its own, before
    // the table's statistics catch up with the rows just written.
    {
        let tx = hasura.transaction().await.unwrap();
        let (changed, took) = timed(refresh_voter_activity(&tx, event)).await;
        assert_eq!(
            changed.unwrap(),
            0,
            "the pass left nothing to bring up to date"
        );
        tx.rollback().await.unwrap();
        check(voters, "activity_refresh", took, budget(voters, 500, 500));
    }

    let set = {
        let tx = hasura.transaction().await.unwrap();
        let set = request_election_set(&tx, event, &posts).await.unwrap();
        tx.commit().await.unwrap();
        set
    };
    let turnout = event_payload(
        &mut hasura,
        event,
        revision,
        &set,
        DataSourceId::VoterTurnout,
    )
    .await;
    peak_rows(voters, DataSourceId::VoterTurnout, &turnout);
    assert_eq!(turnout.groups["post"].len(), POSTS as usize, "every Post");
    assert!(
        turnout.groups["country"].len() >= COUNTRIES.min(voters / 10) as usize,
        "the countries voters live in"
    );
    assert_eq!(
        turnout.totals[&sequent_core::monitoring::sources::Measure::Registered],
        voters as u64
    );
    for source in [
        DataSourceId::EnrollmentDecisions,
        DataSourceId::PollStatus,
        DataSourceId::AccessSecurity,
    ] {
        let payload = event_payload(&mut hasura, event, revision, &set, source).await;
        peak_rows(voters, source, &payload);
    }

    // Nothing changed: no run is written.
    let (second, took) = timed(pass(&mut hasura, &mut keycloak, event, now)).await;
    assert_eq!(
        second,
        PassOutcome::Unchanged {
            shown: Some(revision)
        }
    );
    check(voters, "unchanged_pass", took, budget(voters, 1_000, 1_000));

    // 1% of the voters vote now.
    hasura
        .execute(
            "INSERT INTO sequent_backend.cast_vote
                 (tenant_id, election_event_id, election_id, area_id, voter_id_string, status,
                  created_at)
             SELECT v.tenant_id, v.election_event_id, v.election_id, v.area_id, v.voter_id,
                    'valid', now()
             FROM (SELECT DISTINCT ON (voter_id) * FROM sequent_backend.monitoring_voter
                   WHERE tenant_id = $1 AND election_event_id = $2 AND first_voted_at IS NULL
                   ORDER BY voter_id) v
             LIMIT $3",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &i64::from((voters / 100).max(1)),
            ],
        )
        .await
        .unwrap();
    let (changed, took) = timed(pass(&mut hasura, &mut keycloak, event, now)).await;
    let PassOutcome::Completed {
        revision: after_change,
        figures_written: changed_figures,
    } = changed
    else {
        panic!("a change completes a run: {changed:?}");
    };
    check(
        voters,
        "pass_after_1pct_votes",
        took,
        budget(voters, 1_000, 1_500),
    );
    let (_, bytes_after, _) = payload_sizes(&hasura, event).await;
    println!(
        "[scale] voters={voters} after_1pct figures_written={changed_figures} \
         new_payload_bytes={}",
        bytes_after - bytes
    );
    assert!(
        (changed_figures as i64) < i64::from(voters).max(figures_written as i64),
        "a small change writes what it changed"
    );

    // A full pass over the accounts, which changed in nothing.
    let later = now + Duration::minutes(10);
    let (full, took) = timed(pass(&mut hasura, &mut keycloak, event, later)).await;
    assert_eq!(
        full,
        PassOutcome::Unchanged {
            shown: Some(after_change)
        }
    );
    check(
        voters,
        "unchanged_full_pass",
        took,
        budget(voters, 1_000, 2_000),
    );

    // Pruning drops the replaced run and what only it held. Right after
    // the first passes the tables' statistics lag behind their rows, and
    // the per-payload checks of the delete (the foreign key and the
    // payload's trigger) scan the event's figures: 12 s at 50,000 voters
    // until autovacuum analyzes the table, tens of milliseconds after.
    let (pruned, took) = timed(prune_snapshots(&mut hasura, event, Duration::zero())).await;
    let pruned = pruned.unwrap();
    println!(
        "[scale] voters={voters} pruned runs={} figures={} payloads={}",
        pruned.runs, pruned.figures, pruned.payloads
    );
    assert_eq!(pruned.runs, 1);
    check(voters, "prune", took, budget(voters, 1_000, 4_000));
    let shown = {
        let tx = hasura.transaction().await.unwrap();
        let shown = live_snapshot(&tx, event).await.unwrap().unwrap();
        tx.commit().await.unwrap();
        shown.revision
    };
    assert_eq!(shown, after_change);

    // Every dashboard, as CSV and SQL, for every election.
    let dashboards: Vec<String> = presets::load("comelec")
        .unwrap()
        .unwrap()
        .set
        .dashboards
        .keys()
        .cloned()
        .collect();
    for format in [MonitoringExportFormat::Csv, MonitoringExportFormat::Sql] {
        let mut total = Elapsed::ZERO;
        let mut largest = (String::new(), 0);
        let mut all_bytes = 0;
        for dashboard in &dashboards {
            let ((_, size), took) =
                timed(export(&mut hasura, event, &posts, shown, dashboard, format)).await;
            total += took;
            all_bytes += size;
            if size > largest.1 {
                largest = (dashboard.clone(), size);
            }
            assert!(
                took <= budget(voters, 500, 200),
                "exporting {dashboard} as {format} took {took:?}"
            );
        }
        println!(
            "[scale] voters={voters} export format={format} dashboards={} total_ms={} \
             total_bytes={all_bytes} largest={}:{}",
            dashboards.len(),
            total.as_millis(),
            largest.0,
            largest.1
        );
    }
    println!("[scale] voters={voters} peak_rss_mib={}", peak_rss_mib());
}

#[tokio::test]
async fn the_snapshot_pipeline_holds_up_at_election_sizes() {
    for voters in sizes() {
        at_size(voters).await;
    }
}
