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
        borsh::to_vec(&StatementType::ResultsPublicationAction).unwrap()[0],
        26
    );
    assert_eq!(
        borsh::to_vec(&StatementType::ExternalApiRequest).unwrap()[0],
        27
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
