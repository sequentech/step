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
    assert_eq!(
        borsh::to_vec(&StatementType::ConfigurationPackageImported).unwrap(),
        vec![52]
    );
    // A publication is an `ElectionPublish` entry: it has no type of its own.
    assert_eq!(
        borsh::to_vec(&StatementType::ReportGenerated).unwrap(),
        vec![53]
    );
    assert_eq!(
        borsh::to_vec(&StatementType::ApprovalMatrixUpdated).unwrap(),
        vec![54]
    );
    assert!(borsh::from_slice::<StatementType>(&[55]).is_err());
}

fn report_generated(document_id: Option<&str>) -> StatementBody {
    StatementBody::ReportGenerated(
        EventIdString("event".to_string()),
        ReportGeneratedDetails {
            report_type: "ACTIVITY_LOGS".to_string(),
            document_id: document_id.map(str::to_string),
            report_manifest_sha256: "12".repeat(32),
            external_id: "ov-2028".to_string(),
            revision: 8,
            manifest_sha256: "ab".repeat(32),
        },
    )
}

#[test]
fn a_generated_report_entry_is_appended_and_names_its_hash_manifest() {
    let stored = report_generated(Some("document"));
    let mut expected = vec![33];
    expected.extend(text("event"));
    expected.extend(text("ACTIVITY_LOGS"));
    expected.push(1);
    expected.extend(text("document"));
    expected.extend(text(&"12".repeat(32)));
    expected.extend(text("ov-2028"));
    expected.extend(8_u64.to_le_bytes());
    expected.extend(text(&"ab".repeat(32)));
    assert_eq!(borsh::to_vec(&stored).unwrap(), expected);
    assert!(matches!(
        borsh::from_slice::<StatementBody>(&expected).unwrap(),
        StatementBody::ReportGenerated(_, _)
    ));
    assert!(borsh::from_slice::<StatementBody>(&[35]).is_err());

    let head = StatementHead::from_body(EventIdString("event".to_string()), &stored);
    assert!(matches!(head.kind, StatementType::ReportGenerated));
    assert!(matches!(head.event_type, StatementEventType::SYSTEM));
    assert!(matches!(head.log_type, StatementLogType::INFO));
    assert_eq!(
        head.description,
        format!(
            "ACTIVITY_LOGS report generated with hash manifest {} (document document), for configuration ov-2028 revision 8 (manifest {}).",
            "12".repeat(32),
            "ab".repeat(32)
        )
    );

    let in_a_folder =
        StatementHead::from_body(EventIdString("event".to_string()), &report_generated(None));
    assert_eq!(
        in_a_folder.description,
        format!(
            "ACTIVITY_LOGS report generated with hash manifest {}, for configuration ov-2028 revision 8 (manifest {}).",
            "12".repeat(32),
            "ab".repeat(32)
        )
    );
}

#[test]
fn the_generated_report_kind_fits_its_column() {
    assert!(StatementType::ReportGenerated.to_string().len() <= 40);
}

fn design(area: &str, byte: &str) -> ConfigurationDesignDigest {
    ConfigurationDesignDigest {
        area: area.to_string(),
        election: "national".to_string(),
        sha256: byte.repeat(32),
    }
}

fn configuration_package() -> StatementBody {
    StatementBody::ConfigurationPackage(
        EventIdString("event".to_string()),
        ConfigurationPackageDetails {
            action: ConfigurationPackageAction::Imported,
            external_id: "ov-2028".to_string(),
            revision: 8,
            manifest_sha256: "ab".repeat(32),
            design_digests: vec![design("Post 1", "cd")],
        },
    )
}

fn published_configuration() -> PublishedConfiguration {
    PublishedConfiguration {
        external_id: "ov-2028".to_string(),
        revision: 8,
        manifest_sha256: "ab".repeat(32),
        design_digests: vec![design("Post 1", "cd"), design("Post 2", "ef")],
    }
}

fn text(value: &str) -> Vec<u8> {
    let mut bytes = (value.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

/// The body of an `ElectionPublish` entry as the logs written before signed
/// configurations hold it, assembled by hand: the variant index, the
/// optional election id and the publication id.
fn election_publish_bytes_before_configurations() -> Vec<u8> {
    let mut bytes = vec![2, 1];
    bytes.extend(text("election"));
    bytes.extend(text("publication"));
    bytes
}

#[test]
fn a_configuration_package_entry_is_appended_and_says_it_was_imported() {
    let imported = configuration_package();
    assert_eq!(borsh::to_vec(&imported).unwrap()[0], 31);
    let head = StatementHead::from_body(EventIdString("event".to_string()), &imported);
    assert!(matches!(
        head.kind,
        StatementType::ConfigurationPackageImported
    ));
    assert!(head.description.contains("ov-2028 revision 8 imported"));
    let decoded: StatementBody = borsh::from_slice(&borsh::to_vec(&imported).unwrap()).unwrap();
    assert!(matches!(decoded, StatementBody::ConfigurationPackage(_, _)));
}

#[test]
fn an_election_publish_body_written_before_configurations_still_decodes() {
    let bytes = election_publish_bytes_before_configurations();
    let decoded: StatementBody = borsh::from_slice(&bytes).unwrap();
    match &decoded {
        StatementBody::ElectionPublish(election, publication) => {
            assert_eq!(election.0.as_deref(), Some("election"));
            assert_eq!(publication.0, "publication");
        }
        other => panic!("unexpected body {other:?}"),
    }
    assert_eq!(borsh::to_vec(&decoded).unwrap(), bytes);

    let head = StatementHead::from_body(EventIdString("event".to_string()), &decoded);
    assert!(matches!(head.kind, StatementType::ElectionPublish));
    assert_eq!(head.description, "Election published.");
}

#[test]
fn a_stored_election_publish_entry_continues_after_its_body() {
    // A stored entry neither ends with the statement body nor delimits it:
    // the artifact and the search fields follow. A field added to
    // `ElectionPublish` would be read from their bytes in an entry written
    // before it existed, so the configuration is a body of its own.
    let key = strand::signature::StrandSignatureSk::generate().unwrap();
    let data = crate::messages::message::SigningData::new(key.clone(), "sender", key);
    let message = crate::messages::message::Message::election_published_message(
        EventIdString("event".to_string()),
        ElectionIdString(Some("election".to_string())),
        BallotPublicationIdString("publication".to_string()),
        None,
        &data,
        Some("admin-id".to_string()),
        None,
    )
    .unwrap();

    let mut tail = election_publish_bytes_before_configurations();
    // artifact, user_id, username, election_id, area_id, ballot_id
    tail.push(0);
    tail.push(1);
    tail.extend(text("admin-id"));
    tail.push(0);
    tail.push(1);
    tail.extend(text("election"));
    tail.extend([0, 0]);
    let stored = borsh::to_vec(&message).unwrap();
    assert!(stored.ends_with(&tail));

    let decoded: crate::messages::message::Message = borsh::from_slice(&stored).unwrap();
    assert!(matches!(
        decoded.statement.body,
        StatementBody::ElectionPublish(_, _)
    ));
    assert_eq!(decoded.user_id.as_deref(), Some("admin-id"));
    assert_eq!(decoded.election_id.as_deref(), Some("election"));
}

#[test]
fn an_election_publish_of_a_signed_configuration_carries_its_manifest_and_designs() {
    let body = StatementBody::ElectionPublishWithConfiguration(
        ElectionIdString(Some("election".to_string())),
        BallotPublicationIdString("publication".to_string()),
        published_configuration(),
    );
    let mut expected = vec![32, 1];
    expected.extend(text("election"));
    expected.extend(text("publication"));
    expected.extend(text("ov-2028"));
    expected.extend(8_u64.to_le_bytes());
    expected.extend(text(&"ab".repeat(32)));
    expected.extend(2_u32.to_le_bytes());
    for (area, byte) in [("Post 1", "cd"), ("Post 2", "ef")] {
        expected.extend(text(area));
        expected.extend(text("national"));
        expected.extend(text(&byte.repeat(32)));
    }
    assert_eq!(borsh::to_vec(&body).unwrap(), expected);

    match borsh::from_slice::<StatementBody>(&expected).unwrap() {
        StatementBody::ElectionPublishWithConfiguration(election, publication, configuration) => {
            assert_eq!(election.0.as_deref(), Some("election"));
            assert_eq!(publication.0, "publication");
            assert_eq!(configuration, published_configuration());
        }
        other => panic!("unexpected body {other:?}"),
    }
    assert!(borsh::from_slice::<StatementBody>(&[35]).is_err());

    let head = StatementHead::from_body(EventIdString("event".to_string()), &body);
    assert!(matches!(head.kind, StatementType::ElectionPublish));
    assert!(matches!(head.event_type, StatementEventType::SYSTEM));
    assert!(matches!(head.log_type, StatementLogType::INFO));
    assert_eq!(
        head.description,
        format!(
            "Election published as approved: configuration ov-2028 revision 8 (manifest {}, 2 designs).",
            "ab".repeat(32)
        )
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
