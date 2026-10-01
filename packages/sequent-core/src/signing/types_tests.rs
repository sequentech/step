// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::types::permissions::Permissions;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::str::FromStr;
use strum::IntoEnumIterator;

/// The wire value of an enum, as serde writes it.
fn wire<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value).unwrap() {
        Value::String(text) => text,
        other => panic!("not a string: {other}"),
    }
}

/// serde and strum agree on every value, and both parse it back.
fn assert_wire<T>(cases: &[(T, &str)])
where
    T: Serialize
        + DeserializeOwned
        + FromStr
        + std::fmt::Display
        + PartialEq
        + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    for (value, text) in cases {
        assert_eq!(wire(value), *text);
        assert_eq!(value.to_string(), *text);
        assert_eq!(&T::from_str(text).unwrap(), value);
        assert_eq!(&serde_json::from_value::<T>(json!(text)).unwrap(), value);
    }
}

#[test]
fn the_catalog_has_ten_actions_with_stable_ids() {
    let ids: Vec<String> = SigningAction::iter()
        .map(|action| action.to_string())
        .collect();
    assert_eq!(
        ids,
        [
            "initialize-voting",
            "open-voting",
            "close-voting",
            "generate-election-returns",
            "generate-reports",
            "transmit-results",
            "approve-voter",
            "approve-configuration",
            "key-ceremony",
            "tally-key",
        ]
    );
    for action in SigningAction::iter() {
        assert_eq!(wire(&action), action.to_string());
        assert_eq!(SigningAction::from_str(&action.to_string()), Ok(action));
    }
}

#[test]
fn each_action_is_signed_under_sign_and_its_id() {
    for action in SigningAction::iter() {
        let permission = action.sign_permission();
        assert_eq!(permission.to_string(), format!("sign-{action}"));
        assert_eq!(
            Permissions::from_str(&format!("sign-{action}")),
            Ok(permission)
        );
    }
}

#[test]
fn actions_map_to_their_scope_mode_document_and_group() {
    use DocumentKind as D;
    use ExecutionMode as M;
    use SigningActionGroup as G;
    use SigningScope as S;
    let table = [
        (
            SigningAction::InitializeVoting,
            S::Post,
            M::Deferred,
            D::NoDocument,
            G::Voting,
        ),
        (
            SigningAction::OpenVoting,
            S::Post,
            M::Deferred,
            D::NoDocument,
            G::Voting,
        ),
        (
            SigningAction::CloseVoting,
            S::Post,
            M::Deferred,
            D::NoDocument,
            G::Voting,
        ),
        (
            SigningAction::GenerateElectionReturns,
            S::PostAndCountry,
            M::Deferred,
            D::Pdf,
            G::ResultsAndReports,
        ),
        (
            SigningAction::GenerateReports,
            S::Post,
            M::Deferred,
            D::Pdf,
            G::ResultsAndReports,
        ),
        (
            SigningAction::TransmitResults,
            S::PostAndCountry,
            M::Deferred,
            D::Eml,
            G::ResultsAndReports,
        ),
        (
            SigningAction::ApproveVoter,
            S::Post,
            M::Deferred,
            D::NoDocument,
            G::Enrollment,
        ),
        (
            SigningAction::ApproveConfiguration,
            S::Event,
            M::Deferred,
            D::NoDocument,
            G::ConfigurationAndKeys,
        ),
        (
            SigningAction::ConfirmKeyShare,
            S::Trustee,
            M::Gate,
            D::NoDocument,
            G::ConfigurationAndKeys,
        ),
        (
            SigningAction::ContributeKeyShare,
            S::Trustee,
            M::Gate,
            D::NoDocument,
            G::ConfigurationAndKeys,
        ),
    ];
    assert_eq!(table.len(), SigningAction::iter().count());
    for (action, scope, mode, document, group) in table {
        assert_eq!(action.scope(), scope, "{action}");
        assert_eq!(action.mode(), mode, "{action}");
        assert_eq!(action.document(), document, "{action}");
        assert_eq!(action.group(), group, "{action}");
        assert_eq!(action.is_trustee(), scope == S::Trustee, "{action}");
    }
}

#[test]
fn a_missing_rule_keeps_todays_behaviour() {
    let rule = SigningRule::default_for(SigningAction::GenerateElectionReturns);
    assert_eq!(
        rule,
        SigningRule {
            action: SigningAction::GenerateElectionReturns,
            requirement: SigningRequirement::NotRequired,
            signatures: 1,
            requester_signing: RequesterSigning::NotAllowed,
            expires_minutes: Some(60),
            revision: 0,
        }
    );
    assert!(!rule.is_required());
}

/// Later code reads the count and the requester policy through the
/// accessors, which apply the trustee override; the stored fields stay raw.
#[test]
fn a_trustee_signs_their_own_step_once() {
    let stored = SigningRule {
        action: SigningAction::ContributeKeyShare,
        requirement: SigningRequirement::Required,
        signatures: 3,
        requester_signing: RequesterSigning::NotAllowed,
        expires_minutes: None,
        revision: 7,
    };
    assert_eq!(stored.required(), 1);
    assert_eq!(
        stored.requester_signing_effective(),
        RequesterSigning::Allowed
    );
    let default = SigningRule::default_for(SigningAction::ConfirmKeyShare);
    assert_eq!(default.required(), 1);
    assert_eq!(
        default.requester_signing_effective(),
        RequesterSigning::Allowed
    );

    // A non-trustee rule reads as stored.
    let post_rule = SigningRule {
        action: SigningAction::CloseVoting,
        ..stored
    };
    assert_eq!(post_rule.required(), 3);
    assert_eq!(
        post_rule.requester_signing_effective(),
        RequesterSigning::NotAllowed
    );
}

#[test]
fn a_rule_needs_at_least_one_signature() {
    let valid = SigningRule {
        requirement: SigningRequirement::Required,
        signatures: 2,
        ..SigningRule::default_for(SigningAction::OpenVoting)
    };
    assert_eq!(valid.validate(), Ok(()));
    assert_eq!(
        SigningRule {
            signatures: 0,
            ..valid.clone()
        }
        .validate(),
        Err(SigningRuleError::NoSignatures)
    );
    assert_eq!(
        SigningRule {
            expires_minutes: Some(0),
            ..valid
        }
        .validate(),
        Err(SigningRuleError::ZeroExpiry)
    );
    // A trustee rule needs one signature whatever it stores, but the stored
    // count is still at least 1: the database refuses 0 for every rule.
    let trustee = SigningRule::default_for(SigningAction::ConfirmKeyShare);
    assert_eq!(trustee.validate(), Ok(()));
    assert_eq!(
        SigningRule {
            signatures: 0,
            ..trustee
        }
        .validate(),
        Err(SigningRuleError::NoSignatures)
    );
}

#[test]
fn a_rule_reads_and_writes_its_json() {
    let text = json!({
        "action": "close-voting",
        "requirement": "required",
        "signatures": 2,
        "requester_signing": "not-allowed",
        "expires_minutes": null,
        "revision": 4
    });
    let rule: SigningRule = serde_json::from_value(text.clone()).unwrap();
    assert_eq!(rule.action, SigningAction::CloseVoting);
    assert_eq!(rule.requirement, SigningRequirement::Required);
    assert_eq!(rule.required(), 2);
    // An explicit null is no limit.
    assert_eq!(rule.expires_minutes, None);
    assert_eq!(serde_json::to_value(&rule).unwrap(), text);

    // A stored trustee count reads back raw but never counts.
    let trustee: SigningRule = serde_json::from_value(json!({
        "action": "tally-key",
        "requirement": "required",
        "signatures": 2,
        "requester_signing": "not-allowed",
        "expires_minutes": 30,
        "revision": 1
    }))
    .unwrap();
    assert_eq!(trustee.signatures, 2);
    assert_eq!(trustee.required(), 1);
}

/// Imported rules may leave out the expiry and the revision: an absent
/// expiry is the default hour (an explicit null is no limit) and an absent
/// revision is 0.
#[test]
fn absent_optional_fields_take_their_defaults() {
    let rule: SigningRule = serde_json::from_value(json!({
        "action": "open-voting",
        "requirement": "required",
        "signatures": 2,
        "requester_signing": "allowed"
    }))
    .unwrap();
    assert_eq!(rule.expires_minutes, Some(DEFAULT_EXPIRES_MINUTES));
    assert_eq!(rule.expires_minutes, Some(60));
    assert_eq!(rule.revision, 0);
}

#[test]
fn the_default_checks_match_the_2025_behaviour() {
    assert_eq!(
        SigningChecks::default(),
        SigningChecks {
            revocation_check: RevocationCheck::Check,
            crl_unavailable: CrlUnavailablePolicy::Refuse,
            registration: CertificateRegistration::OnFirstUse,
            post_binding: CertificatePostBinding::OnePost,
            revision: 0,
        }
    );
    assert_eq!(
        serde_json::to_value(SigningChecks::default()).unwrap(),
        json!({
            "revocation_check": "check",
            "crl_unavailable": "refuse",
            "registration": "on-first-use",
            "post_binding": "one-post",
            "revision": 0
        })
    );
}

#[test]
fn policy_enums_use_kebab_case_values() {
    assert_wire(&[
        (SigningRequirement::NotRequired, "not-required"),
        (SigningRequirement::Required, "required"),
    ]);
    assert_wire(&[
        (RequesterSigning::Allowed, "allowed"),
        (RequesterSigning::NotAllowed, "not-allowed"),
    ]);
    assert_wire(&[
        (RevocationCheck::Check, "check"),
        (RevocationCheck::DontCheck, "dont-check"),
    ]);
    assert_wire(&[
        (CrlUnavailablePolicy::Refuse, "refuse"),
        (CrlUnavailablePolicy::AcceptUnchecked, "accept-unchecked"),
    ]);
    assert_wire(&[
        (CertificateRegistration::OnFirstUse, "on-first-use"),
        (
            CertificateRegistration::SecurityOfficerOnly,
            "security-officer-only",
        ),
    ]);
    assert_wire(&[
        (CertificatePostBinding::OnePost, "one-post"),
        (CertificatePostBinding::AnyPost, "any-post"),
    ]);
}

#[test]
fn request_and_certificate_enums_use_kebab_case_values() {
    assert_wire(&[
        (SigningRequestStatus::Waiting, "waiting"),
        (SigningRequestStatus::Completed, "completed"),
        (SigningRequestStatus::Executed, "executed"),
        (SigningRequestStatus::Cancelled, "cancelled"),
        (SigningRequestStatus::Expired, "expired"),
        (SigningRequestStatus::Failed, "failed"),
    ]);
    assert_wire(&[
        (CancelReason::ByRequester, "by-requester"),
        (CancelReason::ByOperator, "by-operator"),
        (CancelReason::RuleChanged, "rule-changed"),
        (CancelReason::PayloadChanged, "payload-changed"),
        (CancelReason::Superseded, "superseded"),
        (CancelReason::CertificateRevoked, "certificate-revoked"),
    ]);
    assert_wire(&[
        (StaffCertificateStatus::Active, "active"),
        (StaffCertificateStatus::Revoked, "revoked"),
    ]);
    assert_wire(&[
        (CertificateAuthorityPurpose::VoterSignIn, "voter-sign-in"),
        (
            CertificateAuthorityPurpose::StaffSignatures,
            "staff-signatures",
        ),
    ]);
    assert_eq!(
        CertificateAuthorityPurpose::default(),
        CertificateAuthorityPurpose::VoterSignIn
    );
    assert_wire(&[
        (SignatureAlgorithm::RsaPkcs1Sha256, "rsa-pkcs1-sha256"),
        (SignatureAlgorithm::EcdsaP256Sha256, "ecdsa-p256-sha256"),
    ]);
    assert_wire(&[
        (StaffCertificateRegistration::FirstUse, "first-use"),
        (
            StaffCertificateRegistration::SecurityOfficer,
            "security-officer",
        ),
    ]);
    assert_wire(&[
        (RevocationStatus::Checked, "checked"),
        (RevocationStatus::Unchecked, "unchecked"),
    ]);
    assert_wire(&[
        (DocumentRevisionState::Base, "base"),
        (DocumentRevisionState::Prepared, "prepared"),
        (DocumentRevisionState::Signed, "signed"),
    ]);
    assert_wire(&[
        (CrlStatus::Ok, "ok"),
        (CrlStatus::Unavailable, "unavailable"),
    ]);
    assert_wire(&[
        (CertificateOpenFailure::WrongPassword, "wrong-password"),
        (CertificateOpenFailure::Unreadable, "unreadable"),
        (CertificateOpenFailure::NoKey, "no-key"),
    ]);
    assert_wire(&[
        (DocumentKind::NoDocument, "no-document"),
        (DocumentKind::Pdf, "pdf"),
        (DocumentKind::Eml, "eml"),
    ]);
    assert_wire(&[
        (SigningScope::Post, "post"),
        (SigningScope::PostAndCountry, "post-and-country"),
        (SigningScope::Event, "event"),
        (SigningScope::Trustee, "trustee"),
    ]);
}

#[test]
fn certificate_check_ids_match_the_dialog_check_list() {
    let ids: Vec<String> = CertificateCheckId::iter()
        .map(|id| {
            assert_eq!(wire(&id), id.to_string());
            id.to_string()
        })
        .collect();
    assert_eq!(
        ids,
        [
            "trusted-issuer",
            "valid-now",
            "signing-key-usage",
            "not-revoked",
            "registered",
            "registered-to-other",
            "already-signed",
            "post-binding",
            "signature",
        ]
    );
}

#[test]
fn subjects_serialize_to_the_documented_shapes() {
    assert_eq!(
        serde_json::to_value(InitializeVotingSubject {
            publication_id: None
        })
        .unwrap(),
        json!({"publication_id": null})
    );
    assert_eq!(
        serde_json::to_value(CloseVotingSubject::new(vec![
            "online".into(),
            "kiosk".into()
        ]))
        .unwrap(),
        json!({"channels": ["kiosk", "online"]})
    );
    assert_eq!(
        serde_json::to_value(ApproveVoterSubject {
            application_id: "a".into(),
            applicant_registry_id: "r".into(),
            decision: ApplicationDecision::Approve,
        })
        .unwrap(),
        json!({"application_id": "a", "applicant_registry_id": "r", "decision": "approve"})
    );
    assert_eq!(
        serde_json::to_value(TransmitResultsSubject::new(
            "t".into(),
            "p".into(),
            "e".into(),
            vec!["s1".into()],
        ))
        .unwrap(),
        json!({"tally_session_id": "t", "package_sha256": "p", "eml_sha256": "e", "destinations": ["s1"]})
    );
    assert_eq!(
        serde_json::to_value(KeyCeremonySubject {
            keys_ceremony_id: "k".into(),
            trustee_id: "t".into(),
            key_share_sha256: "h".into(),
        })
        .unwrap(),
        json!({"keys_ceremony_id": "k", "trustee_id": "t", "key_share_sha256": "h"})
    );
}

#[test]
fn rule_errors_explain_themselves() {
    assert_eq!(
        SigningRuleError::NoSignatures.to_string(),
        "a rule needs at least one signature"
    );
    assert_eq!(
        SigningRuleError::ZeroExpiry.to_string(),
        "a request expiry must be at least one minute"
    );
}

/// Equal subjects canonicalize equally: list fields are sets, sorted and
/// deduplicated however they were built or read.
#[test]
fn list_fields_of_subjects_are_sorted() {
    let built = CloseVotingSubject::new(vec![
        "online".into(),
        "kiosk".into(),
        "online".into(),
    ]);
    assert_eq!(built.channels(), ["kiosk", "online"]);
    let read: CloseVotingSubject =
        serde_json::from_value(json!({"channels": ["online", "kiosk"]}))
            .unwrap();
    assert_eq!(read, built);

    let transmit = TransmitResultsSubject::new(
        "t".into(),
        "p".into(),
        "e".into(),
        vec!["s2".into(), "s1".into()],
    );
    assert_eq!(transmit.destinations(), ["s1", "s2"]);
    let read: TransmitResultsSubject = serde_json::from_value(json!({
        "tally_session_id": "t",
        "package_sha256": "p",
        "eml_sha256": "e",
        "destinations": ["s2", "s1", "s2"]
    }))
    .unwrap();
    assert_eq!(read, transmit);
}
