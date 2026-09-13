// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Validate persisted configuration at its boundary, including optional JSON
//! fields. Invalid nested data must not be accepted merely because the row parses.

use sequent_core::ballot::*;
use sequent_core::types::ceremonies::{
    AutomaticRecountPolicy, CeremoniesPolicy,
};
use sequent_core::types::hasura::core::{self, DocumentAnnotations};
use serde_json::{json, Value};

fn row() -> Value {
    json!({"id": "row-1", "tenant_id": "tenant-1", "election_event_id": "event-1",
        "election_id": "election-1", "contest_id": "contest-1",
        "is_archived": false, "encryption_protocol": "ELGAMAL"})
}

#[test]
fn event_and_election_validation_checks_every_nested_configuration() {
    let event_fields = [
        (
            "presentation",
            serde_json::to_value(ElectionEventPresentation::default()).unwrap(),
        ),
        ("voting_channels", json!({"online": true})),
        ("status", json!({"voting_status": "OPEN"})),
        (
            "statistics",
            serde_json::to_value(ElectionEventStatistics::default()).unwrap(),
        ),
        (
            "bulletin_board_reference",
            json!({"id": 1, "database_name": "board", "is_archived": false}),
        ),
    ];
    let empty: core::ElectionEvent = serde_json::from_value(row()).unwrap();
    empty.validate().unwrap();
    for (field, valid) in event_fields {
        let mut data = row();
        data[field] = valid;
        serde_json::from_value::<core::ElectionEvent>(data.clone())
            .unwrap()
            .validate()
            .unwrap();
        data[field] = json!(["not a configuration object"]);
        assert!(
            serde_json::from_value::<core::ElectionEvent>(data)
                .unwrap()
                .validate()
                .is_err(),
            "{field}"
        );
    }
    let empty: core::Election = serde_json::from_value(row()).unwrap();
    empty.validate().unwrap();
    for (field, valid) in [
        (
            "presentation",
            serde_json::to_value(ElectionPresentation::default()).unwrap(),
        ),
        ("voting_channels", json!({"kiosk": false})),
        ("status", json!({"voting_status": "CLOSED"})),
        (
            "statistics",
            serde_json::to_value(ElectionStatistics::default()).unwrap(),
        ),
    ] {
        let mut data = row();
        data[field] = valid;
        serde_json::from_value::<core::Election>(data.clone())
            .unwrap()
            .validate()
            .unwrap();
        data[field] = json!("broken");
        assert!(
            serde_json::from_value::<core::Election>(data)
                .unwrap()
                .validate()
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn candidate_and_contest_presentations_are_optional_but_must_have_the_right_shape(
) {
    let mut data = row();
    serde_json::from_value::<core::Candidate>(data.clone())
        .unwrap()
        .validate()
        .unwrap();
    serde_json::from_value::<core::Contest>(data.clone())
        .unwrap()
        .validate()
        .unwrap();
    data["presentation"] =
        serde_json::to_value(CandidatePresentation::new()).unwrap();
    serde_json::from_value::<core::Candidate>(data.clone())
        .unwrap()
        .validate()
        .unwrap();
    data["presentation"] =
        serde_json::to_value(ContestPresentation::new()).unwrap();
    serde_json::from_value::<core::Contest>(data.clone())
        .unwrap()
        .validate()
        .unwrap();
    data["presentation"] = json!(false);
    assert!(serde_json::from_value::<core::Candidate>(data.clone())
        .unwrap()
        .validate()
        .is_err());
    assert!(serde_json::from_value::<core::Contest>(data)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn sensitive_document_annotations_keep_password_and_voter_secret_permissions_distinct(
) {
    let ordinary: DocumentAnnotations =
        serde_json::from_value(json!({})).unwrap();
    assert_eq!(ordinary.password_secret_id(), None);
    assert!(!ordinary.requires_voter_secret_attribute_read());
    assert_eq!(serde_json::to_value(&ordinary).unwrap(), json!({}));

    let password = DocumentAnnotations::password_protected("secret-reference");
    assert_eq!(password.password_secret_id(), Some("secret-reference"));
    assert!(!password.requires_voter_secret_attribute_read());
    assert_eq!(
        serde_json::to_value(password).unwrap(),
        json!({"access": {"password_secret_id": "secret-reference"}})
    );

    let voter_export = DocumentAnnotations::voter_secret_export();
    assert_eq!(voter_export.password_secret_id(), None);
    assert!(voter_export.requires_voter_secret_attribute_read());
    assert_eq!(
        serde_json::to_value(voter_export).unwrap(),
        json!({"access": {"voter_secret_attributes": true}})
    );
}

#[test]
fn area_weights_default_to_one_and_reject_malformed_annotations() {
    let ordinary = AreaAnnotations::default();
    assert_eq!(*ordinary.get_weight(), Some(1));
    for weight in [0, 7, u64::MAX] {
        let annotations: AreaAnnotations =
            serde_json::from_value(json!({"weight": weight})).unwrap();
        assert_eq!(*annotations.get_weight(), Some(weight));
    }
    let mut data = row();
    data["area_id"] = json!("area-1");
    let area: core::Area = serde_json::from_value(data.clone()).unwrap();
    assert_eq!(area.read_annotations().unwrap(), None);
    data["annotations"] = json!({"weight": 7});
    let area: core::Area = serde_json::from_value(data.clone()).unwrap();
    assert_eq!(
        *area.read_annotations().unwrap().unwrap().get_weight(),
        Some(7)
    );
    data["annotations"] = json!({"weight": -1});
    assert!(serde_json::from_value::<core::Area>(data)
        .unwrap()
        .read_annotations()
        .is_err());
}

#[test]
fn ceremony_status_and_policies_reject_invalid_state_but_keep_safe_defaults() {
    use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
    let mut data = row();
    data["threshold"] = json!(2);
    data["trustee_ids"] = json!([]);
    let mut ceremony: core::KeysCeremony =
        serde_json::from_value(data).unwrap();
    assert!(ceremony.is_default());
    assert!(ceremony.execution_status().is_err());
    assert!(ceremony.status().is_err());
    assert_eq!(ceremony.policy(), CeremoniesPolicy::MANUAL_CEREMONIES);
    ceremony.is_default = Some(false);
    ceremony.execution_status = Some("SUCCESS".into());
    ceremony.status = Some(json!({"logs": [], "trustees": []}));
    assert!(!ceremony.is_default());
    assert_eq!(
        ceremony.execution_status().unwrap(),
        KeysCeremonyExecutionStatus::SUCCESS
    );
    assert!(ceremony.status().unwrap().trustees.is_empty());
    ceremony.settings = Some(json!({"policy": "invalid-policy"}));
    assert_eq!(ceremony.policy(), CeremoniesPolicy::MANUAL_CEREMONIES);
}

#[test]
fn tally_configuration_accessors_do_not_drop_explicit_encryption_or_publication_policies(
) {
    let empty = core::TallySessionConfiguration::default();
    assert_eq!(
        empty.get_contest_encryption_policy(),
        ContestEncryptionPolicy::SINGLE_CONTEST
    );
    assert_eq!(
        empty.get_delegated_voting_policy(),
        DelegatedVotingPolicy::DISABLED
    );
    assert_eq!(
        empty.get_weighted_voting_policy(),
        WeightedVotingPolicy::DISABLED_WEIGHTED_VOTING
    );
    assert_eq!(
        empty.get_decoded_ballots_policy(),
        DecodedBallotsInclusionPolicy::NOT_INCLUDED
    );
    assert_eq!(
        empty.get_consolidated_report_policy(),
        ConsolidatedReportPolicy::DO_NOT_GENERATE
    );
    let configured = core::TallySessionConfiguration {
        contest_encryption_policy: Some(
            ContestEncryptionPolicy::MULTIPLE_CONTESTS,
        ),
        delegated_voting_policy: Some(DelegatedVotingPolicy::ENABLED),
        weighted_voting_policy: Some(
            WeightedVotingPolicy::AREAS_WEIGHTED_VOTING,
        ),
        decoded_ballots_inclusion_policy: Some(
            DecodedBallotsInclusionPolicy::INCLUDED,
        ),
        consolidated_report_policy: Some(ConsolidatedReportPolicy::GENERATE),
        ..Default::default()
    };
    assert_eq!(
        configured.get_contest_encryption_policy(),
        ContestEncryptionPolicy::MULTIPLE_CONTESTS
    );
    assert_eq!(
        configured.get_delegated_voting_policy(),
        DelegatedVotingPolicy::ENABLED
    );
    assert_eq!(
        configured.get_weighted_voting_policy(),
        WeightedVotingPolicy::AREAS_WEIGHTED_VOTING
    );
    assert_eq!(
        configured.get_decoded_ballots_policy(),
        DecodedBallotsInclusionPolicy::INCLUDED
    );
    assert_eq!(
        configured.get_consolidated_report_policy(),
        ConsolidatedReportPolicy::GENERATE
    );
}

#[test]
fn result_downloads_select_the_requested_format_and_keep_missing_documents_absent(
) {
    use sequent_core::types::results::{ResultDocumentType, ResultDocuments};
    let documents = ResultDocuments {
        json: Some("results.json".into()),
        pdf: Some("report.pdf".into()),
        html: Some("results.html".into()),
        tar_gz: Some("results.tar.gz".into()),
        tar_gz_original: Some("original.tar.gz".into()),
        ..Default::default()
    };
    for (format, expected) in [
        (ResultDocumentType::Json, "results.json"),
        (ResultDocumentType::Pdf, "report.pdf"),
        (ResultDocumentType::Html, "results.html"),
        (ResultDocumentType::TarGz, "results.tar.gz"),
        (ResultDocumentType::TarGzOriginal, "original.tar.gz"),
    ] {
        assert_eq!(
            documents.get_document_by_type(&format).as_deref(),
            Some(expected)
        );
        assert_eq!(
            ResultDocuments::default().get_document_by_type(&format),
            None
        );
    }
}

#[test]
fn tally_sheet_channel_defaults_do_not_override_recognized_channels() {
    use sequent_core::types::tally_sheets::VotingChannel;
    assert_eq!(VotingChannel::from(None), VotingChannel::PAPER);
    assert_eq!(
        VotingChannel::from(Some("unknown".into())),
        VotingChannel::PAPER
    );
    assert_eq!(
        VotingChannel::from(Some("POSTAL".into())),
        VotingChannel::POSTAL
    );
    assert_eq!(
        VotingChannel::from(Some("IN_PERSON".into())),
        VotingChannel::IN_PERSON
    );
}

#[test]
fn persisted_event_policies_keep_explicit_choices_and_default_unknown_values() {
    for presentation in [
        Value::Null,
        json!({}),
        json!({"automatic_recount_policy": "future-policy", "materials": []}),
    ] {
        let mut data = row();
        data["presentation"] = presentation;
        let event: core::ElectionEvent = serde_json::from_value(data).unwrap();
        assert_eq!(
            event.automatic_recount_policy(),
            AutomaticRecountPolicy::DISABLED
        );
        assert_eq!(
            event.effective_support_materials_policy(),
            SupportMaterialsPolicy::default()
        );
    }
    let mut data = row();
    data["presentation"] = json!({"automatic_recount_policy": "enabled", "materials": {"policy": "mandatory_for_voting"}});
    let event: core::ElectionEvent = serde_json::from_value(data).unwrap();
    assert_eq!(
        event.automatic_recount_policy(),
        AutomaticRecountPolicy::ENABLED
    );
    assert_eq!(
        event.effective_support_materials_policy(),
        SupportMaterialsPolicy::MandatoryForVoting
    );
}
