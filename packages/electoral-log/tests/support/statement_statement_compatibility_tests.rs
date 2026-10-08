// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn external_api_request_description(operation: &str) -> String {
    let event_id = EventIdString("0609dd53-3c33-41cd-b2cd-0ffb39738d2d".to_string());
    let body = StatementBody::ExternalApiRequest(
        event_id.clone(),
        ExternalApiSubject {
            user_id: Some("voter-id".to_string()),
            username: Some("voter-name".to_string()),
        },
        ExtApiRequestDirection::Outbound,
        ExtApiName::Datafix,
        operation.to_string(),
    );
    StatementHead::from_body(event_id, &body).description
}

#[test]
fn external_api_request_description_success() {
    assert_eq!(
        external_api_request_description("SetVoted Succeeded"),
        "Outbound request SetVoted Succeeded."
    );
}

#[test]
fn external_api_request_description_failure_without_detail() {
    assert_eq!(
        external_api_request_description("SetNotVoted Failed"),
        "Outbound request SetNotVoted Failed."
    );
}

#[test]
fn external_api_request_description_excludes_detail() {
    assert_eq!(
        external_api_request_description("SetNotVoted Failed: The voter has not voted."),
        "Outbound request SetNotVoted Failed."
    );
}

#[test]
fn statement_body_borsh_discriminants_are_append_only() {
    let election_publish = StatementBody::ElectionPublish(
        ElectionIdString(None),
        BallotPublicationIdString(String::new()),
    );
    let certificate = StatementBody::CertificateAuthEvent(
        CertificateAuthEventAction::Import,
        CertificateSubjectDnsString(Vec::new()),
    );
    let phone = StatementBody::PhoneBlacklistUpdated(
        PhoneE164String(String::new()),
        PhoneBlacklistAction::CreateEntry,
    );
    let external = StatementBody::ExternalApiRequest(
        EventIdString(String::new()),
        ExternalApiSubject {
            user_id: None,
            username: None,
        },
        ExtApiRequestDirection::Outbound,
        ExtApiName::Datafix,
        String::new(),
    );
    let cast_vote_with_channel = StatementBody::CastVoteWithChannel(
        ElectionIdString(None),
        PseudonymHash::new([0; 64]),
        CastVoteHash::new([0; 64]),
        VoterIpString(String::new()),
        VoterCountryString(String::new()),
        VotingChannelString(String::new()),
    );
    let external_reconciliation = StatementBody::ExternalReconciliation(
        EventIdString(String::new()),
        ExternalReconciliationKind::PatchGenerated,
        ExternalReconciliationSequenceString(String::new()),
        ExternalReconciliationGeneratedAtString(String::new()),
        ExternalReconciliationInputHashString(String::new()),
        ExternalReconciliationOutputHashString(None),
    );

    assert_eq!(borsh::to_vec(&election_publish).unwrap()[0], 2);
    assert_eq!(borsh::to_vec(&certificate).unwrap()[0], 23);
    assert_eq!(borsh::to_vec(&phone).unwrap()[0], 24);
    // `ExternalApiRequest` follows `ResultsPublicationAction` in the
    // released enum. The previous expectation of 25 was left stale by the
    // rebase that introduced `ExternalApiRequest`.
    assert_eq!(borsh::to_vec(&external).unwrap()[0], 26);
    assert_eq!(borsh::to_vec(&cast_vote_with_channel).unwrap()[0], 27);
    assert_eq!(borsh::to_vec(&external_reconciliation).unwrap()[0], 28);
    assert_eq!(borsh::to_vec(&monitoring_config_changed()).unwrap()[0], 29);
    assert_eq!(borsh::to_vec(&signing()).unwrap()[0], 30);
    for (body, tag) in ballot_box_bodies().iter().zip(31u8..) {
        assert_eq!(borsh::to_vec(body).unwrap()[0], tag);
    }
}

fn ballot_box_bodies() -> Vec<StatementBody> {
    let election = || ElectionIdString(Some("election".to_string()));
    let area = || AreaIdString("area".to_string());
    vec![
        StatementBody::BallotBoxSealed(election(), area(), SealHash::new([7; 64]), 3, 2, None),
        StatementBody::BallotBoxSealFailed(election(), area(), "reason".to_string()),
        StatementBody::TallyBallotBoxVerified(
            election(),
            area(),
            SealHash::new([7; 64]),
            2,
            "session".to_string(),
        ),
        StatementBody::TallyBallotBoxRejected(
            election(),
            area(),
            "what".to_string(),
            "session".to_string(),
        ),
    ]
}

/// The seal kinds are appended after `LockdownChanged`; the failures are
/// ERROR entries, every one is a SYSTEM entry.
#[test]
fn the_ballot_box_seal_kinds_are_appended() {
    let expected = [
        (StatementType::BallotBoxSealed, 52, StatementLogType::INFO),
        (
            StatementType::BallotBoxSealFailed,
            53,
            StatementLogType::ERROR,
        ),
        (
            StatementType::TallyBallotBoxVerified,
            54,
            StatementLogType::INFO,
        ),
        (
            StatementType::TallyBallotBoxRejected,
            55,
            StatementLogType::ERROR,
        ),
    ];
    for (body, (kind, tag, log_type)) in ballot_box_bodies().iter().zip(expected) {
        assert_eq!(borsh::to_vec(&kind).unwrap(), vec![tag]);
        let head = StatementHead::from_body(EventIdString("event".to_string()), body);
        assert_eq!(head.kind.to_string(), kind.to_string());
        assert_eq!(head.log_type, log_type);
        assert_eq!(head.event_type, StatementEventType::SYSTEM);
        assert!(head.kind.to_string().len() <= 40);
    }
}

#[test]
fn a_seal_description_names_the_box_and_groups_thousands() {
    assert_eq!(
        ballot_box_sealed_description("Madrid PE", "Spain", 1340, 1342),
        "Ballot box of Madrid PE, Spain sealed: 1,340 of 1,342 ballots counted."
    );
    assert_eq!(group_thousands(0), "0");
    assert_eq!(group_thousands(999), "999");
    assert_eq!(group_thousands(1_000_000), "1,000,000");
}

fn signing() -> StatementBody {
    StatementBody::Signing(SigningLogEntry {
        kind: SigningStatementKind::SigningRequestCreated,
        event_type: StatementEventType::USER,
        log_type: StatementLogType::INFO,
        description: String::new(),
        details_json: String::new(),
        step_id: String::new(),
    })
}

fn monitoring_config_changed() -> StatementBody {
    StatementBody::MonitoringConfigChanged(
        EventIdString(String::new()),
        MonitoringConfigChangeDetails {
            origin: MonitoringConfigOrigin::Editor,
            preset: None,
            mode: MonitoringDashboardMode::Configured,
            generation: 0,
            revisions: Vec::new(),
        },
    )
}

#[test]
fn legacy_cast_vote_body_remains_deserializable() {
    let legacy = StatementBody::CastVote(
        ElectionIdString(Some("election-id".to_string())),
        PseudonymHash::new([1; 64]),
        CastVoteHash::new([2; 64]),
        VoterIpString("ip".to_string()),
        VoterCountryString("country".to_string()),
    );

    let bytes = borsh::to_vec(&legacy).unwrap();
    assert_eq!(bytes[0], 0);
    let decoded: StatementBody = borsh::from_slice(&bytes).unwrap();
    assert!(matches!(decoded, StatementBody::CastVote(_, _, _, _, _)));
}

#[test]
fn statement_type_borsh_discriminants_are_append_only() {
    assert_eq!(
        borsh::to_vec(&StatementType::ElectionPublish).unwrap()[0],
        3
    );
    assert_eq!(
        borsh::to_vec(&StatementType::CertificateAuthEvent).unwrap()[0],
        24
    );
    assert_eq!(
        borsh::to_vec(&StatementType::PhoneBlacklistUpdated).unwrap()[0],
        25
    );
    assert_eq!(
        borsh::to_vec(&StatementType::ExternalReconciliation).unwrap(),
        vec![28]
    );
    assert_eq!(
        borsh::to_vec(&StatementType::ResultsPublicationAction).unwrap()[0],
        26
    );
    assert_eq!(
        borsh::to_vec(&StatementType::ExternalApiRequest).unwrap()[0],
        27
    );
    assert_eq!(
        borsh::to_vec(&StatementType::MonitoringConfigChanged).unwrap(),
        vec![29]
    );
    assert_eq!(
        borsh::to_vec(&StatementType::SigningRequestCreated).unwrap(),
        vec![30]
    );
    assert_eq!(
        borsh::to_vec(&StatementType::SigningRequestsExported).unwrap(),
        vec![45]
    );
    assert_eq!(
        borsh::to_vec(&StatementType::LifecycleWindowChanged).unwrap(),
        vec![46]
    );
    assert_eq!(
        borsh::to_vec(&StatementType::ScheduleRecomputeApplied).unwrap(),
        vec![47]
    );
    assert_eq!(
        borsh::to_vec(&StatementType::ScheduleImported).unwrap(),
        vec![48]
    );
}

#[test]
fn external_api_subject_is_part_of_borsh_payload() {
    let without_subject = StatementBody::ExternalApiRequest(
        EventIdString("event".to_string()),
        ExternalApiSubject {
            user_id: None,
            username: None,
        },
        ExtApiRequestDirection::Outbound,
        ExtApiName::Datafix,
        "SetVoted Succeeded".to_string(),
    );
    let with_subject = StatementBody::ExternalApiRequest(
        EventIdString("event".to_string()),
        ExternalApiSubject {
            user_id: Some("voter-id".to_string()),
            username: Some("voter-name".to_string()),
        },
        ExtApiRequestDirection::Outbound,
        ExtApiName::Datafix,
        "SetVoted Succeeded".to_string(),
    );

    assert_ne!(
        borsh::to_vec(&without_subject).unwrap(),
        borsh::to_vec(&with_subject).unwrap()
    );
}

#[test]
fn a_schedule_import_is_a_signing_outbox_kind_of_its_own_type() {
    assert!(matches!(
        SigningStatementKind::ScheduleImported.statement_type(),
        StatementType::ScheduleImported
    ));
    assert_eq!(
        borsh::to_vec(&SigningStatementKind::ScheduleImported).unwrap(),
        vec![18]
    );
}
