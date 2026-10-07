// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Persisted string wrappers must retain their Borsh length prefix and UTF-8
//! payload. Expected bytes are assembled independently of the serializer.

use electoral_log::messages::newtypes::*;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};

macro_rules! string_wire_cases {
    ($($name:ident => $wrapper:ident),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                for text in ["", "élection-\u{1f5f3}", "a\0b"] {
                    let value = $wrapper(text.to_owned());
                    let mut expected = (text.len() as u32).to_le_bytes().to_vec();
                    expected.extend_from_slice(text.as_bytes());
                    assert_eq!(borsh::to_vec(&value).unwrap(), expected);
                    let decoded: $wrapper = borsh::from_slice(&expected).unwrap();
                    assert_eq!(decoded.0, text);

                    // Appended or truncated data cannot be accepted as a
                    // different signed identifier through prefix decoding.
                    let mut trailing = expected.clone();
                    trailing.push(0);
                    assert!(borsh::from_slice::<$wrapper>(&trailing).is_err());
                    assert!(borsh::from_slice::<$wrapper>(&expected[..expected.len() - 1]).is_err());
                    assert_eq!(serde_json::to_value(&value).unwrap(), serde_json::json!(text));
                    assert_eq!(serde_json::from_value::<$wrapper>(serde_json::json!(text)).unwrap().0, text);
                }
            }
        )+
    };
}

string_wire_cases! {
    event_identifier => EventIdString,
    contest_identifier => ContestIdString,
    tenant_identifier => TenantIdString,
    administrator_identifier => AdminUserIdString,
    trustee_name => TrusteeNameString,
    ballot_publication_identifier => BallotPublicationIdString,
    public_key_encoding => PublicKeyDerB64,
    voter_address => VoterIpString,
    voter_country => VoterCountryString,
    voting_channel => VotingChannelString,
    cast_vote_error => CastVoteErrorString,
    identity_error => ErrorMessageString,
    identity_event_type => KeycloakEventTypeString,
    reconciliation_sequence => ExternalReconciliationSequenceString,
    reconciliation_timestamp => ExternalReconciliationGeneratedAtString,
    reconciliation_input_hash => ExternalReconciliationInputHashString,
    phone_number => PhoneE164String,
    results_publication_identifier => ResultsPublicationIdString,
    results_route_scope => ResultsPublicationRouteScopeString,
    results_access => ResultsPublicationAccessString,
    results_visibility_scope => ResultsPublicationVisibilityScopeString,
    monitoring_config_kind => MonitoringConfigKindString,
    monitoring_config_key => MonitoringConfigKeyString,
    monitoring_config_digest => MonitoringConfigDigestString,
    monitoring_preset_identifier => MonitoringPresetIdString,
    approval_matrix_digest => ApprovalMatrixDigestString,
}

/// The version is signed as a fixed-width little-endian number.
#[test]
fn an_approval_matrix_version_is_four_little_endian_bytes() {
    for number in [0, 2, u32::MAX] {
        let value = ApprovalMatrixVersion(number);
        let expected = number.to_le_bytes().to_vec();
        assert_eq!(borsh::to_vec(&value).unwrap(), expected);
        let decoded: ApprovalMatrixVersion = borsh::from_slice(&expected).unwrap();
        assert_eq!(decoded, value);

        let mut trailing = expected.clone();
        trailing.push(0);
        assert!(borsh::from_slice::<ApprovalMatrixVersion>(&trailing).is_err());
        assert!(borsh::from_slice::<ApprovalMatrixVersion>(&expected[..3]).is_err());
        assert_eq!(
            serde_json::to_value(&value).unwrap(),
            serde_json::json!(number)
        );
        assert_eq!(
            serde_json::from_value::<ApprovalMatrixVersion>(serde_json::json!(number)).unwrap(),
            value
        );
    }
}

#[test]
fn absent_election_and_present_empty_identifier_have_different_wire_encodings() {
    assert_eq!(borsh::to_vec(&ElectionIdString(None)).unwrap(), vec![0]);
    assert_eq!(
        borsh::to_vec(&ElectionIdString(Some(String::new()))).unwrap(),
        vec![1, 0, 0, 0, 0]
    );
    assert!(borsh::from_slice::<ElectionIdString>(&[2]).is_err());
}

macro_rules! action_wire_cases {
    ($($name:ident => $type:ident { $($variant:ident = $tag:literal),+ }),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                $(
                    let value = $type::$variant;
                    assert_eq!(borsh::to_vec(&value).unwrap(), vec![$tag]);
                    let decoded: $type = borsh::from_slice(&[$tag]).unwrap();
                    assert_eq!(decoded, value);
                    assert_eq!(value.to_string(), stringify!($variant));
                    assert_eq!(serde_json::to_value(&value).unwrap(), serde_json::json!(stringify!($variant)));
                )+
                assert!(borsh::from_slice::<$type>(&[255]).is_err());
            }
        )+
    };
}

// Numeric tags are persisted and signed. Reordering an enum must fail these
// golden cases even if serialization and deserialization change together.
action_wire_cases! {
    certificate_actions => CertificateAuthEventAction {Import = 0, Delete = 1},
    request_directions => ExtApiRequestDirection {Inbound = 0, Outbound = 1},
    api_names => ExtApiName {Datafix = 0, Other = 1},
    reconciliation_actions => ExternalReconciliationKind {PatchGenerated = 0, ChangesApplied = 1},
    blacklist_actions => PhoneBlacklistAction {CreateEntry = 0, DeleteEntry = 1},
    results_actions => ResultsPublicationAction {Publish = 0, Revoke = 1},
    monitoring_config_actions => MonitoringConfigChangeAction {Upsert = 0, Delete = 1},
    monitoring_config_origins => MonitoringConfigOrigin {Editor = 0, Preset = 1},
    monitoring_dashboard_modes => MonitoringDashboardMode {Legacy = 0, Configured = 1},
    // Inside the signed Signing body since it carries its own head fields.
    statement_event_types => StatementEventType {USER = 0, SYSTEM = 1},
    statement_log_types => StatementLogType {INFO = 0, ERROR = 1},
    signing_statement_kinds => SigningStatementKind {
        SigningRequestCreated = 0,
        SigningCertificateOpenFailed = 1,
        SigningRequestSigned = 2,
        SigningSignatureRefused = 3,
        SigningCertificateRegistered = 4,
        SigningHandover = 5,
        SigningRequestCancelled = 6,
        SigningRequestExpired = 7,
        SigningRequestCompleted = 8,
        SigningActionExecuted = 9,
        SigningRuleChanged = 10,
        SigningPermissionChanged = 11,
        SigningIssuerChanged = 12,
        SigningChecksChanged = 13,
        SigningCertificateRevoked = 14,
        SigningRequestsExported = 15,
        LifecycleWindowChanged = 16,
        ScheduleRecomputeApplied = 17,
        ScheduleImported = 18,
        ScheduledOutcomeChanged = 19,
        ElectionInitialized = 20,
        LockdownChanged = 21
    },
}

/// Field order is signed: every field of the audit entry, in order, as
/// independently assembled bytes.
#[test]
fn a_monitoring_config_change_is_encoded_field_by_field() {
    let text = |value: &str| {
        let mut bytes = (value.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes
    };
    let details = MonitoringConfigChangeDetails {
        origin: MonitoringConfigOrigin::Preset,
        preset: Some(MonitoringPresetRef {
            id: MonitoringPresetIdString("generic".into()),
            version: 3,
        }),
        mode: MonitoringDashboardMode::Configured,
        generation: 7,
        revisions: vec![
            MonitoringConfigRevisionRef {
                kind: MonitoringConfigKindString("widget".into()),
                key: MonitoringConfigKeyString("turnout".into()),
                revision: 2,
                action: MonitoringConfigChangeAction::Upsert,
                digest: Some(MonitoringConfigDigestString("ab".into())),
            },
            MonitoringConfigRevisionRef {
                kind: MonitoringConfigKindString("theme".into()),
                key: MonitoringConfigKeyString("dark".into()),
                revision: 5,
                action: MonitoringConfigChangeAction::Delete,
                digest: None,
            },
        ],
    };
    let mut expected = vec![1, 1];
    expected.extend(text("generic"));
    expected.extend(3_u32.to_le_bytes());
    expected.push(1);
    expected.extend(7_u64.to_le_bytes());
    expected.extend(2_u32.to_le_bytes());
    expected.extend(text("widget"));
    expected.extend(text("turnout"));
    expected.extend(2_u32.to_le_bytes());
    expected.extend([0, 1]);
    expected.extend(text("ab"));
    expected.extend(text("theme"));
    expected.extend(text("dark"));
    expected.extend(5_u32.to_le_bytes());
    expected.extend([1, 0]);
    assert_eq!(borsh::to_vec(&details).unwrap(), expected);
    assert_eq!(
        borsh::from_slice::<MonitoringConfigChangeDetails>(&expected).unwrap(),
        details
    );
}

/// Field order is signed: every field of a signing entry, in order, as
/// independently assembled bytes.
#[test]
fn a_signing_entry_is_encoded_field_by_field() {
    let text = |value: &str| {
        let mut bytes = (value.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes
    };
    let entry = SigningLogEntry {
        kind: SigningStatementKind::SigningSignatureRefused,
        event_type: StatementEventType::SYSTEM,
        log_type: StatementLogType::ERROR,
        description: "Refused on 7F3A-91C2".into(),
        details_json: r#"{"check":"trusted-issuer"}"#.into(),
        step_id: "2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87".into(),
    };
    let mut expected = vec![3, 1, 1];
    expected.extend(text("Refused on 7F3A-91C2"));
    expected.extend(text(r#"{"check":"trusted-issuer"}"#));
    expected.extend(text("2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87"));
    assert_eq!(borsh::to_vec(&entry).unwrap(), expected);
    assert_eq!(
        borsh::from_slice::<SigningLogEntry>(&expected).unwrap(),
        entry
    );
    let mut trailing = expected.clone();
    trailing.push(0);
    assert!(borsh::from_slice::<SigningLogEntry>(&trailing).is_err());
    assert!(borsh::from_slice::<SigningLogEntry>(&expected[..expected.len() - 1]).is_err());
}

#[test]
fn empty_lists_and_absent_lists_remain_distinct() {
    assert_eq!(borsh::to_vec(&ElectionsIdsString(None)).unwrap(), vec![0]);
    assert_eq!(
        borsh::to_vec(&ElectionsIdsString(Some(vec![]))).unwrap(),
        vec![1, 0, 0, 0, 0]
    );
    assert_eq!(
        borsh::to_vec(&CertificateSubjectDnsString(vec![])).unwrap(),
        vec![0, 0, 0, 0]
    );
    assert_eq!(
        borsh::to_vec(&ResolutionIdsString(vec![])).unwrap(),
        vec![0, 0, 0, 0]
    );
    assert_eq!(
        borsh::to_vec(&ExternalReconciliationOutputHashString(None)).unwrap(),
        vec![0]
    );
    assert_eq!(
        borsh::to_vec(&ExternalApiSubject {
            user_id: None,
            username: None
        })
        .unwrap(),
        vec![0, 0]
    );
}
