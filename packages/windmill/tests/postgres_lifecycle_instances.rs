// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! One process of a component stopping or starting, against a private migrated
//! Hasura database and the real functions: a failure only when no process of the
//! component is left. Other processes are rows, renewed here at the real cadence,
//! and the waits are the real ones, so this binary takes over two minutes.
//!
//! `get_hasura_pool` is a process static whose connections belong to the runtime
//! that opened them, and the scenarios share the lease table, so they run in
//! sequence in one test.
//!
//! Needs `HASURA_DB__*`, `KEYCLOAK_DB__*`, `LOW_SQL_LIMIT`, `DEFAULT_SQL_LIMIT` and
//! `DEFAULT_SQL_BATCH_SIZE`.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use chrono::Utc;
use deadpool_postgres::Pool;
use sequent_core::self_tests::{
    SelfTestCheck, SelfTestCheckResult, SelfTestOutcome, SelfTestRun, SelfTestTrigger,
};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;
use windmill::postgres::trusted_write;
use windmill::services::election_lifecycle::{
    heartbeat, instance_stopped, process_instance, register_instance,
};
use windmill::services::self_tests::{audit_when_free, claim_run, execute, reset_interrupted_runs};

const WINDMILL: &str = "windmill";
const HARVEST: &str = "harvest";
const STOPPED: &str = "A required service stopped or restarted";
const PROOF_WAIT: Duration = Duration::from_secs(12);

struct World {
    pool: Pool,
    tenant: String,
    event: String,
}

/// What the renewal task of other processes does, until dropped.
struct Renewing(tokio::task::JoinHandle<()>);

impl Drop for Renewing {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl World {
    async fn new() -> Self {
        let pool = schema::pool().await;
        let client = pool.get().await.unwrap();
        let database: String = client
            .query_one("SELECT current_database()", &[])
            .await
            .unwrap()
            .get(0);
        std::env::set_var("HASURA_DB__DBNAME", database);
        let (tenant, event) = (Uuid::new_v4(), Uuid::new_v4());
        client
            .execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
                &[&tenant, &format!("tenant-{tenant}")],
            )
            .await
            .unwrap();
        client
            .execute(
                "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
                 VALUES ($1, $2, 'RSA256')",
                &[&event, &tenant],
            )
            .await
            .unwrap();
        drop(client);
        World {
            pool,
            tenant: tenant.to_string(),
            event: event.to_string(),
        }
    }

    /// No process, both leases live, the event operational in execution.
    async fn reset(&self) {
        let mut client = self.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        trusted_write(&tx).await.unwrap();
        tx.batch_execute(
            "DELETE FROM sequent_backend.lifecycle_service_instance;
             DELETE FROM sequent_backend.lifecycle_service_lease;
             INSERT INTO sequent_backend.lifecycle_service_lease(component,expires_at)
             VALUES ('harvest',clock_timestamp()+interval '45 seconds'),
                    ('windmill',clock_timestamp()+interval '45 seconds');",
        )
        .await
        .unwrap();
        tx.execute(
            "UPDATE sequent_backend.election_lifecycle
             SET phase='execution',safety='operational',reason=NULL
             WHERE election_event_id=$1::text::uuid",
            &[&self.event],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    /// The row of another process, started and last renewed this long ago.
    async fn process(&self, component: &str, started_ago: i32, renewed_ago: i32) -> String {
        let instance = Uuid::new_v4().to_string();
        self.pool
            .get()
            .await
            .unwrap()
            .execute(
                "INSERT INTO sequent_backend.lifecycle_service_instance
                 (component,instance,started_at,renewed_at)
                 VALUES ($1,$2::text::uuid,clock_timestamp()-make_interval(secs=>$3),
                         clock_timestamp()-make_interval(secs=>$4))",
                &[
                    &component,
                    &instance,
                    &f64::from(started_ago),
                    &f64::from(renewed_ago),
                ],
            )
            .await
            .unwrap();
        instance
    }

    fn renewing(&self, instances: &[&str]) -> Renewing {
        let pool = self.pool.clone();
        let instances: Vec<Uuid> = instances
            .iter()
            .map(|instance| Uuid::parse_str(instance).unwrap())
            .collect();
        Renewing(tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                pool.get()
                    .await
                    .unwrap()
                    .execute(
                        "UPDATE sequent_backend.lifecycle_service_instance
                         SET renewed_at=clock_timestamp() WHERE instance=ANY($1)",
                        &[&instances],
                    )
                    .await
                    .unwrap();
            }
        }))
    }

    async fn lease_live(&self, component: &str) -> bool {
        self.pool
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM sequent_backend.lifecycle_service_lease
                 WHERE component=$1 AND expires_at>clock_timestamp())",
                &[&component],
            )
            .await
            .unwrap()
            .get(0)
    }

    async fn expire_lease(&self, component: &str) {
        self.pool
            .get()
            .await
            .unwrap()
            .execute(
                "UPDATE sequent_backend.lifecycle_service_lease
                 SET expires_at=clock_timestamp()-interval '1 second' WHERE component=$1",
                &[&component],
            )
            .await
            .unwrap();
    }

    async fn has_row(&self, instance: &str) -> bool {
        self.pool
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM sequent_backend.lifecycle_service_instance
                 WHERE instance=$1::text::uuid)",
                &[&instance],
            )
            .await
            .unwrap()
            .get(0)
    }

    async fn safety(&self) -> (String, Option<String>) {
        let row = self
            .pool
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT safety,reason FROM sequent_backend.election_lifecycle
                 WHERE election_event_id=$1::text::uuid",
                &[&self.event],
            )
            .await
            .unwrap();
        (row.get(0), row.get(1))
    }

    async fn assert_undisturbed(&self, component: &str, scenario: &str) {
        assert_eq!(
            self.safety().await,
            ("operational".to_string(), None),
            "{scenario}"
        );
        assert!(self.lease_live(component).await, "{scenario}");
    }

    async fn assert_failed(&self, scenario: &str) {
        assert_eq!(
            self.safety().await,
            ("recoverable".to_string(), Some(STOPPED.to_string())),
            "{scenario}"
        );
    }

    /// An unfinished run of the event.
    async fn queue_run(&self, checks: Vec<SelfTestCheckResult>) -> String {
        let id = Uuid::new_v4().to_string();
        let run = SelfTestRun {
            run_id: id.clone(),
            generation: 0,
            trigger: SelfTestTrigger::Periodic,
            started_at: Utc::now().to_rfc3339(),
            finished_at: None,
            outcome: SelfTestOutcome::RecoverableFailure,
            checks,
        };
        self.pool
            .get()
            .await
            .unwrap()
            .execute(
                "INSERT INTO sequent_backend.self_test_run
                 (id,tenant_id,election_event_id,generation,trigger,result,started_at)
                 VALUES ($1::text::uuid,$2::text::uuid,$3::text::uuid,0,'periodic',$4,
                         now()-interval '1 minute')",
                &[
                    &id,
                    &self.tenant,
                    &self.event,
                    &serde_json::to_value(&run).unwrap(),
                ],
            )
            .await
            .unwrap();
        id
    }

    async fn finish_run(&self, id: &str) {
        self.pool
            .get()
            .await
            .unwrap()
            .execute(
                "UPDATE sequent_backend.self_test_run SET finished_at=now(),outcome='passed'
                 WHERE id=$1::text::uuid",
                &[&id],
            )
            .await
            .unwrap();
    }

    /// (process executing it, repeats, repeated within the last minute, checks)
    async fn run_row(&self, id: &str) -> (Option<String>, i16, bool, usize) {
        let row = self
            .pool
            .get()
            .await
            .unwrap()
            .query_one(
                "SELECT executing_instance::text,repeats,started_at>now()-interval '30 seconds',
                 result FROM sequent_backend.self_test_run WHERE id=$1::text::uuid",
                &[&id],
            )
            .await
            .unwrap();
        let run: SelfTestRun = serde_json::from_value(row.get(3)).unwrap();
        (row.get(0), row.get(1), row.get(2), run.checks.len())
    }

    async fn claim(&self, id: &str, instance: &str) -> bool {
        claim_run(&self.tenant, &self.event, id, instance)
            .await
            .unwrap()
    }

    fn reset_entry(&self, id: &str) -> Vec<(String, String, String)> {
        vec![(self.tenant.clone(), self.event.clone(), id.to_string())]
    }
}

fn check(outcome: SelfTestOutcome) -> SelfTestCheckResult {
    SelfTestCheckResult {
        check: SelfTestCheck::BallotBox,
        outcome,
        detail: "synthetic".into(),
    }
}

/// The first registration of a process starts its renewal, which here is the
/// only real one: a row is renewed, also while every worker of the process's
/// runtime is busy, stays gone once removed, and an abandoned row is cleaned up.
async fn renewal(w: &World) {
    w.reset().await;
    let instance = Uuid::new_v4().to_string();
    let abandoned = w.process(WINDMILL, 7300, 7300).await;
    register_instance(WINDMILL, &instance).await.unwrap();
    w.assert_failed("nothing recently renewed").await;
    tokio::time::sleep(Duration::from_secs(7)).await;
    let renewed: bool = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT renewed_at>started_at+interval '4 seconds'
             FROM sequent_backend.lifecycle_service_instance WHERE instance=$1::text::uuid",
            &[&instance],
        )
        .await
        .unwrap()
        .get(0);
    assert!(renewed, "the renewal task sets renewed_at");
    assert!(!w.has_row(&abandoned).await, "an abandoned row is removed");

    // Every worker of this runtime is busy for longer than a process counts as
    // alive without a renewal. An observer with a runtime of its own still finds
    // the row recently renewed.
    let runtime = tokio::runtime::Handle::current();
    let blockers = std::thread::spawn(move || {
        (0..4)
            .map(|_| runtime.spawn(async { std::thread::sleep(Duration::from_secs(8)) }))
            .collect::<Vec<_>>()
    })
    .join()
    .unwrap();
    let observed = std::thread::spawn({
        let instance = instance.clone();
        move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    tokio::time::sleep(Duration::from_secs(7)).await;
                    schema::pool()
                        .await
                        .get()
                        .await
                        .unwrap()
                        .query_one(
                            "SELECT renewed_at>clock_timestamp()-interval '6 seconds'
                             FROM sequent_backend.lifecycle_service_instance
                             WHERE instance=$1::text::uuid",
                            &[&instance],
                        )
                        .await
                        .unwrap()
                        .get::<_, bool>(0)
                })
        }
    });
    for blocker in blockers {
        blocker.await.unwrap();
    }
    assert!(
        observed.join().unwrap(),
        "a busy runtime does not delay the renewal"
    );

    instance_stopped(WINDMILL, &instance).await.unwrap();
    tokio::time::sleep(Duration::from_secs(6)).await;
    assert!(
        !w.has_row(&instance).await,
        "a renewal cannot bring back a stopped process"
    );
}

async fn stops(w: &World) {
    w.reset().await;
    let (me, other) = (
        w.process(WINDMILL, 60, 0).await,
        w.process(WINDMILL, 60, 14).await,
    );
    instance_stopped(WINDMILL, &me).await.unwrap();
    assert!(!w.has_row(&me).await);
    assert!(w.has_row(&other).await);
    w.assert_undisturbed(WINDMILL, "another process renewed within 15 s")
        .await;

    w.reset().await;
    let me = w.process(WINDMILL, 60, 0).await;
    instance_stopped(WINDMILL, &me).await.unwrap();
    w.assert_failed("the last process").await;
    assert!(
        !w.lease_live(WINDMILL).await,
        "the last process ends the lease"
    );

    w.reset().await;
    let me = w.process(WINDMILL, 60, 0).await;
    w.process(WINDMILL, 60, 16).await;
    instance_stopped(WINDMILL, &me).await.unwrap();
    w.assert_failed("the other process stopped renewing").await;

    w.reset().await;
    let me = w.process(HARVEST, 60, 0).await;
    w.process(WINDMILL, 60, 0).await;
    instance_stopped(HARVEST, &me).await.unwrap();
    w.assert_failed("another component's process proves nothing")
        .await;
    assert!(w.lease_live(WINDMILL).await, "only its own lease ends");

    // Either may lose the event's lock to the other, as to any writer: one of
    // them records the failure, and an error ends a process that was stopping.
    for _ in 0..5 {
        w.reset().await;
        let (a, b) = (
            w.process(WINDMILL, 60, 0).await,
            w.process(WINDMILL, 60, 0).await,
        );
        let _ = tokio::join!(
            instance_stopped(WINDMILL, &a),
            instance_stopped(WINDMILL, &b)
        );
        w.assert_failed("two stopping at once").await;
        assert!(!w.lease_live(WINDMILL).await);
    }

    assert!(instance_stopped("other", &Uuid::new_v4().to_string())
        .await
        .is_err());
}

async fn starts(w: &World) {
    w.reset().await;
    let begun = Instant::now();
    register_instance(WINDMILL, &Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(begun.elapsed() < PROOF_WAIT, "no recent row means no wait");
    w.assert_failed("no other process").await;
    assert!(
        w.lease_live(WINDMILL).await,
        "its heartbeat renews the lease"
    );

    w.reset().await;
    w.process(WINDMILL, 60, 16).await;
    let begun = Instant::now();
    register_instance(WINDMILL, &Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(begun.elapsed() < PROOF_WAIT, "a stale row means no wait");
    w.assert_failed("only a stale row").await;

    w.reset().await;
    let running = w.process(WINDMILL, 60, 1).await;
    let _renewals = w.renewing(&[&running]);
    let begun = Instant::now();
    register_instance(WINDMILL, &Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(begun.elapsed() >= PROOF_WAIT);
    w.assert_undisturbed(WINDMILL, "a process already running keeps renewing")
        .await;
    drop(_renewals);

    w.reset().await;
    w.process(WINDMILL, 60, 1).await;
    let begun = Instant::now();
    register_instance(WINDMILL, &Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(begun.elapsed() >= PROOF_WAIT);
    w.assert_failed("a process that just crashed").await;
    assert!(w.lease_live(WINDMILL).await);

    // A process started shortly before this one may be waiting for the same proof.
    w.reset().await;
    let young = w.process(WINDMILL, 5, 0).await;
    let _renewals = w.renewing(&[&young]);
    register_instance(WINDMILL, &Uuid::new_v4().to_string())
        .await
        .unwrap();
    w.assert_failed("a process that started too recently").await;
    drop(_renewals);

    // After a crash, those starting together do not vouch for each other.
    w.reset().await;
    w.process(WINDMILL, 60, 1).await;
    let (a, b) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    let _renewals = w.renewing(&[&a, &b]);
    let (first, second) = tokio::join!(
        register_instance(WINDMILL, &a),
        register_instance(WINDMILL, &b)
    );
    assert!(first.is_ok() || second.is_ok());
    w.assert_failed("several starting together").await;
    drop(_renewals);

    w.reset().await;
    let running = w.process(WINDMILL, 60, 1).await;
    let _renewals = w.renewing(&[&running]);
    w.expire_lease(WINDMILL).await;
    register_instance(WINDMILL, &Uuid::new_v4().to_string())
        .await
        .unwrap();
    w.assert_failed("the component stopped working: no live lease")
        .await;
    assert!(w.lease_live(WINDMILL).await);
    drop(_renewals);

    // Harvest retries a first registration that ran into its deadline.
    w.reset().await;
    let running = w.process(HARVEST, 60, 1).await;
    let _renewals = w.renewing(&[&running]);
    let me = Uuid::new_v4().to_string();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), register_instance(HARVEST, &me))
            .await
            .is_err()
    );
    let started_at = |client: deadpool_postgres::Object, me: String| async move {
        client
            .query_one(
                "SELECT started_at::text FROM sequent_backend.lifecycle_service_instance
                 WHERE instance=$1::text::uuid",
                &[&me],
            )
            .await
            .unwrap()
            .get::<_, String>(0)
    };
    let first = started_at(w.pool.get().await.unwrap(), me.clone()).await;
    register_instance(HARVEST, &me).await.unwrap();
    assert_eq!(first, started_at(w.pool.get().await.unwrap(), me).await);
    w.assert_undisturbed(HARVEST, "a retried registration")
        .await;
    drop(_renewals);

    assert!(register_instance("other", &Uuid::new_v4().to_string())
        .await
        .is_err());
}

async fn expired_lease(w: &World) {
    w.reset().await;
    let running = w.process(WINDMILL, 60, 0).await;
    heartbeat(WINDMILL).await.unwrap();
    w.assert_undisturbed(WINDMILL, "an ordinary heartbeat")
        .await;
    w.expire_lease(WINDMILL).await;
    heartbeat(WINDMILL).await.unwrap();
    w.assert_failed("an expired lease, with a process still renewing")
        .await;
    assert!(w.has_row(&running).await);
    assert!(w.lease_live(WINDMILL).await);
}

/// An update's order of events, and then the end of the last two processes.
async fn rolling_update(w: &World) {
    w.reset().await;
    let old1 = w.process(WINDMILL, 600, 0).await;
    let old2 = w.process(WINDMILL, 600, 0).await;
    let (new1, new2) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    let _renewals = w.renewing(&[&old1, &old2, &new1, &new2]);
    register_instance(WINDMILL, &new1).await.unwrap();
    w.assert_undisturbed(WINDMILL, "first replacement").await;
    instance_stopped(WINDMILL, &old1).await.unwrap();
    w.assert_undisturbed(WINDMILL, "first older process stops")
        .await;
    register_instance(WINDMILL, &new2).await.unwrap();
    w.assert_undisturbed(WINDMILL, "second replacement").await;
    instance_stopped(WINDMILL, &old2).await.unwrap();
    w.assert_undisturbed(WINDMILL, "second older process stops")
        .await;
    instance_stopped(WINDMILL, &new1).await.unwrap();
    w.assert_undisturbed(WINDMILL, "one process left").await;
    instance_stopped(WINDMILL, &new2).await.unwrap();
    w.assert_failed("none left").await;
}

async fn interrupted_runs(w: &World) {
    w.reset().await;
    let alive = w.process(WINDMILL, 60, 0).await;
    let later = Uuid::new_v4().to_string();

    let id = w
        .queue_run(vec![check(SelfTestOutcome::RecoverableFailure)])
        .await;
    assert!(
        reset_interrupted_runs().await.unwrap().is_empty(),
        "a run no process has begun waits for its message"
    );
    assert!(w.claim(&id, &alive).await);
    assert!(
        !w.claim(&id, &later).await,
        "a message delivered again does not begin a run again"
    );
    assert!(
        reset_interrupted_runs().await.unwrap().is_empty(),
        "its process is alive"
    );
    assert_eq!(
        w.run_row(&id).await,
        (Some(alive.clone()), 0, false, 1),
        "untouched while its process lives"
    );

    instance_stopped(WINDMILL, &alive).await.unwrap();
    w.reset().await;
    assert_eq!(
        reset_interrupted_runs().await.unwrap(),
        w.reset_entry(&id),
        "its process is gone"
    );
    assert_eq!(w.run_row(&id).await, (None, 1, true, 0), "starts over");
    assert_eq!(
        w.safety().await,
        ("operational".to_string(), None),
        "and enters no failure"
    );
    assert!(reset_interrupted_runs().await.unwrap().is_empty());

    assert!(w.claim(&id, &later).await, "the same run is taken again");
    assert_eq!(
        reset_interrupted_runs().await.unwrap(),
        w.reset_entry(&id),
        "a process with no row is gone"
    );
    assert_eq!(w.run_row(&id).await.1, 2);
    assert!(w.claim(&id, &later).await);
    assert!(
        reset_interrupted_runs().await.unwrap().is_empty(),
        "repeated too often: left for the stale sweep"
    );
    assert_eq!(w.run_row(&id).await, (Some(later.clone()), 2, true, 0));
    w.finish_run(&id).await;
    assert!(
        !w.claim(&id, &later).await,
        "a finished run cannot be taken"
    );

    let id = w
        .queue_run(vec![check(SelfTestOutcome::UnrecoverableFailure)])
        .await;
    assert!(w.claim(&id, &later).await);
    assert!(
        reset_interrupted_runs().await.unwrap().is_empty(),
        "an unrecoverable result stays for the stale sweep"
    );
    assert_eq!(w.run_row(&id).await, (Some(later), 0, false, 1));
    w.finish_run(&id).await;
}

/// A run's audit waits for another audit of the event to end, and then goes on.
async fn another_audit_running(w: &World) {
    let mut client = w.pool.get().await.unwrap();
    let other = client.transaction().await.unwrap();
    other
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("electoral-log-audit:{}:{}", w.tenant, w.event)],
        )
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_secs(3), audit_when_free(&w.tenant, &w.event))
            .await
            .is_err(),
        "no refusal while the other audit runs"
    );
    other.rollback().await.unwrap();
    let audit = timeout(
        Duration::from_secs(30),
        audit_when_free(&w.tenant, &w.event),
    )
    .await
    .expect("goes on once the other audit ended");
    let error = audit.expect_err("this event has no electoral-log board");
    assert!(
        error.to_string().contains("no electoral-log board"),
        "{error:#}"
    );
}

/// A run's only message waits a moment for the lock of the event's previous run,
/// and gives up without taking the run if it stays held.
async fn previous_run_finishing(w: &World) {
    w.reset().await;
    let id = w.queue_run(vec![]).await;
    let mut client = w.pool.get().await.unwrap();
    let previous = client.transaction().await.unwrap();
    previous
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("self-tests:execute:{}", w.event)],
        )
        .await
        .unwrap();
    execute(&w.tenant, &w.event, &id).await.unwrap();
    assert_eq!(w.run_row(&id).await.0, None, "gave up, the run not taken");

    let finishes = async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_eq!(w.run_row(&id).await.0, None, "still waiting");
        previous.rollback().await.unwrap();
    };
    let message = timeout(Duration::from_secs(120), execute(&w.tenant, &w.event, &id));
    // The event has no board or endpoints: the suite itself ends in failed checks.
    let (_, ended) = tokio::join!(finishes, message);
    assert!(ended.is_ok(), "the suite's deadline ends it");
    assert_eq!(
        w.run_row(&id).await.0.as_deref(),
        Some(process_instance()),
        "taken once the lock was free"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_failure_only_when_no_process_of_the_component_is_left() {
    let world = World::new().await;
    renewal(&world).await;
    stops(&world).await;
    starts(&world).await;
    expired_lease(&world).await;
    rolling_update(&world).await;
    interrupted_runs(&world).await;
    another_audit_running(&world).await;
    previous_run_finishing(&world).await;
}
