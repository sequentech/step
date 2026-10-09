// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box seal check when a tally session is created and when it
//! starts (VOTE-FREEZE), through the in-memory adapters.

use super::*;
use crate::adapters::memory::clock::SequentialIds;
use crate::adapters::memory::tally_ceremony::{InMemoryTallyCeremony, TallyCall};
use crate::postgres::ballot_box_seal::BallotBoxSealStatus::{self, *};
use sequent_core::ballot::Contest as SequentContest;

const TENANT: &str = "tenant";
const EVENT: &str = "event";
const ELECTION: &str = "00000000-0000-4000-8000-000000000010";
const KEYS_CEREMONY: &str = "keys-ceremony";
const SESSION: &str = "session";
const ADMIN_ID: &str = "admin-id";
const ADMIN: &str = "admin";
const ELECTORAL_RESULTS: &str = "ELECTORAL_RESULTS";
const INITIALIZATION_REPORT: &str = "INITIALIZATION_REPORT";
const AREAS: [&str; 2] = ["north", "south"];

fn seal_at_close() -> Value {
    json!({"ballot_box_seal_policy": "seal-at-close"})
}

fn published(area_id: &str) -> BallotStyle {
    published_with(area_id, false)
}

/// The published style of `area_id`, whose mayor contest is acclaimed or not.
fn published_with(area_id: &str, is_acclaimed: bool) -> BallotStyle {
    let ballot_style = SequentBallotStyle {
        id: format!("style-{area_id}"),
        tenant_id: TENANT.into(),
        election_event_id: EVENT.into(),
        election_id: ELECTION.into(),
        num_allowed_revotes: None,
        description: None,
        public_key: None,
        area_id: area_id.into(),
        area_presentation: None,
        contests: vec![SequentContest {
            id: "mayor".into(),
            election_id: ELECTION.into(),
            counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
            is_acclaimed: Some(is_acclaimed),
            ..Default::default()
        }],
        election_event_presentation: None,
        election_presentation: None,
        election_dates: None,
        election_event_annotations: None,
        election_annotations: None,
        area_annotations: None,
        multi_contest_encoding_mode: None,
        ballot_box_key: None,
    };
    serde_json::from_value(json!({
        "id": ballot_style.id, "tenant_id": TENANT, "election_event_id": EVENT,
        "election_id": ELECTION, "area_id": area_id,
        "ballot_eml": serde_json::to_string(&ballot_style).unwrap(),
        "ballot_publication_id": "publication",
    }))
    .unwrap()
}

/// An event with `presentation` whose election has closed, with a mayor
/// contest published in two areas, a finished keys ceremony and a tally
/// session (`SESSION`) ready to start.
fn closed_event(presentation: Value) -> InMemoryTallyCeremony {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(
        serde_json::from_value(json!({
            "id": EVENT, "tenant_id": TENANT, "is_archived": false,
            "encryption_protocol": "RistrettoCtx", "presentation": presentation,
            "bulletin_board_reference":
                {"id": 1, "database_name": "event-board", "is_archived": false},
        }))
        .unwrap(),
    );
    ceremony.add_election(
        serde_json::from_value(json!({
            "id": ELECTION, "tenant_id": TENANT, "election_event_id": EVENT,
            "status": {"is_published": true, "voting_status": "CLOSED",
                       "allow_tally": "allowed", "init_report": "allowed"},
            "keys_ceremony_id": KEYS_CEREMONY,
        }))
        .unwrap(),
    );
    ceremony.add_contest(
        serde_json::from_value(json!({
            "id": "mayor", "tenant_id": TENANT, "election_event_id": EVENT,
            "election_id": ELECTION,
        }))
        .unwrap(),
    );
    for area_id in AREAS {
        ceremony.add_area(
            serde_json::from_value(
                json!({"id": area_id, "tenant_id": TENANT, "election_event_id": EVENT}),
            )
            .unwrap(),
        );
        ceremony.add_area_contest(
            TENANT,
            EVENT,
            AreaContest {
                id: format!("{area_id}-mayor"),
                area_id: area_id.into(),
                contest_id: "mayor".into(),
            },
        );
        ceremony.add_ballot_style(published(area_id));
    }
    ceremony.add_keys_ceremony(
        serde_json::from_value(json!({
            "id": KEYS_CEREMONY, "tenant_id": TENANT, "election_event_id": EVENT,
            "trustee_ids": [], "threshold": 2, "execution_status": "SUCCESS",
            "settings": {"policy": "manual-ceremonies"},
            "status": {"stop_date": null, "public_key": "public-key", "logs": [], "trustees": [
                {"name": "alice", "status": "KEY_CHECKED"},
                {"name": "bob", "status": "KEY_CHECKED"},
            ]},
        }))
        .unwrap(),
    );
    ceremony.add_session(tally_session(None));
    ceremony.add_execution(
        serde_json::from_value(json!({
            "id": "execution", "tenant_id": TENANT, "election_event_id": EVENT,
            "current_message_id": 7, "tally_session_id": SESSION,
            "status": {"stop_date": null, "logs": [], "elections_status": [], "trustees": [
                {"name": "alice", "status": "KEY_RESTORED"},
                {"name": "bob", "status": "KEY_RESTORED"},
            ]},
        }))
        .unwrap(),
    );
    ceremony.set_env_slug("dev");
    ceremony
}

fn tally_session(tally_type: Option<&str>) -> TallySession {
    serde_json::from_value(json!({
        "id": SESSION, "tenant_id": TENANT, "election_event_id": EVENT,
        "election_ids": [ELECTION], "is_execution_completed": false,
        "keys_ceremony_id": KEYS_CEREMONY, "execution_status": "CONNECTED",
        "threshold": 2, "tally_type": tally_type,
    }))
    .unwrap()
}

fn add_seal(ceremony: &InMemoryTallyCeremony, area_id: &str, status: BallotBoxSealStatus) {
    ceremony.add_ballot_box_seal(
        TENANT,
        EVENT,
        BallotBoxSealState {
            election_id: ELECTION.into(),
            area_id: area_id.into(),
            status,
            ballots_in_box: Some(1),
        },
    );
}

async fn create(ceremony: &InMemoryTallyCeremony, tally_type: &str) -> Result<String> {
    create_tally_ceremony_with(
        ceremony,
        ceremony,
        ceremony,
        ceremony,
        ceremony,
        &SequentialIds::default(),
        TallyCreation {
            tenant_id: TENANT.into(),
            user_id: ADMIN_ID,
            election_event_id: EVENT.into(),
            election_ids: vec![ELECTION.into()],
            configuration: None,
            tally_type: tally_type.into(),
            permission_labels: &vec![],
            username: ADMIN.into(),
            area_ids: None,
        },
    )
    .await
}

async fn start(ceremony: &InMemoryTallyCeremony, tally_type: Option<&str>) -> Result<()> {
    update_tally_ceremony_with(
        ceremony,
        ceremony,
        ceremony,
        ceremony,
        TallyStatusChange {
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            tally_session: tally_session(tally_type),
            new_execution_status: TallyExecutionStatus::IN_PROGRESS,
            user_id: ADMIN_ID.into(),
            username: ADMIN.into(),
        },
    )
    .await
}

fn refusal(error: anyhow::Error) -> String {
    error
        .downcast_ref::<TallyValidationError>()
        .unwrap_or_else(|| panic!("expected a TallyValidationError, got {error:?}"))
        .to_string()
}

fn stored_status(ceremony: &InMemoryTallyCeremony) -> Option<String> {
    ceremony.session(SESSION).execution_status
}

#[tokio::test]
async fn results_are_created_and_started_once_every_box_is_published() {
    let ceremony = closed_event(seal_at_close());
    for area_id in AREAS {
        add_seal(&ceremony, area_id, Published);
    }
    create(&ceremony, ELECTORAL_RESULTS).await.unwrap();
    start(&ceremony, None).await.unwrap();
    assert_eq!(stored_status(&ceremony), Some("IN_PROGRESS".into()));
}

#[tokio::test]
async fn results_are_refused_while_a_box_is_not_published() {
    for (south, reason) in [
        (None, "south (not sealed yet)"),
        (Some(Pending), "south (not sealed yet)"),
        (
            Some(Sealed),
            "south (sealed, not yet on the bulletin board)",
        ),
        (Some(Failed), "south (not sealed: incident)"),
    ] {
        let ceremony = closed_event(seal_at_close());
        add_seal(&ceremony, "north", Published);
        if let Some(status) = south {
            add_seal(&ceremony, "south", status);
        }
        let created = refusal(create(&ceremony, ELECTORAL_RESULTS).await.unwrap_err());
        assert!(created.contains(reason), "{created}");
        assert!(!created.contains("north"), "{created}");
        assert!(ceremony
            .sessions()
            .iter()
            .all(|session| session.id == SESSION));

        let started = refusal(start(&ceremony, None).await.unwrap_err());
        assert_eq!(started, created);
        assert_eq!(stored_status(&ceremony), Some("CONNECTED".into()));
    }
}

#[tokio::test]
async fn a_seal_row_outside_the_published_styles_must_be_published_too() {
    let ceremony = closed_event(seal_at_close());
    for area_id in AREAS {
        add_seal(&ceremony, area_id, Published);
    }
    // A box with ballots but no votable style: the sealer seals it too.
    add_seal(&ceremony, "east", Failed);
    let error = refusal(create(&ceremony, ELECTORAL_RESULTS).await.unwrap_err());
    assert!(error.ends_with(": east (not sealed: incident)."), "{error}");
    assert!(start(&ceremony, Some(ELECTORAL_RESULTS)).await.is_err());
}

#[tokio::test]
async fn initialization_reports_do_not_read_the_seals() {
    let ceremony = closed_event(seal_at_close());
    ceremony.fail(TallyCall::BallotBoxSeals, "seals are unavailable");
    create(&ceremony, INITIALIZATION_REPORT).await.unwrap();
    start(&ceremony, Some(INITIALIZATION_REPORT)).await.unwrap();
    assert_eq!(stored_status(&ceremony), Some("IN_PROGRESS".into()));
}

#[tokio::test]
async fn an_event_that_does_not_seal_reads_only_the_event_to_start() {
    let ceremony = closed_event(json!({}));
    create(&ceremony, ELECTORAL_RESULTS).await.unwrap();
    for call in [
        TallyCall::BallotBoxSeals,
        TallyCall::EventSnapshot,
        TallyCall::PublishedBallotStyles,
    ] {
        ceremony.fail(call, "not needed without the seal policy");
    }
    start(&ceremony, None).await.unwrap();
    assert_eq!(stored_status(&ceremony), Some("IN_PROGRESS".into()));
}

#[tokio::test]
async fn a_session_without_executions_does_not_read_the_event() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_session(tally_session(None));
    ceremony.add_election(
        serde_json::from_value(json!({
            "id": ELECTION, "tenant_id": TENANT, "election_event_id": EVENT,
            "status": {"is_published": true, "voting_status": "CLOSED", "allow_tally": "allowed"},
        }))
        .unwrap(),
    );
    ceremony.fail(TallyCall::GetElectionEvent, "no event");
    start(&ceremony, None).await.unwrap();
    assert_eq!(stored_status(&ceremony), Some("CONNECTED".into()));
}

#[tokio::test]
async fn a_sealed_box_with_ballots_outside_the_published_styles_refuses_the_tally() {
    for (ballots_in_box, refused) in [(Some(3), true), (Some(0), false)] {
        let ceremony = closed_event(seal_at_close());
        for area_id in AREAS {
            add_seal(&ceremony, area_id, Published);
        }
        // Sealed and published, but its area lost the election's contests.
        ceremony.add_ballot_box_seal(
            TENANT,
            EVENT,
            BallotBoxSealState {
                election_id: ELECTION.into(),
                area_id: "east".into(),
                status: Published,
                ballots_in_box,
            },
        );
        let created = create(&ceremony, ELECTORAL_RESULTS).await;
        let started = start(&ceremony, None).await;
        if refused {
            let message = refusal(created.unwrap_err());
            assert!(
                message.starts_with("The sealed ballot box of ")
                    && message.ends_with(
                        ", east is not in this tally: its area no longer has the election's contests."
                    ),
                "{message}"
            );
            assert_eq!(refusal(started.unwrap_err()), message);
        } else {
            created.unwrap();
            started.unwrap();
        }
    }
}

#[tokio::test]
async fn a_sealed_box_whose_contests_are_all_acclaimed_refuses_the_tally_at_creation() {
    let ceremony = closed_event(seal_at_close());
    for area_id in AREAS {
        add_seal(&ceremony, area_id, Published);
    }
    // East has a published style, but its only contest is acclaimed: the
    // session gets no contest there, so its sealed ballots wouldn't count.
    ceremony.add_ballot_style(published_with("east", true));
    ceremony.add_ballot_box_seal(
        TENANT,
        EVENT,
        BallotBoxSealState {
            election_id: ELECTION.into(),
            area_id: "east".into(),
            status: Published,
            ballots_in_box: Some(2),
        },
    );
    let message = refusal(create(&ceremony, ELECTORAL_RESULTS).await.unwrap_err());
    assert!(
        message.ends_with(
            ", east is not in this tally: its area no longer has the election's contests."
        ),
        "{message}"
    );
    assert!(ceremony
        .sessions()
        .iter()
        .all(|session| session.id == SESSION));
}
