// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Initialization at the event's scope (VOTE-LIFECYCLE §9) against the
//! migrated schema: what a completed initialization report records per Post
//! and per country, and the opening gate that reads it. Each test writes its
//! own rows in a transaction and rolls back.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::json;
use uuid::Uuid;
use windmill::postgres::election::get_elections;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::postgres::election_initialization::list_election_initializations;
use windmill::services::election_event_status::TransitionRefusal;
use windmill::services::initialization_record::{
    stage_initialization_log, store_initialization, INITIALIZATION_AREA_IDS_ANNOTATION,
};
use windmill::services::initialization_scope::initialization_refusals;

/// One event with Post `p1` (countries `a` and `b`) and Post `p2` (country
/// `c`), both requiring their initialization report.
struct World {
    tenant: Uuid,
    event: Uuid,
    p1: Uuid,
    p2: Uuid,
    a: Uuid,
    b: Uuid,
    c: Uuid,
}

async fn execute(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) {
    tx.execute(sql, params).await.unwrap();
}

impl World {
    async fn new(tx: &Transaction<'_>, scope: &str) -> World {
        windmill::postgres::trusted_write(tx).await.unwrap();
        let world = World {
            tenant: Uuid::new_v4(),
            event: Uuid::new_v4(),
            p1: Uuid::new_v4(),
            p2: Uuid::new_v4(),
            a: Uuid::new_v4(),
            b: Uuid::new_v4(),
            c: Uuid::new_v4(),
        };
        execute(
            tx,
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&world.tenant, &world.tenant.to_string()],
        )
        .await;
        let presentation = json!({
            "lifecycle_policies": { "initialization_scope": scope },
        });
        let board = json!({ "id": 1, "database_name": "board", "is_archived": false });
        execute(
            tx,
            "INSERT INTO sequent_backend.election_event
                 (id, tenant_id, encryption_protocol, presentation, bulletin_board_reference)
             VALUES ($1, $2, 'RSA256', $3, $4)",
            &[&world.event, &world.tenant, &presentation, &board],
        )
        .await;
        execute(
            tx,
            "INSERT INTO sequent_backend.ballot_publication (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&world.event, &world.tenant, &world.event],
        )
        .await;
        for (post, name, areas) in [
            (
                world.p1,
                "Dubai PCG",
                vec![(world.a, "United Arab Emirates"), (world.b, "Oman")],
            ),
            (world.p2, "Madrid", vec![(world.c, "Spain")]),
        ] {
            let presentation = json!({
                "initialization_report_policy": "required",
                "i18n": { "en": { "name": name } },
            });
            execute(
                tx,
                "INSERT INTO sequent_backend.election
                     (id, tenant_id, election_event_id, presentation)
                 VALUES ($1, $2, $3, $4)",
                &[&post, &world.tenant, &world.event, &presentation],
            )
            .await;
            let contest = Uuid::new_v4();
            execute(
                tx,
                "INSERT INTO sequent_backend.contest
                     (id, tenant_id, election_event_id, election_id)
                 VALUES ($1, $2, $3, $4)",
                &[&contest, &world.tenant, &world.event, &post],
            )
            .await;
            for (area, area_name) in areas {
                execute(
                    tx,
                    "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
                     VALUES ($1, $2, $3, $4)",
                    &[&area, &world.tenant, &world.event, &area_name],
                )
                .await;
                execute(
                    tx,
                    "INSERT INTO sequent_backend.area_contest
                         (id, tenant_id, election_event_id, area_id, contest_id)
                     VALUES ($1, $2, $3, $4, $5)",
                    &[
                        &Uuid::new_v4(),
                        &world.tenant,
                        &world.event,
                        &area,
                        &contest,
                    ],
                )
                .await;
                world.ballot_style(tx, post, area).await;
            }
        }
        world
    }

    /// Voters of `post` vote in `area`.
    async fn ballot_style(&self, tx: &Transaction<'_>, post: Uuid, area: Uuid) {
        execute(
            tx,
            "INSERT INTO sequent_backend.ballot_style
                 (tenant_id, election_event_id, election_id, area_id, ballot_publication_id)
             VALUES ($1, $2, $3, $4, $5)",
            &[&self.tenant, &self.event, &post, &area, &self.event],
        )
        .await;
    }

    /// A completed initialization report tally of `post` covering `areas`,
    /// area-filtered when `filtered`, whose results carry `hash`.
    async fn report(
        &self,
        tx: &Transaction<'_>,
        post: Uuid,
        areas: &[Uuid],
        filtered: bool,
        hash: &str,
    ) -> (Uuid, Uuid) {
        let session = Uuid::new_v4();
        let keys = Uuid::new_v4();
        execute(
            tx,
            "INSERT INTO sequent_backend.keys_ceremony
                 (id, tenant_id, election_event_id, trustee_ids, threshold)
             VALUES ($1, $2, $3, '{}', 2)",
            &[&keys, &self.tenant, &self.event],
        )
        .await;
        let mut annotations = json!({
            "executer_username": "sbei.dubai",
            "executer_user_id": "user-1",
        });
        if filtered {
            annotations[INITIALIZATION_AREA_IDS_ANNOTATION] = json!(areas);
        }
        execute(
            tx,
            "INSERT INTO sequent_backend.tally_session
                 (id, tenant_id, election_event_id, keys_ceremony_id, threshold, election_ids,
                  area_ids, annotations, tally_type)
             VALUES ($1, $2, $3, $4, 2, $5, $6, $7, 'INITIALIZATION_REPORT')",
            &[
                &session,
                &self.tenant,
                &self.event,
                &keys,
                &vec![post],
                &areas.to_vec(),
                &annotations,
            ],
        )
        .await;
        let results = Uuid::new_v4();
        execute(
            tx,
            "INSERT INTO sequent_backend.results_event (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&results, &self.tenant, &self.event],
        )
        .await;
        let document = Uuid::new_v4();
        execute(
            tx,
            "INSERT INTO sequent_backend.results_election
                 (id, tenant_id, election_event_id, results_event_id, election_id,
                  total_voters_percent, annotations, documents)
             VALUES ($1, $2, $3, $4, $5, 0, $6, $7)",
            &[
                &Uuid::new_v4(),
                &self.tenant,
                &self.event,
                &results,
                &post,
                &json!({ "results_hash": hash }),
                &json!({ "pdf": document.to_string() }),
            ],
        )
        .await;
        execute(
            tx,
            "INSERT INTO sequent_backend.tally_session_execution
                 (id, tenant_id, election_event_id, tally_session_id, current_message_id,
                  results_event_id)
             VALUES ($1, $2, $3, $4, 1, $5)",
            &[
                &Uuid::new_v4(),
                &self.tenant,
                &self.event,
                &session,
                &results,
            ],
        )
        .await;
        (session, document)
    }

    async fn store(&self, tx: &Transaction<'_>, session: Uuid, post: Uuid) {
        store_initialization(
            tx,
            &self.tenant.to_string(),
            &self.event.to_string(),
            &session.to_string(),
            &post.to_string(),
        )
        .await
        .unwrap();
    }

    async fn initialized(&self, tx: &Transaction<'_>, post: Uuid) -> bool {
        tx.query_one(
            "SELECT COALESCE(initialization_report_generated, false)
             FROM sequent_backend.election WHERE id = $1",
            &[&post],
        )
        .await
        .unwrap()
        .get(0)
    }

    async fn refusals(&self, tx: &Transaction<'_>) -> Vec<(String, &'static str)> {
        let event = get_election_event_by_id(tx, &self.tenant.to_string(), &self.event.to_string())
            .await
            .unwrap();
        let elections = get_elections(tx, &self.tenant.to_string(), &self.event.to_string())
            .await
            .unwrap();
        initialization_refusals(tx, &event, &elections)
            .await
            .unwrap()
            .into_iter()
            .map(|(id, refusal)| (id, refusal.code()))
            .collect()
    }

    fn name(&self, post: Uuid) -> String {
        post.to_string()
    }
}

#[tokio::test]
async fn a_post_report_records_each_country_with_its_hash_and_document() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "post").await;
    let (session, document) = world
        .report(&tx, world.p1, &[world.a, world.b], false, "hash-1")
        .await;

    world.store(&tx, session, world.p1).await;

    let rows = list_election_initializations(&tx, world.tenant, world.event)
        .await
        .unwrap();
    let mut areas: Vec<Option<Uuid>> = rows.iter().map(|row| row.area_id).collect();
    areas.sort();
    let mut expected = vec![Some(world.a), Some(world.b)];
    expected.sort();
    assert_eq!(areas, expected);
    for row in &rows {
        assert_eq!(row.election_id, world.p1);
        assert_eq!(row.tally_session_id, session);
        assert_eq!(row.report_hash.as_deref(), Some("hash-1"));
        assert_eq!(row.document_id, Some(document));
        assert_eq!(row.created_by_username.as_deref(), Some("sbei.dubai"));
    }
    assert!(world.initialized(&tx, world.p1).await);
    assert!(!world.initialized(&tx, world.p2).await);
}

#[tokio::test]
async fn country_reports_initialize_the_post_with_its_last_country() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "post-and-country").await;

    let (first, first_document) = world
        .report(&tx, world.p1, &[world.a], true, "hash-a")
        .await;
    world.store(&tx, first, world.p1).await;
    assert!(!world.initialized(&tx, world.p1).await);

    let (second, second_document) = world
        .report(&tx, world.p1, &[world.b], true, "hash-b")
        .await;
    world.store(&tx, second, world.p1).await;
    assert!(world.initialized(&tx, world.p1).await);

    // One dated row and report per country.
    let rows = list_election_initializations(&tx, world.tenant, world.event)
        .await
        .unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (row.area_id, row.report_hash.clone(), row.document_id))
            .collect::<Vec<_>>(),
        vec![
            (
                Some(world.a),
                Some("hash-a".to_string()),
                Some(first_document)
            ),
            (
                Some(world.b),
                Some("hash-b".to_string()),
                Some(second_document)
            ),
        ]
    );
    assert!(rows[0].created_at <= rows[1].created_at);
}

#[tokio::test]
async fn the_gate_reads_the_recorded_countries_and_posts() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    // Post and country: p1 waits for Oman after the UAE report.
    let world = World::new(&tx, "post-and-country").await;
    assert_eq!(
        world.refusals(&tx).await,
        sorted(vec![
            (world.name(world.p1), "initialization-report-required"),
            (world.name(world.p2), "initialization-report-required"),
        ])
    );
    let (session, _) = world
        .report(&tx, world.p1, &[world.a], true, "hash-a")
        .await;
    world.store(&tx, session, world.p1).await;
    assert_eq!(
        world.refusals(&tx).await,
        sorted(vec![
            (world.name(world.p1), "initialization-report-required"),
            (world.name(world.p2), "initialization-report-required"),
        ])
    );
    let (session, _) = world
        .report(&tx, world.p1, &[world.b], true, "hash-b")
        .await;
    world.store(&tx, session, world.p1).await;
    assert_eq!(
        world.refusals(&tx).await,
        vec![(world.name(world.p2), "initialization-report-required")]
    );

    // Event: p1 waits for p2.
    let world = World::new(&tx, "event").await;
    let (session, _) = world
        .report(&tx, world.p1, &[world.a, world.b], false, "hash-1")
        .await;
    world.store(&tx, session, world.p1).await;
    assert_eq!(
        world.refusals(&tx).await,
        sorted(vec![
            (world.name(world.p1), "event-not-initialized"),
            (world.name(world.p2), "initialization-report-required"),
        ])
    );
    let (session, _) = world
        .report(&tx, world.p2, &[world.c], false, "hash-2")
        .await;
    world.store(&tx, session, world.p2).await;
    assert_eq!(world.refusals(&tx).await, vec![]);
}

#[tokio::test]
async fn a_country_added_after_a_post_report_waits_under_post_and_country() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "post-and-country").await;
    let (session, _) = world
        .report(&tx, world.p1, &[world.a, world.b], false, "hash-1")
        .await;
    world.store(&tx, session, world.p1).await;
    let (session, _) = world
        .report(&tx, world.p2, &[world.c], false, "hash-2")
        .await;
    world.store(&tx, session, world.p2).await;
    assert_eq!(world.refusals(&tx).await, vec![]);

    // A new country of p1, after its report.
    let late = Uuid::new_v4();
    execute(
        &tx,
        "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, 'Qatar')",
        &[&late, &world.tenant, &world.event],
    )
    .await;
    execute(
        &tx,
        "INSERT INTO sequent_backend.area_contest
             (id, tenant_id, election_event_id, area_id, contest_id)
         SELECT $1, $2, $3, $4, id FROM sequent_backend.contest WHERE election_id = $5",
        &[
            &Uuid::new_v4(),
            &world.tenant,
            &world.event,
            &late,
            &world.p1,
        ],
    )
    .await;
    world.ballot_style(&tx, world.p1, late).await;
    let event = get_election_event_by_id(&tx, &world.tenant.to_string(), &world.event.to_string())
        .await
        .unwrap();
    let elections = get_elections(&tx, &world.tenant.to_string(), &world.event.to_string())
        .await
        .unwrap();
    let refusals = initialization_refusals(&tx, &event, &elections)
        .await
        .unwrap();
    assert_eq!(
        refusals.get(&world.p1.to_string()),
        Some(&TransitionRefusal::CountriesNotInitialized {
            post: "Dubai PCG".to_string(),
            area_ids: vec![late.to_string()],
            names: vec!["Qatar".to_string()],
        })
    );
}

fn sorted(mut items: Vec<(String, &'static str)>) -> Vec<(String, &'static str)> {
    items.sort();
    items
}

#[tokio::test]
async fn a_rerun_of_a_report_records_nothing_twice() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "post").await;
    let (session, _) = world
        .report(&tx, world.p1, &[world.a, world.b], false, "hash-1")
        .await;
    world.store(&tx, session, world.p1).await;
    world.store(&tx, session, world.p1).await;
    assert_eq!(
        list_election_initializations(&tx, world.tenant, world.event)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn a_report_without_results_never_marks_initialization_complete() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "post").await;
    let (session, _) = world
        .report(&tx, world.p1, &[world.a, world.b], false, "hash-1")
        .await;
    tx.execute(
        "DELETE FROM sequent_backend.results_election WHERE election_id = $1",
        &[&world.p1],
    )
    .await
    .unwrap();
    let error = store_initialization(
        &tx,
        &world.tenant.to_string(),
        &world.event.to_string(),
        &session.to_string(),
        &world.p1.to_string(),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("missing its hash or document"));
    let rows = list_election_initializations(&tx, world.tenant, world.event)
        .await
        .unwrap();
    assert!(rows.is_empty());
    assert!(!world.initialized(&tx, world.p1).await);
}

/// The entries go through the signing log outbox once each, after the
/// tally's commit.
#[tokio::test]
async fn each_country_is_staged_once_in_the_electoral_log_outbox() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "post-and-country").await;
    let (session, _) = world
        .report(&tx, world.p1, &[world.a], true, "hash-a")
        .await;
    world.store(&tx, session, world.p1).await;
    tx.commit().await.unwrap();

    let mut client = pool.get().await.unwrap();
    assert_eq!(
        stage_initialization_log(&mut client, world.tenant, world.event)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        stage_initialization_log(&mut client, world.tenant, world.event)
            .await
            .unwrap(),
        0
    );
    let initialization: Uuid = client
        .query_one(
            "SELECT id FROM sequent_backend.election_initialization WHERE election_event_id = $1",
            &[&world.event],
        )
        .await
        .unwrap()
        .get(0);
    let rows = client
        .query(
            "SELECT election_id, area_id, body->>'description', step_id
             FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND statement_kind = 'ElectionInitialized'",
            &[&world.event],
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 2, "one USER and one SYSTEM entry");
    for row in &rows {
        assert_eq!(row.get::<_, Option<Uuid>>(0), Some(world.p1));
        assert_eq!(row.get::<_, Option<Uuid>>(1), Some(world.a));
        assert_eq!(row.get::<_, Uuid>(3), initialization);
        let description: String = row.get(2);
        assert!(
            description.starts_with("Initialized Post Dubai PCG, country United Arab Emirates at "),
            "{description}"
        );
        assert!(description.ends_with("with initialization report hash hash-a."));
    }
}

/// A scheduled opening row of `post` (`None`: event-wide) at `date`.
async fn scheduled(
    tx: &Transaction<'_>,
    world: &World,
    processor: &str,
    post: Option<Uuid>,
    date: &str,
) -> sequent_core::types::scheduled_event::ScheduledEvent {
    let id = Uuid::new_v4();
    execute(
        tx,
        "INSERT INTO sequent_backend.scheduled_event
             (id, tenant_id, election_event_id, event_processor, cron_config, event_payload)
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &id,
            &world.tenant,
            &world.event,
            &processor,
            &json!({ "scheduled_date": date }),
            &json!({ "election_id": post }),
        ],
    )
    .await;
    find(tx, world, id).await
}

async fn find(
    tx: &Transaction<'_>,
    world: &World,
    id: Uuid,
) -> sequent_core::types::scheduled_event::ScheduledEvent {
    windmill::postgres::scheduled_event::find_scheduled_event_by_id(
        tx,
        Some(world.tenant.to_string()),
        Some(world.event.to_string()),
        &id.to_string(),
    )
    .await
    .unwrap()
    .unwrap()
}

async fn opening_log(tx: &Transaction<'_>, world: &World) -> Vec<String> {
    tx.query(
        "SELECT body->>'description' FROM sequent_backend.signing_log_outbox
         WHERE election_event_id = $1 AND statement_kind = 'SigningActionExecuted'
           AND event_type = 'SYSTEM'
         ORDER BY id",
        &[&world.event],
    )
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.get(0))
    .collect()
}

async fn post(
    tx: &Transaction<'_>,
    world: &World,
    id: Uuid,
) -> sequent_core::types::hasura::core::Election {
    windmill::postgres::election::get_election_by_id(
        tx,
        &world.tenant.to_string(),
        &world.event.to_string(),
        &id.to_string(),
    )
    .await
    .unwrap()
    .unwrap()
}

#[tokio::test]
async fn a_scheduled_opening_waits_for_the_countries_and_logs_it_once() {
    use windmill::services::initialization_schedule::{
        scheduled_post_opening, ScheduledPostOpening,
    };
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "post-and-country").await;
    let (session, _) = world
        .report(&tx, world.p1, &[world.a], true, "hash-a")
        .await;
    world.store(&tx, session, world.p1).await;
    // Initialized as a Post only once Oman is: mark the flag to test the scope.
    tx.execute(
        "UPDATE sequent_backend.election SET initialization_report_generated = true WHERE id = $1",
        &[&world.p1],
    )
    .await
    .unwrap();
    let start = scheduled(
        &tx,
        &world,
        "START_VOTING_PERIOD",
        Some(world.p1),
        "2028-04-08T20:00:00Z",
    )
    .await;
    let opening = |start| {
        let (tx, world) = (&tx, &world);
        async move {
            let election = post(tx, world, world.p1).await;
            let start = find(tx, world, start).await;
            scheduled_post_opening(
                tx,
                &world.tenant.to_string(),
                &world.event.to_string(),
                &election,
                &start,
            )
            .await
            .unwrap()
        }
    };
    let start_id = Uuid::parse_str(&start.id).unwrap();
    assert_eq!(opening(start_id).await, ScheduledPostOpening::Wait);
    assert_eq!(opening(start_id).await, ScheduledPostOpening::Wait);
    let log = opening_log(&tx, &world).await;
    assert_eq!(log.len(), 1, "logged once: {log:?}");
    assert!(log[0].starts_with("Scheduled opening of Post Dubai PCG waits for its initialization:"));
    assert!(log[0].contains("Not initialized yet: Oman."), "{}", log[0]);

    let (session, _) = world
        .report(&tx, world.p1, &[world.b], true, "hash-b")
        .await;
    world.store(&tx, session, world.p1).await;
    assert_eq!(opening(start_id).await, ScheduledPostOpening::Open);

    // After the Post's close, the opening doesn't run.
    scheduled(
        &tx,
        &world,
        "END_VOTING_PERIOD",
        None,
        "2000-01-01T00:00:00Z",
    )
    .await;
    assert_eq!(opening(start_id).await, ScheduledPostOpening::AfterClose);
    let log = opening_log(&tx, &world).await;
    assert_eq!(log.len(), 2);
    assert_eq!(
        log[1],
        "Scheduled opening of Post Dubai PCG not run: its voting period closed at 2000-01-01T00:00:00+00:00."
    );
}

#[tokio::test]
async fn an_event_wide_opening_keeps_waiting_for_the_posts_that_cant_open() {
    use sequent_core::ballot::VotingStatusChannel;
    use windmill::services::initialization_schedule::open_event_on_schedule;
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = World::new(&tx, "event").await;
    tx.execute(
        "UPDATE sequent_backend.election SET voting_channels = '{\"online\": true}'
         WHERE election_event_id = $1",
        &[&world.event],
    )
    .await
    .unwrap();
    let (session, _) = world
        .report(&tx, world.p1, &[world.a, world.b], false, "hash-1")
        .await;
    world.store(&tx, session, world.p1).await;
    let start = scheduled(
        &tx,
        &world,
        "START_VOTING_PERIOD",
        None,
        "2028-04-08T20:00:00Z",
    )
    .await;
    let id = Uuid::parse_str(&start.id).unwrap();
    let run = || {
        let (tx, world) = (&tx, &world);
        async move {
            let start = find(tx, world, id).await;
            open_event_on_schedule(
                tx,
                &world.tenant.to_string(),
                &world.event.to_string(),
                &start,
                &Some(vec![VotingStatusChannel::ONLINE]),
                &std::collections::HashSet::new(),
            )
            .await
            .unwrap()
        }
    };
    // Neither Post opens: p2 isn't initialized, so p1 waits for it.
    assert!(!run().await);
    assert!(!run().await);
    let log = opening_log(&tx, &world).await;
    assert_eq!(log.len(), 2, "one per Post, once: {log:?}");
    assert!(log
        .iter()
        .any(|entry| entry.contains("Post Dubai PCG waits")
            && entry.contains("Not initialized yet: Madrid.")));
    assert!(log.iter().any(|entry| entry.contains("Post Madrid waits")));
    let pending =
        find(&tx, &world, id).await.annotations.unwrap()["initialization_pending"].clone();
    let mut pending: Vec<String> = serde_json::from_value(pending).unwrap();
    pending.sort();
    let mut expected = vec![world.p1.to_string(), world.p2.to_string()];
    expected.sort();
    assert_eq!(pending, expected);

    // Once the event-wide close has passed, the opening stops and says so.
    scheduled(
        &tx,
        &world,
        "END_VOTING_PERIOD",
        None,
        "2000-01-01T00:00:00Z",
    )
    .await;
    assert!(run().await);
    let log = opening_log(&tx, &world).await;
    assert_eq!(log.len(), 4);
    assert!(log[2..]
        .iter()
        .all(|entry| entry.contains("not run: its voting period closed")));
}

#[tokio::test]
async fn published_country_requirements_survive_live_loosening_and_are_per_target() {
    use sequent_core::ballot::InitializationScope;
    use sequent_core::types::scheduled_outcome::LifecycleSnapshot;
    use windmill::services::scheduled_outcome::write_publication_snapshot;
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "post").await;
    execute(&tx, "UPDATE sequent_backend.election SET initialization_report_generated = true WHERE election_event_id = $1", &[&w.event]).await;
    let mut signed = LifecycleSnapshot::default();
    signed.policies.initialization_scope = InitializationScope::POST_AND_COUNTRY;
    write_publication_snapshot(&tx, w.tenant, w.event, Uuid::new_v4(), None, None, &signed)
        .await
        .unwrap();
    let event = get_election_event_by_id(&tx, &w.tenant.to_string(), &w.event.to_string())
        .await
        .unwrap();
    let elections = get_elections(&tx, &w.tenant.to_string(), &w.event.to_string())
        .await
        .unwrap();
    let blocked = initialization_refusals(&tx, &event, &elections)
        .await
        .unwrap();
    assert!(matches!(
        blocked.get(&w.p1.to_string()),
        Some(TransitionRefusal::CountriesNotInitialized { .. })
    ));
    assert!(blocked.contains_key(&w.p2.to_string()));
    write_publication_snapshot(
        &tx,
        w.tenant,
        w.event,
        Uuid::new_v4(),
        Some(w.p1),
        None,
        &LifecycleSnapshot::default(),
    )
    .await
    .unwrap();
    let blocked = initialization_refusals(&tx, &event, &elections)
        .await
        .unwrap();
    assert!(!blocked.contains_key(&w.p1.to_string()));
    assert!(blocked.contains_key(&w.p2.to_string()));
}

#[tokio::test]
async fn initialization_completion_changes_waiting_prediction_once() {
    use sequent_core::types::scheduled_outcome::ScheduledOutcomeKind;
    use windmill::services::scheduled_outcome::{recompute_predictions, scheduled_outcomes};
    use windmill::services::signing::log::Actor;
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "post-and-country").await;
    let start = scheduled(
        &tx,
        &w,
        "START_VOTING_PERIOD",
        Some(w.p1),
        "2099-04-09T00:00:00Z",
    )
    .await;
    let actor = Actor {
        user_id: "operator".into(),
        username: "operator".into(),
    };
    recompute_predictions(&tx, &w.tenant.to_string(), &w.event.to_string(), &actor)
        .await
        .unwrap();
    let predicted = scheduled_outcomes(&tx, w.tenant, w.event).await.unwrap();
    assert_eq!(
        predicted
            .iter()
            .find(|row| row.scheduled_event_id == start.id)
            .unwrap()
            .explanation
            .outcome,
        ScheduledOutcomeKind::WaitingForInitialization
    );
    let (session, _) = w
        .report(&tx, w.p1, &[w.a, w.b], false, "complete-hash")
        .await;
    store_initialization(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &session.to_string(),
        &w.p1.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    stage_initialization_log(&mut client, w.tenant, w.event)
        .await
        .unwrap();
    let tx = client.transaction().await.unwrap();
    let predicted = scheduled_outcomes(&tx, w.tenant, w.event).await.unwrap();
    assert_eq!(
        predicted
            .iter()
            .find(|row| row.scheduled_event_id == start.id)
            .unwrap()
            .explanation
            .outcome,
        ScheduledOutcomeKind::Runs
    );
    let count: i64 = tx.query_one("SELECT count(*) FROM sequent_backend.signing_log_outbox WHERE election_event_id = $1 AND statement_kind = 'ScheduledOutcomeChanged'", &[&w.event]).await.unwrap().get(0);
    assert_eq!(count, 4); // creation USER/SYSTEM, then completion USER/SYSTEM
    let fired: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.lifecycle_fired WHERE election_event_id = $1",
            &[&w.event],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(fired, 0);
    tx.commit().await.unwrap();
    stage_initialization_log(&mut client, w.tenant, w.event)
        .await
        .unwrap();
    let after: i64 = client.query_one("SELECT count(*) FROM sequent_backend.signing_log_outbox WHERE election_event_id = $1 AND statement_kind = 'ScheduledOutcomeChanged'", &[&w.event]).await.unwrap().get(0);
    assert_eq!(after, count);
}

#[tokio::test]
async fn changing_a_live_report_policy_cannot_skip_the_published_report_requirement() {
    use sequent_core::ballot::EInitializeReportPolicy;
    use windmill::services::scheduled_outcome::{lifecycle_snapshot, write_publication_snapshot};
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "post").await;
    let required = lifecycle_snapshot(&tx, w.tenant, w.event, Some(w.p1))
        .await
        .unwrap();
    assert_eq!(
        required
            .initialization_report_policies
            .get(&w.p1.to_string()),
        Some(&EInitializeReportPolicy::REQUIRED)
    );
    write_publication_snapshot(
        &tx,
        w.tenant,
        w.event,
        Uuid::new_v4(),
        Some(w.p1),
        None,
        &required,
    )
    .await
    .unwrap();
    tx.execute("UPDATE sequent_backend.election SET presentation = jsonb_set(presentation, '{initialization_report_policy}', '\"not-required\"') WHERE id = $1", &[&w.p1]).await.unwrap();
    let event = get_election_event_by_id(&tx, &w.tenant.to_string(), &w.event.to_string())
        .await
        .unwrap();
    let elections = get_elections(&tx, &w.tenant.to_string(), &w.event.to_string())
        .await
        .unwrap();
    let refusals = initialization_refusals(&tx, &event, &elections)
        .await
        .unwrap();
    assert!(matches!(
        refusals.get(&w.p1.to_string()),
        Some(TransitionRefusal::InitializationReportRequired)
    ));
    let replacement = lifecycle_snapshot(&tx, w.tenant, w.event, Some(w.p1))
        .await
        .unwrap();
    write_publication_snapshot(
        &tx,
        w.tenant,
        w.event,
        Uuid::new_v4(),
        Some(w.p1),
        None,
        &replacement,
    )
    .await
    .unwrap();
    let refusals = initialization_refusals(&tx, &event, &elections)
        .await
        .unwrap();
    assert!(!refusals.contains_key(&w.p1.to_string()));
}

#[tokio::test]
async fn missing_initialization_artifacts_never_create_evidence_or_open_the_gate() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "post").await;
    let (session, _) = w.report(&tx, w.p1, &[w.a, w.b], false, "report-hash").await;
    tx.execute(
        "UPDATE sequent_backend.results_election SET documents = NULL WHERE election_id = $1",
        &[&w.p1],
    )
    .await
    .unwrap();
    let error = store_initialization(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &session.to_string(),
        &w.p1.to_string(),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("missing its hash or document"));
    assert!(!post(&tx, &w, w.p1)
        .await
        .initialization_report_generated
        .unwrap_or(false));
    assert!(list_election_initializations(&tx, w.tenant, w.event)
        .await
        .unwrap()
        .is_empty());
    let event = get_election_event_by_id(&tx, &w.tenant.to_string(), &w.event.to_string())
        .await
        .unwrap();
    let elections = get_elections(&tx, &w.tenant.to_string(), &w.event.to_string())
        .await
        .unwrap();
    let refusal = initialization_refusals(&tx, &event, &elections)
        .await
        .unwrap();
    assert!(matches!(
        refusal.get(&w.p1.to_string()),
        Some(TransitionRefusal::InitializationReportRequired)
    ));
}

#[tokio::test]
async fn batch_preview_preserves_dispatch_identity_and_stopped_rearming() {
    use sequent_core::types::scheduled_event::{generate_manage_date_task_name, EventProcessors};
    use windmill::services::scheduled_outcome::{preview_changes, EventState, PendingChange};
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "post").await;
    let dummy = scheduled(
        &tx,
        &w,
        "START_VOTING_PERIOD",
        Some(w.p1),
        "2099-01-01T00:00:00Z",
    )
    .await;
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id='dummy' WHERE id=$1",
        &[&Uuid::parse_str(&dummy.id).unwrap()],
    )
    .await
    .unwrap();
    let global = scheduled(&tx, &w, "START_VOTING_PERIOD", None, "2099-01-01T00:00:00Z").await;
    let state = EventState::read(&tx, w.tenant, w.event).await.unwrap();
    let global_transition = windmill::services::scheduled_outcome::transition_of(
        &global.id,
        "START_VOTING_PERIOD",
        Some(&json!({"scheduled_date":"2099-01-01T00:00:00Z"})),
        None,
    )
    .unwrap();
    let global_row = windmill::services::scheduled_outcome::ScheduledRow {
        transition: global_transition,
        annotations: json!({}),
        written_now: false,
    };
    assert!(state.posts_of(&global_row).contains(&w.p1));
    let changes = [PendingChange::ScheduledEvent {
        id: Some("preview-new".into()),
        event_processor: "START_VOTING_PERIOD".into(),
        cron_config: Some(json!({"scheduled_date":"2099-01-02T00:00:00Z"})),
        event_payload: None,
    }];
    let (after, _, _) =
        windmill::services::scheduled_outcome::apply_change(&state, &state.live_rows, &changes[0]);
    assert!(
        after.posts_of(&global_row).contains(&w.p1),
        "An unrelated preview cannot promote a dummy task to ownership: own_rows={:?}, metadata={:?}, live_rows={:?}", after.own_rows, after.live_row_metadata, after.live_rows
    );
    let preview = preview_changes(&tx, w.tenant, w.event, &changes)
        .await
        .unwrap();
    assert!(!preview
        .iter()
        .any(|change| change.scheduled_event_id == global.id && change.election_id == Some(w.p1)));
    let task = generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.p1.to_string()),
        &EventProcessors::START_VOTING_PERIOD,
    );
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id=$2, stopped_at=NOW() WHERE id=$1",
        &[&Uuid::parse_str(&dummy.id).unwrap(), &task],
    )
    .await
    .unwrap();
    for (date, expected) in [
        ("2000-01-01T00:00:00Z", false),
        ("2099-01-03T00:00:00Z", true),
    ] {
        let edit = PendingChange::ScheduledEvent {
            id: Some(dummy.id.clone()),
            event_processor: "START_VOTING_PERIOD".into(),
            cron_config: Some(json!({"scheduled_date":date})),
            event_payload: Some(json!({"election_id":w.p1})),
        };
        let preview = preview_changes(&tx, w.tenant, w.event, &[edit])
            .await
            .unwrap();
        assert_eq!(
            preview
                .iter()
                .any(|change| change.scheduled_event_id == dummy.id),
            expected
        );
    }
}

#[tokio::test]
async fn late_initialization_cannot_escape_a_common_close_via_dummy_or_kiosk_rows() {
    use sequent_core::types::scheduled_event::{generate_manage_date_task_name, EventProcessors};
    use windmill::services::initialization_schedule::{
        scheduled_post_opening, ScheduledPostOpening,
    };
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "post").await;
    tx.execute(
        "UPDATE sequent_backend.election SET initialization_report_generated=true WHERE id=$1",
        &[&w.p1],
    )
    .await
    .unwrap();
    let start = scheduled(
        &tx,
        &w,
        "START_VOTING_PERIOD",
        Some(w.p1),
        "2000-01-01T00:00:00Z",
    )
    .await;
    let common = scheduled(&tx, &w, "END_VOTING_PERIOD", None, "2000-01-02T00:00:00Z").await;
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET stopped_at=NOW() WHERE id=$1",
        &[&Uuid::parse_str(&common.id).unwrap()],
    )
    .await
    .unwrap();
    let own = scheduled(
        &tx,
        &w,
        "END_VOTING_PERIOD",
        Some(w.p1),
        "2099-01-01T00:00:00Z",
    )
    .await;
    let expected_task = generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.p1.to_string()),
        &EventProcessors::END_VOTING_PERIOD,
    );
    for (task, channels) in [
        ("dummy".to_owned(), json!(["ONLINE"])),
        (expected_task, json!(["KIOSK"])),
    ] {
        tx.execute("UPDATE sequent_backend.scheduled_event SET task_id=$2, event_payload=event_payload || jsonb_build_object('voting_channels',$3::jsonb) WHERE id=$1", &[&Uuid::parse_str(&own.id).unwrap(), &task, &channels]).await.unwrap();
        let election = post(&tx, &w, w.p1).await;
        assert_eq!(
            scheduled_post_opening(
                &tx,
                &w.tenant.to_string(),
                &w.event.to_string(),
                &election,
                &start
            )
            .await
            .unwrap(),
            ScheduledPostOpening::AfterClose
        );
    }
}

#[tokio::test]
async fn invalid_initialization_topology_cannot_block_closing() {
    use sequent_core::types::scheduled_outcome::ScheduledOutcomeKind;
    use windmill::services::scheduled_outcome::{fire_time_state, EventState};
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, "post").await;
    tx.execute(
        "UPDATE sequent_backend.area SET parent_id=id WHERE id=$1",
        &[&w.a],
    )
    .await
    .unwrap();
    let error = EventState::read(&tx, w.tenant, w.event).await.unwrap_err();
    assert!(format!("{error:#}").contains("Loop detected"));
    let close = scheduled(
        &tx,
        &w,
        "END_VOTING_PERIOD",
        Some(w.p1),
        "2000-01-01T00:00:00Z",
    )
    .await;
    let (state, row) = fire_time_state(&tx, w.tenant, w.event, &close.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        state
            .explain(
                &row,
                Some(w.p1),
                windmill::services::scheduled_outcome::Moment::FireTime
            )
            .outcome,
        ScheduledOutcomeKind::Runs
    );
    assert!(EventState::read_for_closes(&tx, w.tenant, w.event)
        .await
        .is_ok());
}
