// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! `monitoring_voter`: what a pass over the voters' accounts writes, what it
//! leaves alone and removes, and how each voter's enrollment and first vote
//! follow their applications and votes.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use deadpool_postgres::Transaction;
use sequent_core::monitoring::config::Settings;
use sequent_core::monitoring::presets;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;
use windmill::postgres::monitoring_config::EventRef;
use windmill::services::monitoring::projection::{
    event_first_day, full_pass_due, load_event_places, project_accounts, refresh_voter_activity,
    LastPass, ProjectionContext, VoterAccount,
};

fn comelec_settings() -> Settings {
    presets::load("comelec")
        .unwrap()
        .unwrap()
        .set
        .settings
        .clone()
        .unwrap()
}

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, day, hour, 0, 0).unwrap()
}

struct Seed<'t, 'c> {
    tx: &'t Transaction<'c>,
    event: EventRef,
}

impl<'t, 'c> Seed<'t, 'c> {
    async fn new(tx: &'t Transaction<'c>) -> Seed<'t, 'c> {
        let event = EventRef {
            tenant_id: Uuid::new_v4(),
            election_event_id: Uuid::new_v4(),
        };
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&event.tenant_id, &format!("tenant-{}", event.tenant_id)],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event.election_event_id, &event.tenant_id],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
             VALUES ($1, $2)",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .unwrap();
        Seed { tx, event }
    }

    /// An election (a Post) of `region`, with unlimited revotes.
    async fn election(&self, region: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.tx
            .execute(
                "INSERT INTO sequent_backend.election
                     (id, tenant_id, election_event_id, annotations, num_allowed_revotes)
                 VALUES ($1, $2, $3, $4, 0)",
                &[
                    &id,
                    &self.event.tenant_id,
                    &self.event.election_event_id,
                    &json!({"miru:geographical-region": region}),
                ],
            )
            .await
            .unwrap();
        id
    }

    /// An area that votes in `elections`.
    async fn area(&self, elections: &[Uuid]) -> Uuid {
        let area = Uuid::new_v4();
        self.tx
            .execute(
                "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
                 VALUES ($1, $2, $3, 'area')",
                &[&area, &self.event.tenant_id, &self.event.election_event_id],
            )
            .await
            .unwrap();
        for election in elections {
            let contest = Uuid::new_v4();
            self.tx
                .execute(
                    "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
                     VALUES ($1, $2, $3, $4)",
                    &[&contest, &self.event.tenant_id, &self.event.election_event_id, election],
                )
                .await
                .unwrap();
            self.tx
                .execute(
                    "INSERT INTO sequent_backend.area_contest
                         (id, tenant_id, election_event_id, area_id, contest_id)
                     VALUES ($1, $2, $3, $4, $5)",
                    &[
                        &Uuid::new_v4(),
                        &self.event.tenant_id,
                        &self.event.election_event_id,
                        &area,
                        &contest,
                    ],
                )
                .await
                .unwrap();
        }
        area
    }

    async fn vote(
        &self,
        election: Uuid,
        area: Uuid,
        voter: &str,
        status: &str,
        when: DateTime<Utc>,
    ) {
        self.tx
            .execute(
                "INSERT INTO sequent_backend.cast_vote
                     (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                      status, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &self.event.tenant_id,
                    &self.event.election_event_id,
                    &election,
                    &area,
                    &voter,
                    &status,
                    &when,
                ],
            )
            .await
            .unwrap();
    }

    async fn application(
        &self,
        area: Uuid,
        voter: &str,
        status: &str,
        annotations: Value,
        when: DateTime<Utc>,
    ) {
        self.tx
            .execute(
                "INSERT INTO sequent_backend.applications
                     (tenant_id, election_event_id, area_id, applicant_id, status,
                      verification_type, applicant_data, annotations, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, 'MANUAL', '{}', $6, $7, $7)",
                &[
                    &self.event.tenant_id,
                    &self.event.election_event_id,
                    &area,
                    &voter,
                    &status,
                    &annotations,
                    &when,
                ],
            )
            .await
            .unwrap();
    }

    async fn rows(&self) -> Vec<Value> {
        self.tx
            .query(
                "SELECT to_jsonb(v) - 'tenant_id' - 'election_event_id' - 'updated_at'
                 FROM sequent_backend.monitoring_voter v
                 WHERE tenant_id = $1 AND election_event_id = $2
                 ORDER BY voter_id, election_id",
                &[&self.event.tenant_id, &self.event.election_event_id],
            )
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect()
    }

    async fn row(&self, election: Uuid, voter: &str) -> Value {
        self.rows()
            .await
            .into_iter()
            .find(|row| row["voter_id"] == voter && row["election_id"] == election.to_string())
            .unwrap_or_else(|| panic!("no row for {voter}"))
    }
}

fn account(id: &str, area: Option<Uuid>, attributes: &[(&str, &str)]) -> VoterAccount {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(area) = area {
        map.insert("area-id".into(), vec![area.to_string()]);
    }
    for (name, value) in attributes {
        map.entry(name.to_string())
            .or_default()
            .push(value.to_string());
    }
    VoterAccount {
        voter_id: id.to_string(),
        attributes: map,
        credentials_at: None,
    }
}

async fn pass(
    seed: &Seed<'_, '_>,
    settings: &Settings,
    settings_revision: i32,
    accounts: &[VoterAccount],
) -> (u64, u64) {
    let places = load_event_places(seed.tx, seed.event).await.unwrap();
    let context = ProjectionContext {
        event: seed.event,
        settings,
        settings_revision,
        places: &places,
        on: NaiveDate::from_ymd_opt(2026, 5, 11).unwrap(),
    };
    project_accounts(seed.tx, &context, accounts).await.unwrap()
}

#[tokio::test]
async fn a_voter_is_projected_into_each_election_their_area_votes_in() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let madrid = seed.election("Europe").await;
    let manila = seed.election("Asia").await;
    let both = seed.area(&[madrid, manila]).await;
    let voter = account(
        "voter-1",
        Some(both),
        &[
            ("country", "Spain/Madrid"),
            ("sex", "F"),
            ("dateOfBirth", "1990-01-01"),
            ("landBasedOrSeafarer", "Seafarer"),
            ("sequent.read-only.id-card-number-validated", "VERIFIED"),
        ],
    );
    let nowhere = account("voter-2", None, &[("sex", "M")]);
    let unknown_area = account("voter-3", Some(Uuid::new_v4()), &[]);
    let bare = account("voter-4", Some(both), &[]);
    assert_eq!(
        pass(
            &seed,
            &settings,
            1,
            &[voter.clone(), nowhere, unknown_area, bare]
        )
        .await,
        (4, 0)
    );

    let row = seed.row(madrid, "voter-1").await;
    assert_eq!(row["region"], "Europe", "the region is the election's");
    assert_eq!(
        row["country"], "Spain",
        "the country part of Country/Embassy"
    );
    assert_eq!(
        row["dims"],
        json!({"age_band": "25-39", "sex": "F", "status": "Seafarer"}),
        "derived values, never the date of birth"
    );
    assert!(row["pre_enrolled_at"].is_string());
    assert_eq!(row["area_id"], both.to_string());
    assert_eq!(row["settings_revision"], 1);
    assert_eq!(seed.row(manila, "voter-1").await["region"], "Asia");

    let bare = seed.row(madrid, "voter-4").await;
    assert_eq!(bare["dims"], json!({}), "every dimension Unknown");
    assert_eq!(bare["country"], Value::Null);
    assert_eq!(bare["pre_enrolled_at"], Value::Null);
    assert_eq!(
        seed.rows().await.len(),
        4,
        "a voter with no area or an area of no election has no row"
    );
}

#[tokio::test]
async fn a_pass_rewrites_only_what_changed_and_removes_who_is_gone() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let election = seed.election("Europe").await;
    let area = seed.area(&[election]).await;
    let verified = ("sequent.read-only.id-card-number-validated", "VERIFIED");
    let one = account("voter-1", Some(area), &[("sex", "F"), verified]);
    let two = account("voter-2", Some(area), &[("sex", "M")]);
    assert_eq!(
        pass(&seed, &settings, 1, &[one.clone(), two.clone()]).await,
        (2, 0)
    );
    let first = seed.row(election, "voter-1").await;

    assert_eq!(
        pass(&seed, &settings, 1, &[one.clone(), two.clone()]).await,
        (0, 0),
        "an unchanged voter is not rewritten"
    );

    let changed = account("voter-1", Some(area), &[("sex", "M"), verified]);
    assert_eq!(pass(&seed, &settings, 1, &[changed.clone()]).await, (1, 1));
    let row = seed.row(election, "voter-1").await;
    assert_eq!(row["dims"], json!({"sex": "M"}));
    assert_eq!(
        row["pre_enrolled_at"], first["pre_enrolled_at"],
        "pre-enrolled since it was first seen so"
    );
    assert_eq!(seed.rows().await.len(), 1, "a deleted account is removed");

    assert_eq!(
        pass(&seed, &settings, 2, &[changed.clone()]).await,
        (1, 0),
        "new settings rewrite every voter"
    );
    assert_eq!(seed.row(election, "voter-1").await["settings_revision"], 2);

    let unverified = account("voter-1", Some(area), &[("sex", "M")]);
    pass(&seed, &settings, 2, &[unverified]).await;
    assert_eq!(
        seed.row(election, "voter-1").await["pre_enrolled_at"],
        Value::Null
    );
}

#[tokio::test]
async fn enrollment_and_the_first_vote_follow_applications_and_votes() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let election = seed.election("Europe").await;
    let other = seed.election("Asia").await;
    let area = seed.area(&[election, other]).await;
    let verified = ("sequent.read-only.id-card-number-validated", "VERIFIED");
    let accounts = [
        account("accepted", Some(area), &[verified]),
        account("rejected", Some(area), &[]),
        account("pending", Some(area), &[]),
        account("none", Some(area), &[]),
    ];
    pass(&seed, &settings, 1, &accounts).await;

    seed.application(
        area,
        "accepted",
        "REJECTED",
        json!({"rejection_reason": "blurry"}),
        at(1, 9),
    )
    .await;
    seed.application(area, "accepted", "ACCEPTED", json!({}), at(2, 9))
        .await;
    seed.application(
        area,
        "rejected",
        "REJECTED",
        json!({"rejection_reason": " no match "}),
        at(3, 9),
    )
    .await;
    seed.application(area, "pending", "PENDING", json!({}), at(3, 9))
        .await;

    seed.vote(election, area, "accepted", "valid", at(5, 12))
        .await;
    seed.vote(election, area, "accepted", "valid", at(5, 10))
        .await;
    seed.vote(election, area, "accepted", "discarded", at(5, 8))
        .await;
    seed.vote(election, area, "rejected", "in-progress", at(5, 8))
        .await;

    assert!(refresh_voter_activity(&tx, seed.event).await.unwrap() > 0);
    let accepted = seed.row(election, "accepted").await;
    assert_eq!(
        accepted["enrollment_state"], "ACCEPTED",
        "the latest application"
    );
    assert_eq!(accepted["enrollment_reason"], Value::Null);
    let decided: DateTime<Utc> =
        serde_json::from_value(accepted["enrollment_decided_at"].clone()).unwrap();
    assert_eq!(decided, at(2, 9));
    let pre_enrolled: DateTime<Utc> =
        serde_json::from_value(accepted["pre_enrolled_at"].clone()).unwrap();
    assert_eq!(pre_enrolled, at(2, 9), "pre-enrolled from the approval on");
    let voted: DateTime<Utc> = serde_json::from_value(accepted["first_voted_at"].clone()).unwrap();
    assert_eq!(voted, at(5, 10), "the earliest valid vote, over revotes");
    assert_eq!(
        seed.row(other, "accepted").await["first_voted_at"],
        Value::Null,
        "a vote counts in its own election"
    );

    let rejected = seed.row(election, "rejected").await;
    assert_eq!(rejected["enrollment_state"], "REJECTED");
    assert_eq!(rejected["enrollment_reason"], "no match");
    assert_eq!(
        rejected["first_voted_at"],
        Value::Null,
        "only valid votes count"
    );
    let pending = seed.row(election, "pending").await;
    assert_eq!(pending["enrollment_state"], "PENDING");
    assert_eq!(pending["enrollment_decided_at"], Value::Null);
    assert_eq!(
        seed.row(election, "none").await["enrollment_state"],
        Value::Null
    );

    assert_eq!(
        refresh_voter_activity(&tx, seed.event).await.unwrap(),
        0,
        "nothing changed, nothing is written"
    );

    // A pass that rewrites the voter keeps what the activity refresh set.
    pass(&seed, &settings, 2, &accounts).await;
    let kept = seed.row(election, "accepted").await;
    assert_eq!(kept["enrollment_state"], "ACCEPTED");
    assert_eq!(kept["first_voted_at"], accepted["first_voted_at"]);
    assert_eq!(kept["pre_enrolled_at"], accepted["pre_enrolled_at"]);
}

#[tokio::test]
async fn ages_are_counted_as_of_the_event_first_day_in_its_time_zone() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let mut settings = comelec_settings();
    settings.time_zone = "Asia/Manila".into();
    let now = at(20, 0);
    assert_eq!(
        event_first_day(&tx, seed.event, &settings, now)
            .await
            .unwrap(),
        NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
        "no start scheduled: today, in Manila"
    );
    for (processor, date, archived) in [
        ("START_VOTING_PERIOD", "2026-05-11T16:30:00Z", false),
        ("START_VOTING_PERIOD", "2026-05-12T16:30:00Z", false),
        ("START_VOTING_PERIOD", "2026-05-01T00:00:00Z", true),
        ("START_ENROLLMENT_PERIOD", "2026-04-01T00:00:00Z", false),
    ] {
        tx.execute(
            "INSERT INTO sequent_backend.scheduled_event
                 (tenant_id, election_event_id, event_processor, cron_config, archived_at)
             VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() END)",
            &[
                &seed.event.tenant_id,
                &seed.event.election_event_id,
                &processor,
                &json!({"scheduled_date": date}),
                &archived,
            ],
        )
        .await
        .unwrap();
    }
    assert_eq!(
        event_first_day(&tx, seed.event, &settings, now)
            .await
            .unwrap(),
        NaiveDate::from_ymd_opt(2026, 5, 12).unwrap(),
        "the first voting start, 16:30 UTC being past midnight in Manila"
    );
    settings.time_zone = "UTC".into();
    assert_eq!(
        event_first_day(&tx, seed.event, &settings, now)
            .await
            .unwrap(),
        NaiveDate::from_ymd_opt(2026, 5, 11).unwrap()
    );
}

#[test]
fn a_full_pass_is_due_first_on_new_settings_and_then_every_interval() {
    let every = Duration::minutes(5);
    let now = at(11, 12);
    assert!(full_pass_due(None, 1, now, every));
    let last = LastPass {
        at: now - Duration::minutes(4),
        settings_revision: 1,
    };
    assert!(!full_pass_due(Some(last), 1, now, every));
    assert!(full_pass_due(Some(last), 2, now, every));
    assert!(full_pass_due(
        Some(last),
        1,
        now + Duration::minutes(1),
        every
    ));
}
