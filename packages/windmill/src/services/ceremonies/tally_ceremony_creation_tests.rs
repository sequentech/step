// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::clock::SequentialIds;
use crate::adapters::memory::tally_ceremony::{InMemoryTallyCeremony, TallyAuditEntry, TallyCall};
use sequent_core::ballot::{
    Contest as SequentContest, DecodedBallotsInclusionPolicy, DelegatedVotingPolicy,
};
use sequent_core::types::hasura::core::{Area, TallySessionContest, TallySheet};
use sequent_core::types::tally_sheets::TallySheetStatus;

const TENANT: &str = "tenant";
const EVENT: &str = "event";
const ELECTION: &str = "election";
const OTHER_ELECTION: &str = "other-election";
const KEYS_CEREMONY: &str = "keys-ceremony";
const EVENT_BOARD: &str = "event-board";
const ADMIN_ID: &str = "admin-id";
const ADMIN: &str = "admin";
const ELECTORAL_RESULTS: &str = "ELECTORAL_RESULTS";
/// The first id `SequentialIds` hands out.
const FIRST_ID: &str = "00000000-0000-0000-0000-000000000001";

fn election_event(presentation: Value, bulletin_board_reference: Option<Value>) -> ElectionEvent {
    serde_json::from_value(json!({
        "id": EVENT, "tenant_id": TENANT, "is_archived": false,
        "encryption_protocol": "RistrettoCtx", "presentation": presentation,
        "bulletin_board_reference": bulletin_board_reference,
    }))
    .unwrap()
}

fn event_board() -> Option<Value> {
    Some(json!({"id": 1, "database_name": EVENT_BOARD, "is_archived": false}))
}

fn election(id: &str, keys_ceremony_id: Option<&str>, permission_label: Option<&str>) -> Election {
    serde_json::from_value(json!({
        "id": id, "tenant_id": TENANT, "election_event_id": EVENT,
        "status": {"is_published": true, "voting_status": "CLOSED", "allow_tally": "allowed"},
        "keys_ceremony_id": keys_ceremony_id, "permission_label": permission_label,
    }))
    .unwrap()
}

fn contest(id: &str, election_id: &str) -> Contest {
    serde_json::from_value(json!({
        "id": id, "tenant_id": TENANT, "election_event_id": EVENT, "election_id": election_id,
    }))
    .unwrap()
}

fn area(id: &str) -> Area {
    serde_json::from_value(json!({"id": id, "tenant_id": TENANT, "election_event_id": EVENT}))
        .unwrap()
}

fn area_contest(area_id: &str, contest_id: &str) -> AreaContest {
    AreaContest {
        id: format!("{area_id}-{contest_id}"),
        area_id: area_id.into(),
        contest_id: contest_id.into(),
    }
}

fn ballot_contest(
    id: &str,
    counting_algorithm: CountingAlgType,
    is_acclaimed: bool,
) -> SequentContest {
    SequentContest {
        id: id.into(),
        election_id: ELECTION.into(),
        counting_algorithm: Some(counting_algorithm),
        is_acclaimed: Some(is_acclaimed),
        ..Default::default()
    }
}

fn plurality(id: &str) -> SequentContest {
    ballot_contest(id, CountingAlgType::PluralityAtLarge, false)
}

fn acclaimed(id: &str) -> SequentContest {
    ballot_contest(id, CountingAlgType::PluralityAtLarge, true)
}

/// The published ballot style of `election_id` in `area_id`.
fn published(
    election_id: &str,
    area_id: &str,
    contests: Vec<SequentContest>,
    area_annotations: Option<Value>,
) -> BallotStyle {
    let ballot_style = SequentBallotStyle {
        id: format!("style-{election_id}-{area_id}"),
        tenant_id: TENANT.into(),
        election_event_id: EVENT.into(),
        election_id: election_id.into(),
        num_allowed_revotes: None,
        description: None,
        public_key: None,
        area_id: area_id.into(),
        area_presentation: None,
        contests,
        election_event_presentation: None,
        election_presentation: None,
        election_dates: None,
        election_event_annotations: None,
        election_annotations: None,
        area_annotations: area_annotations.map(|value| serde_json::from_value(value).unwrap()),
        multi_contest_encoding_mode: None,
    };
    BallotStyle {
        id: ballot_style.id.clone(),
        tenant_id: TENANT.into(),
        election_id: election_id.into(),
        area_id: Some(area_id.into()),
        created_at: None,
        last_updated_at: None,
        labels: None,
        annotations: None,
        ballot_eml: Some(serde_json::to_string(&ballot_style).unwrap()),
        ballot_signature: None,
        status: None,
        election_event_id: EVENT.into(),
        deleted_at: None,
        ballot_publication_id: "publication".into(),
    }
}

fn keys_ceremony(execution_status: &str, policy: &str) -> KeysCeremony {
    serde_json::from_value(json!({
        "id": KEYS_CEREMONY, "tenant_id": TENANT, "election_event_id": EVENT,
        "trustee_ids": [], "threshold": 2, "execution_status": execution_status,
        "settings": {"policy": policy},
        "status": {"stop_date": null, "public_key": "public-key", "logs": [], "trustees": [
            {"name": "alice", "status": "KEY_CHECKED"},
            {"name": "bob", "status": "KEY_CHECKED"},
            {"name": "carol", "status": "KEY_CHECKED"},
        ]},
    }))
    .unwrap()
}

fn approved_tally_sheet(election_id: &str) -> TallySheet {
    serde_json::from_value(json!({
        "id": format!("sheet-{election_id}"), "tenant_id": TENANT, "election_event_id": EVENT,
        "election_id": election_id, "contest_id": "mayor", "area_id": "north",
        "created_by_user_id": ADMIN_ID, "status": "APPROVED", "version": 1,
        "reviewed_at": "2026-01-01T00:00:00Z", "reviewed_by_user_id": "reviewer",
    }))
    .unwrap()
}

/// An event with `presentation` whose `ELECTION` has closed. Its contest is
/// published in two areas and its keys ceremony has finished.
fn closed_event(presentation: Value) -> InMemoryTallyCeremony {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(election_event(presentation, event_board()));
    ceremony.add_election(election(ELECTION, Some(KEYS_CEREMONY), None));
    ceremony.add_contest(contest("mayor", ELECTION));
    for area_id in ["north", "south"] {
        ceremony.add_area(area(area_id));
        ceremony.add_area_contest(area_contest(area_id, "mayor"));
        ceremony.add_ballot_style(published(ELECTION, area_id, vec![plurality("mayor")], None));
    }
    ceremony.add_keys_ceremony(keys_ceremony("SUCCESS", "manual-ceremonies"));
    ceremony
}

fn voter_weighted() -> Value {
    json!({"weighted_voting_policy": "voters-weighted-voting"})
}

async fn create_tally(
    ceremony: &InMemoryTallyCeremony,
    tally_type: &str,
    election_ids: &[&str],
    permission_labels: &[&str],
) -> Result<String> {
    create_tally_with_events(
        ceremony,
        ceremony,
        tally_type,
        election_ids,
        permission_labels,
    )
    .await
}

async fn create_tally_with_events(
    ceremony: &InMemoryTallyCeremony,
    election_events: &impl ElectionEventReader,
    tally_type: &str,
    election_ids: &[&str],
    permission_labels: &[&str],
) -> Result<String> {
    let permission_labels: Vec<String> = permission_labels.iter().map(|l| l.to_string()).collect();
    create_tally_ceremony_with(
        ceremony,
        ceremony,
        ceremony,
        election_events,
        ceremony,
        &SequentialIds::default(),
        TallyCreation {
            tenant_id: TENANT.into(),
            user_id: ADMIN_ID,
            election_event_id: EVENT.into(),
            election_ids: election_ids.iter().map(|id| id.to_string()).collect(),
            configuration: None,
            tally_type: tally_type.into(),
            permission_labels: &permission_labels,
            username: ADMIN.into(),
        },
    )
    .await
}

async fn create(ceremony: &InMemoryTallyCeremony) -> Result<String> {
    create_tally(ceremony, ELECTORAL_RESULTS, &[ELECTION], &[]).await
}

fn validation_message(error: anyhow::Error) -> String {
    error
        .downcast_ref::<TallyValidationError>()
        .unwrap_or_else(|| panic!("expected a TallyValidationError, got {error:?}"))
        .to_string()
}

fn stored_session(ceremony: &InMemoryTallyCeremony) -> TallySession {
    let sessions = ceremony.sessions();
    assert_eq!(sessions.len(), 1);
    sessions[0].clone()
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn batches(session_contests: &[TallySessionContest]) -> HashSet<i32> {
    session_contests
        .iter()
        .map(|session_contest| session_contest.session_id)
        .collect()
}

fn assert_nothing_written(ceremony: &InMemoryTallyCeremony) {
    assert_eq!(ceremony.sessions(), vec![]);
    assert_eq!(ceremony.executions(FIRST_ID), vec![]);
    assert_eq!(ceremony.session_contests(FIRST_ID), vec![]);
    assert_eq!(ceremony.audit_entries(), vec![]);
}

#[tokio::test]
async fn a_session_is_stored_with_its_keys_ceremony_areas_and_executer() {
    let ceremony = closed_event(json!({}));
    let tally_session_id = create(&ceremony).await.unwrap();
    assert_eq!(tally_session_id, FIRST_ID);

    let tally_session = stored_session(&ceremony);
    assert_eq!(tally_session.id, FIRST_ID);
    assert_eq!(tally_session.election_ids, Some(vec![ELECTION.to_string()]));
    assert_eq!(
        tally_session.area_ids.map(sorted),
        Some(vec!["north".to_string(), "south".to_string()])
    );
    assert_eq!(tally_session.keys_ceremony_id, KEYS_CEREMONY);
    assert_eq!(tally_session.threshold, 2);
    assert_eq!(tally_session.tally_type.as_deref(), Some(ELECTORAL_RESULTS));
    assert_eq!(
        tally_session.annotations,
        Some(json!({"executer_username": ADMIN, "executer_user_id": ADMIN_ID}))
    );
    assert_eq!(tally_session.permission_label, Some(vec![]));
}

#[tokio::test]
async fn the_session_configuration_takes_its_policies_from_the_event() {
    let ceremony = closed_event(json!({
        "contest_encryption_policy": "multiple-contests",
        "decoded_ballot_inclusion_policy": "included",
        "delegated_voting_policy": "enabled",
        "weighted_voting_policy": "areas-weighted-voting",
    }));
    let permission_labels = vec![];
    create_tally_ceremony_with(
        &ceremony,
        &ceremony,
        &ceremony,
        &ceremony,
        &ceremony,
        &SequentialIds::default(),
        TallyCreation {
            tenant_id: TENANT.into(),
            user_id: ADMIN_ID,
            election_event_id: EVENT.into(),
            election_ids: vec![ELECTION.into()],
            configuration: Some(TallySessionConfiguration {
                report_content_template_id: Some("template".into()),
                contest_encryption_policy: Some(ContestEncryptionPolicy::SINGLE_CONTEST),
                ..Default::default()
            }),
            tally_type: ELECTORAL_RESULTS.into(),
            permission_labels: &permission_labels,
            username: ADMIN.into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        stored_session(&ceremony).configuration,
        Some(TallySessionConfiguration {
            report_content_template_id: Some("template".into()),
            contest_encryption_policy: Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS),
            decoded_ballots_inclusion_policy: Some(DecodedBallotsInclusionPolicy::INCLUDED),
            delegated_voting_policy: Some(DelegatedVotingPolicy::ENABLED),
            consolidated_report_policy: None,
            weighted_voting_policy: Some(WeightedVotingPolicy::AREAS_WEIGHTED_VOTING),
        })
    );
}

#[tokio::test]
async fn a_new_session_starts_with_every_trustee_and_election_waiting() {
    let ceremony = closed_event(json!({}));
    create(&ceremony).await.unwrap();

    let executions = ceremony.executions(FIRST_ID);
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].current_message_id, -1);
    assert_eq!(executions[0].run_reason.as_deref(), Some("NORMAL"));
    let status = get_tally_ceremony_status(executions[0].status.clone()).unwrap();
    assert_eq!(
        status
            .trustees
            .iter()
            .map(|trustee| (trustee.name.as_str(), trustee.status.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("alice", TallyTrusteeStatus::WAITING),
            ("bob", TallyTrusteeStatus::WAITING),
            ("carol", TallyTrusteeStatus::WAITING),
        ]
    );
    assert_eq!(status.elections_status.len(), 1);
    assert_eq!(status.elections_status[0].election_id, ELECTION);
    assert_eq!(
        status.elections_status[0].status,
        TallyElectionStatus::WAITING
    );
    assert_eq!(status.elections_status[0].progress, 0.0);
    assert_eq!(
        status
            .logs
            .iter()
            .map(|log| log.log_text.as_str())
            .collect::<Vec<_>>(),
        vec!["Created Tally Ceremony for election ids: [\"election\"]"]
    );
}

#[tokio::test]
async fn the_session_starts_as_started_unless_ceremonies_are_automated() {
    let manual = closed_event(json!({}));
    create(&manual).await.unwrap();
    assert_eq!(
        stored_session(&manual).execution_status.as_deref(),
        Some("STARTED")
    );

    let automated = InMemoryTallyCeremony::default();
    automated.add_election_event(election_event(json!({}), event_board()));
    automated.add_election(election(ELECTION, Some(KEYS_CEREMONY), None));
    automated.add_keys_ceremony(keys_ceremony("SUCCESS", "automated-ceremonies"));
    create(&automated).await.unwrap();
    assert_eq!(
        stored_session(&automated).execution_status.as_deref(),
        Some("IN_PROGRESS")
    );
}

#[tokio::test]
async fn an_invalid_tally_type_is_refused() {
    let ceremony = closed_event(json!({}));
    let error = create_tally(&ceremony, "RESULTS", &[ELECTION], &[])
        .await
        .unwrap_err();
    assert_eq!(validation_message(error), "Invalid tally type");
    assert_nothing_written(&ceremony);
}

#[tokio::test]
async fn the_selected_elections_are_validated_before_anything_is_written() {
    let ceremony = closed_event(json!({}));
    let error = create_tally(&ceremony, ELECTORAL_RESULTS, &[ELECTION, "missing"], &[])
        .await
        .unwrap_err();
    assert_eq!(
        validation_message(error),
        "Selected election missing was not found."
    );
    assert_nothing_written(&ceremony);
}

#[tokio::test]
async fn every_voter_weighted_voting_refusal_stops_the_creation() {
    let delegated = closed_event(json!({
        "weighted_voting_policy": "voters-weighted-voting", "delegated_voting_policy": "enabled",
    }));
    let decoded = closed_event(json!({
        "weighted_voting_policy": "voters-weighted-voting",
        "decoded_ballot_inclusion_policy": "included",
    }));
    let tally_sheet = closed_event(voter_weighted());
    tally_sheet.add_tally_sheet(approved_tally_sheet(ELECTION));
    let area_weight = closed_event(voter_weighted());
    area_weight.add_ballot_style(published(
        ELECTION,
        "north",
        vec![plurality("mayor")],
        Some(json!({"weight": 4})),
    ));
    let algorithm = closed_event(voter_weighted());
    algorithm.add_ballot_style(published(
        ELECTION,
        "south",
        vec![ballot_contest("council", CountingAlgType::Borda, false)],
        None,
    ));
    for (ceremony, refusal) in [
        (
            &delegated,
            "Delegated voting and voter-weighted voting cannot both be enabled",
        ),
        (
            &decoded,
            "Decoded ballots cannot be included in the results",
        ),
        (
            &tally_sheet,
            "1 approved tally sheet(s) exist for this election event",
        ),
        (&area_weight, "because the two would multiply: north."),
        (&algorithm, "These contests use another algorithm: council"),
    ] {
        let error = create(ceremony).await.unwrap_err();
        let message = validation_message(error);
        assert!(message.contains(refusal), "{message}");
        assert_nothing_written(ceremony);
    }
}

#[tokio::test]
async fn other_weighting_policies_skip_the_voter_weighted_voting_checks() {
    let ceremony = closed_event(json!({
        "weighted_voting_policy": "areas-weighted-voting",
        "delegated_voting_policy": "enabled",
        "decoded_ballot_inclusion_policy": "included",
    }));
    ceremony.fail(TallyCall::ApprovedTallySheets, "tally sheets are not read");
    ceremony.add_ballot_style(published(
        ELECTION,
        "north",
        vec![ballot_contest("council", CountingAlgType::Borda, false)],
        Some(json!({"weight": 4})),
    ));
    create(&ceremony).await.unwrap();
}

#[tokio::test]
async fn the_event_policies_are_checked_before_the_tally_sheets_are_read() {
    let ceremony = closed_event(json!({
        "weighted_voting_policy": "voters-weighted-voting", "delegated_voting_policy": "enabled",
    }));
    ceremony.fail(TallyCall::ApprovedTallySheets, "tally sheets unavailable");
    let error = create(&ceremony).await.unwrap_err();
    assert!(validation_message(error).starts_with("Delegated voting"));
}

#[tokio::test]
async fn tally_sheets_of_other_elections_do_not_stop_a_voter_weighted_tally() {
    let ceremony = closed_event(voter_weighted());
    ceremony.add_tally_sheet(approved_tally_sheet(OTHER_ELECTION));
    create(&ceremony).await.unwrap();
    assert_eq!(ceremony.sessions().len(), 1);
}

#[tokio::test]
async fn the_default_area_weight_does_not_stop_a_voter_weighted_tally() {
    let ceremony = closed_event(voter_weighted());
    ceremony.add_ballot_style(published(
        ELECTION,
        "north",
        vec![plurality("mayor")],
        Some(json!({"weight": 1})),
    ));
    create(&ceremony).await.unwrap();
    assert_eq!(ceremony.sessions().len(), 1);
}

#[tokio::test]
async fn an_election_outside_the_users_permission_labels_is_refused() {
    let ceremony = closed_event(json!({}));
    ceremony.add_election(election(OTHER_ELECTION, Some(KEYS_CEREMONY), Some("north")));
    let error = create_tally(
        &ceremony,
        ELECTORAL_RESULTS,
        &[ELECTION, OTHER_ELECTION],
        &["south"],
    )
    .await
    .unwrap_err();
    assert_eq!(
        validation_message(error),
        "Some elections don't have the required permission label or are not published"
    );
    assert_nothing_written(&ceremony);
}

#[tokio::test]
async fn the_permission_labels_of_the_tallied_elections_are_stored() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(election_event(json!({}), event_board()));
    ceremony.add_election(election(ELECTION, Some(KEYS_CEREMONY), Some("north")));
    ceremony.add_election(election(OTHER_ELECTION, Some(KEYS_CEREMONY), Some("south")));
    ceremony.add_election(election("unlabelled", Some(KEYS_CEREMONY), None));
    ceremony.add_keys_ceremony(keys_ceremony("SUCCESS", "manual-ceremonies"));
    create_tally(
        &ceremony,
        ELECTORAL_RESULTS,
        &[ELECTION, OTHER_ELECTION, "unlabelled"],
        &["north", "south", "east"],
    )
    .await
    .unwrap();
    assert_eq!(
        stored_session(&ceremony).permission_label.map(sorted),
        Some(vec!["north".to_string(), "south".to_string()])
    );
}

#[tokio::test]
async fn without_permission_labels_no_label_is_checked_or_stored() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(election_event(json!({}), event_board()));
    ceremony.add_election(election(ELECTION, Some(KEYS_CEREMONY), Some("north")));
    ceremony.add_keys_ceremony(keys_ceremony("SUCCESS", "manual-ceremonies"));
    create(&ceremony).await.unwrap();
    assert_eq!(stored_session(&ceremony).permission_label, Some(vec![]));
}

#[tokio::test]
async fn the_elections_must_share_exactly_one_successful_keys_ceremony() {
    let cases = [
        (
            vec![election(ELECTION, None, None)],
            "The selected elections have no keys ceremony",
        ),
        (
            vec![
                election(ELECTION, Some(KEYS_CEREMONY), None),
                election(OTHER_ELECTION, Some("another-keys-ceremony"), None),
            ],
            "Elections have different keys ceremonies",
        ),
        (
            vec![
                election(ELECTION, None, None),
                election(OTHER_ELECTION, Some(KEYS_CEREMONY), None),
            ],
            "Election has no keys ceremony",
        ),
    ];
    for (elections, refusal) in cases {
        let ceremony = InMemoryTallyCeremony::default();
        ceremony.add_election_event(election_event(json!({}), event_board()));
        let election_ids: Vec<String> = elections.iter().map(|e| e.id.clone()).collect();
        for election in elections {
            ceremony.add_election(election);
        }
        ceremony.add_keys_ceremony(keys_ceremony("SUCCESS", "manual-ceremonies"));
        let election_ids: Vec<&str> = election_ids.iter().map(String::as_str).collect();
        let error = create_tally(&ceremony, ELECTORAL_RESULTS, &election_ids, &[])
            .await
            .unwrap_err();
        assert_eq!(validation_message(error), refusal);
        assert_nothing_written(&ceremony);
    }
}

#[tokio::test]
async fn an_unfinished_keys_ceremony_cannot_be_tallied() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(election_event(json!({}), event_board()));
    ceremony.add_election(election(ELECTION, Some(KEYS_CEREMONY), None));
    ceremony.add_keys_ceremony(keys_ceremony("IN_PROGRESS", "manual-ceremonies"));
    let error = create(&ceremony).await.unwrap_err();
    assert_eq!(validation_message(error), "Invalid keys ceremony");
    assert_nothing_written(&ceremony);
}

#[tokio::test]
async fn single_contest_sessions_decrypt_each_votable_contest() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(election_event(json!({}), event_board()));
    ceremony.add_election(election(ELECTION, Some(KEYS_CEREMONY), None));
    ceremony.add_keys_ceremony(keys_ceremony("SUCCESS", "manual-ceremonies"));
    ceremony.add_ballot_style(published(
        ELECTION,
        "north",
        vec![plurality("mayor"), acclaimed("council")],
        None,
    ));
    ceremony.add_ballot_style(published(
        ELECTION,
        "south",
        vec![acclaimed("council")],
        None,
    ));
    create(&ceremony).await.unwrap();
    let decrypted: Vec<_> = ceremony
        .session_contests(FIRST_ID)
        .into_iter()
        .map(|session_contest| {
            (
                session_contest.election_id,
                session_contest.area_id,
                session_contest.contest_id,
            )
        })
        .collect();
    assert_eq!(
        decrypted,
        vec![(ELECTION.into(), "north".into(), Some("mayor".into()))]
    );
}

#[tokio::test]
async fn multi_contest_sessions_skip_fully_acclaimed_areas() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(election_event(
        json!({"contest_encryption_policy": "multiple-contests"}),
        event_board(),
    ));
    ceremony.add_election(election(ELECTION, Some(KEYS_CEREMONY), None));
    ceremony.add_keys_ceremony(keys_ceremony("SUCCESS", "manual-ceremonies"));
    ceremony.add_ballot_style(published(
        ELECTION,
        "north",
        vec![plurality("mayor"), acclaimed("council")],
        None,
    ));
    ceremony.add_ballot_style(published(
        ELECTION,
        "south",
        vec![acclaimed("council")],
        None,
    ));
    create(&ceremony).await.unwrap();
    let decrypted: Vec<_> = ceremony
        .session_contests(FIRST_ID)
        .into_iter()
        .map(|session_contest| (session_contest.area_id, session_contest.contest_id))
        .collect();
    assert_eq!(decrypted, vec![("north".to_string(), None)]);
}

#[tokio::test]
async fn each_contest_area_gets_its_own_run_of_batches_after_the_existing_ones() {
    let stride = VOTE_WEIGHT_BATCHES as i32;
    let ceremony = closed_event(json!({}));
    ceremony.add_ballot_style(published(ELECTION, "east", vec![plurality("mayor")], None));
    ceremony.add_session_contest(TallySessionContest {
        id: "earlier".into(),
        tenant_id: TENANT.into(),
        election_event_id: EVENT.into(),
        area_id: "north".into(),
        contest_id: Some("mayor".into()),
        session_id: 10,
        created_at: None,
        last_updated_at: None,
        labels: None,
        annotations: None,
        tally_session_id: "earlier-session".into(),
        election_id: ELECTION.into(),
    });
    create(&ceremony).await.unwrap();
    assert_eq!(
        batches(&ceremony.session_contests(FIRST_ID)),
        HashSet::from([10 + stride, 10 + 2 * stride, 10 + 3 * stride])
    );
}

#[tokio::test]
async fn the_first_session_of_an_event_starts_at_batch_zero() {
    let stride = VOTE_WEIGHT_BATCHES as i32;
    let ceremony = closed_event(json!({}));
    create(&ceremony).await.unwrap();
    assert_eq!(
        batches(&ceremony.session_contests(FIRST_ID)),
        HashSet::from([0, stride])
    );
}

#[tokio::test]
async fn creating_a_session_posts_a_key_insertion_start_entry() {
    let ceremony = closed_event(json!({}));
    create(&ceremony).await.unwrap();
    assert_eq!(
        ceremony.audit_entries(),
        vec![TallyAuditEntry::KeyInsertionStarted {
            board_name: EVENT_BOARD.into(),
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            election_ids: vec![ELECTION.into()],
            user_id: ADMIN_ID.into(),
            username: ADMIN.into(),
        }]
    );
}

#[tokio::test]
async fn the_session_is_stored_before_the_bulletin_board_is_resolved() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_election_event(election_event(json!({}), None));
    ceremony.add_election(election(ELECTION, Some(KEYS_CEREMONY), None));
    ceremony.add_keys_ceremony(keys_ceremony("SUCCESS", "manual-ceremonies"));
    ceremony.add_ballot_style(published(ELECTION, "north", vec![plurality("mayor")], None));
    let error = create(&ceremony).await.unwrap_err();
    assert_eq!(error.to_string(), "missing bulletin board");
    assert_eq!(ceremony.sessions().len(), 1);
    assert_eq!(ceremony.executions(FIRST_ID).len(), 1);
    assert_eq!(ceremony.session_contests(FIRST_ID).len(), 1);
    assert_eq!(ceremony.audit_entries(), vec![]);
}

#[tokio::test]
async fn a_published_ballot_style_must_hold_a_readable_ballot() {
    let missing = closed_event(json!({}));
    let mut without_ballot = published(ELECTION, "east", vec![plurality("mayor")], None);
    without_ballot.ballot_eml = None;
    missing.add_ballot_style(without_ballot);
    assert_eq!(
        create(&missing).await.unwrap_err().to_string(),
        format!("Published ballot style style-{ELECTION}-east has no ballot EML")
    );

    let unreadable = closed_event(json!({}));
    let mut garbled = published(ELECTION, "east", vec![plurality("mayor")], None);
    garbled.ballot_eml = Some("{".into());
    unreadable.add_ballot_style(garbled);
    assert!(create(&unreadable)
        .await
        .unwrap_err()
        .to_string()
        .starts_with(&format!(
            "Could not read published ballot style style-{ELECTION}-east: "
        )));
    assert_nothing_written(&unreadable);
}

#[tokio::test]
async fn approved_sheet_with_complete_review_blocks_voter_weighted_creation() {
    let ceremony = closed_event(voter_weighted());
    ceremony.add_tally_sheet(approved_tally_sheet(ELECTION));
    let error = create(&ceremony).await.unwrap_err();
    assert!(validation_message(error)
        .contains("1 approved tally sheet(s) exist for this election event"));
    assert_nothing_written(&ceremony);
}

async fn assert_sheet_does_not_block_creation(sheet: TallySheet) {
    let ceremony = closed_event(voter_weighted());
    ceremony.add_tally_sheet(sheet);
    assert_eq!(create(&ceremony).await.unwrap(), FIRST_ID);
    assert_eq!(ceremony.session_contests(FIRST_ID).len(), 2);
    assert_eq!(ceremony.audit_entries().len(), 1);
}

#[tokio::test]
async fn approved_sheet_without_review_timestamp_does_not_block_creation() {
    let mut sheet = approved_tally_sheet(ELECTION);
    sheet.reviewed_at = None;
    assert_sheet_does_not_block_creation(sheet).await;
}

#[tokio::test]
async fn approved_sheet_without_reviewer_does_not_block_creation() {
    let mut sheet = approved_tally_sheet(ELECTION);
    sheet.reviewed_by_user_id = None;
    assert_sheet_does_not_block_creation(sheet).await;
}

#[tokio::test]
async fn pending_or_disapproved_sheet_does_not_block_creation() {
    for status in [TallySheetStatus::PENDING, TallySheetStatus::DISAPPROVED] {
        let mut sheet = approved_tally_sheet(ELECTION);
        sheet.status = status;
        assert_sheet_does_not_block_creation(sheet).await;
    }
}

#[tokio::test]
async fn deleted_approved_sheet_does_not_block_creation() {
    let mut sheet = approved_tally_sheet(ELECTION);
    sheet.deleted_at = sheet.reviewed_at;
    assert_sheet_does_not_block_creation(sheet).await;
}

#[tokio::test]
async fn approved_sheet_from_another_tenant_or_event_does_not_block_creation() {
    let mut other_tenant = approved_tally_sheet(ELECTION);
    other_tenant.tenant_id = "other-tenant".into();
    assert_sheet_does_not_block_creation(other_tenant).await;
    let mut other_event = approved_tally_sheet(ELECTION);
    other_event.election_event_id = "other-event".into();
    assert_sheet_does_not_block_creation(other_event).await;
}

fn assert_creation_writes(
    ceremony: &InMemoryTallyCeremony,
    sessions: usize,
    executions: usize,
    contests: usize,
) {
    let rows = ceremony.sessions();
    assert_eq!(rows.len(), sessions);
    if let Some(row) = rows.first() {
        assert_eq!(row.id, FIRST_ID);
        assert_eq!(row.tenant_id, TENANT);
        assert_eq!(row.election_event_id, EVENT);
    }
    let snapshots = ceremony.executions(FIRST_ID);
    assert_eq!(snapshots.len(), executions);
    if let Some(snapshot) = snapshots.first() {
        assert_eq!(snapshot.current_message_id, -1);
        assert_eq!(snapshot.run_reason.as_deref(), Some("NORMAL"));
    }
    let rows = ceremony.session_contests(FIRST_ID);
    assert_eq!(rows.len(), contests);
    if !rows.is_empty() {
        assert_eq!(
            rows.iter()
                .map(|row| row.area_id.as_str())
                .collect::<HashSet<_>>(),
            HashSet::from(["north", "south"])
        );
    }
    assert!(ceremony.audit_entries().is_empty());
}

async fn assert_creation_failure(
    call: TallyCall,
    sessions: usize,
    executions: usize,
    contests: usize,
) {
    let ceremony = closed_event(voter_weighted());
    ceremony.fail(call, "creation dependency unavailable");
    let error = create(&ceremony).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "creation dependency unavailable",
        "{call:?}"
    );
    assert!(error.downcast_ref::<TallyValidationError>().is_none());
    assert_creation_writes(&ceremony, sessions, executions, contests);
}

#[tokio::test]
async fn session_insert_failure_leaves_creation_empty() {
    assert_creation_failure(TallyCall::InsertSession, 0, 0, 0).await;
}

#[tokio::test]
async fn execution_insert_failure_keeps_only_the_session() {
    assert_creation_failure(TallyCall::AppendExecution, 1, 0, 0).await;
}

#[tokio::test]
async fn batch_allocation_failure_keeps_the_initial_execution_without_contests() {
    assert_creation_failure(TallyCall::NextBatch, 1, 1, 0).await;
}

#[tokio::test]
async fn contest_insert_failure_stops_before_reading_the_event_for_audit() {
    let ceremony = closed_event(voter_weighted());
    ceremony.fail(TallyCall::InsertContest, "contest insertion failed");
    let election_events = InMemoryTallyCeremony::default();
    election_events.fail(TallyCall::GetElectionEvent, "event read must not run");
    assert_eq!(
        create_tally_with_events(
            &ceremony,
            &election_events,
            ELECTORAL_RESULTS,
            &[ELECTION],
            &[]
        )
        .await
        .unwrap_err()
        .to_string(),
        "contest insertion failed"
    );
    assert_creation_writes(&ceremony, 1, 1, 0);
}

#[tokio::test]
async fn creation_audit_failure_keeps_the_session_execution_and_contests() {
    assert_creation_failure(TallyCall::KeyInsertionStarted, 1, 1, 2).await;
}

#[tokio::test]
async fn creation_event_read_failure_keeps_the_prior_writes_without_audit() {
    let ceremony = closed_event(voter_weighted());
    let election_events = InMemoryTallyCeremony::default();
    election_events.fail(TallyCall::GetElectionEvent, "event read unavailable");
    let error = create_tally_with_events(
        &ceremony,
        &election_events,
        ELECTORAL_RESULTS,
        &[ELECTION],
        &[],
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "event read unavailable");
    assert!(error.downcast_ref::<TallyValidationError>().is_none());
    assert_creation_writes(&ceremony, 1, 1, 2);
}

#[tokio::test]
async fn creation_source_read_failures_happen_before_any_writes() {
    for call in [
        TallyCall::EventSnapshot,
        TallyCall::PublishedBallotStyles,
        TallyCall::ApprovedTallySheets,
        TallyCall::GetKeysCeremony,
    ] {
        assert_creation_failure(call, 0, 0, 0).await;
    }
}
