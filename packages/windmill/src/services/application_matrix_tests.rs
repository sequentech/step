// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::services::approval_matrix::evaluate::MatrixSource;
use crate::services::approval_matrix::ApprovalMatrix;
use serde_json::json;

const SEARCH: &str = "firstName,middleName,lastName,dateOfBirth,embassy";

fn annotations(identity: Option<&str>) -> ApplicationAnnotations {
    serde_json::from_value(json!({
        "session_id": null,
        "credentials": null,
        "verified_by": null,
        "rejection_reason": null,
        "rejection_message": null,
        "search-attributes": SEARCH,
        "unset-attributes": "email",
        "update-attributes": "email",
        "identity-method": identity,
        "mismatches": null,
        "fields_match": null,
        "manual_verify_reason": null,
    }))
    .unwrap()
}

fn applicant(card_type: Option<&str>) -> HashMap<String, String> {
    let mut data: HashMap<String, String> = [
        ("firstName", "Juan Carlos"),
        ("middleName", "Santos"),
        ("lastName", "Dela Cruz"),
        ("dateOfBirth", "1990-01-01"),
        ("embassy", "Tokyo PE"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();
    if let Some(card_type) = card_type {
        data.insert(ID_CARD_TYPE_FIELD.to_string(), card_type.to_string());
    }
    data
}

fn registry_voter(id: &str, changes: &[(&str, &str)]) -> User {
    let mut values = applicant(None);
    for (key, value) in changes {
        values.insert(key.to_string(), value.to_string());
    }
    let attributes = ["middleName", "dateOfBirth", "embassy"]
        .iter()
        .filter_map(|key| Some((key.to_string(), vec![values.get(*key)?.clone()])))
        .collect();
    User {
        id: Some(id.to_string()),
        email: values.get("email").cloned(),
        username: Some(format!("voter-{id}")),
        first_name: values.get("firstName").cloned(),
        last_name: values.get("lastName").cloned(),
        attributes: Some(attributes),
        ..Default::default()
    }
}

fn built_in() -> MatrixVersion {
    MatrixVersion::built_in(SEARCH.split(',').map(str::to_string).collect())
}

fn verify(
    users: Vec<User>,
    matrix: &MatrixVersion,
    identity: Option<&str>,
    card_type: Option<&str>,
) -> (ApplicationVerificationResult, DecisionRecord) {
    automatic_verification(users, matrix, &annotations(identity), &applicant(card_type)).unwrap()
}

#[test]
fn a_voter_whose_fields_match_is_accepted() {
    let (result, decision) = verify(
        vec![registry_voter("a", &[("firstName", "JUAN-CARLOS")])],
        &built_in(),
        None,
        Some("philippinePassport"),
    );

    assert_eq!(result.application_status, ApplicationStatus::ACCEPTED);
    assert_eq!(result.application_type, ApplicationType::AUTOMATIC);
    assert_eq!(result.user_id, Some("a".to_string()));
    assert_eq!(result.username, "voter-a");
    assert_eq!(result.mismatches, Some(0));
    assert_eq!(result.rejection_reason, None);
    assert_eq!(result.manual_verify_reason, None);
    assert_eq!(decision.rule, Some(3));
    assert_eq!(decision.matrix_version, 1);
    assert_eq!(decision.matrix_source, MatrixSource::BUILT_IN);
    assert_eq!(
        decision.inputs.valid_id.as_deref(),
        Some("philippinePassport")
    );
}

#[test]
fn one_mismatch_goes_to_manual_review_with_its_field() {
    let (result, decision) = verify(
        vec![registry_voter("a", &[("dateOfBirth", "1991-01-01")])],
        &built_in(),
        None,
        None,
    );

    assert_eq!(result.application_status, ApplicationStatus::PENDING);
    assert_eq!(result.application_type, ApplicationType::MANUAL);
    assert_eq!(result.user_id, None);
    assert_eq!(result.username, "");
    assert_eq!(result.mismatches, Some(1));
    assert_eq!(
        result.rejection_reason,
        Some(ApplicationRejectReason::NO_VOTER)
    );
    assert_eq!(
        result.manual_verify_reason.as_deref(),
        Some("Mismatch at Date Of Birth")
    );
    assert_eq!(
        result.fields_match.unwrap().get("dateOfBirth"),
        Some(&false)
    );
    assert_eq!(decision.rule, Some(5));
}

#[test]
fn a_voter_who_already_enrolled_is_rejected_and_named() {
    let (result, decision) = verify(
        vec![registry_voter("a", &[("email", "juan@example.com")])],
        &built_in(),
        None,
        None,
    );

    assert_eq!(result.application_status, ApplicationStatus::REJECTED);
    assert_eq!(result.application_type, ApplicationType::AUTOMATIC);
    assert_eq!(result.user_id, Some("a".to_string()));
    assert_eq!(
        result.rejection_reason,
        Some(ApplicationRejectReason::ALREADY_APPROVED)
    );
    assert_eq!(result.attributes_unset.unwrap().get("email"), Some(&false));
    assert_eq!(decision.rule, Some(1));
    assert!(decision.inputs.already_enrolled);
}

#[test]
fn no_registry_voter_is_rejected_with_every_field_differing() {
    let (result, decision) = verify(vec![], &built_in(), None, None);

    assert_eq!(result.application_status, ApplicationStatus::REJECTED);
    assert_eq!(
        result.rejection_reason,
        Some(ApplicationRejectReason::NO_VOTER)
    );
    assert_eq!(result.mismatches, Some(5));
    let fields_match = result.fields_match.unwrap();
    assert_eq!(fields_match.len(), 5);
    assert!(fields_match.values().all(|is_match| !is_match));
    assert_eq!(result.attributes_unset, None);
    assert_eq!(result.manual_verify_reason, None);
    assert_eq!(decision.rule, None);
    assert!(!decision.inputs.voter_found);
}

#[test]
fn a_manual_entry_goes_to_manual_review_even_when_everything_matches() {
    let (result, decision) = verify(
        vec![registry_voter("a", &[])],
        &built_in(),
        Some("MANUAL_ENTRY"),
        None,
    );

    assert_eq!(result.application_status, ApplicationStatus::PENDING);
    assert_eq!(
        result.rejection_reason,
        Some(ApplicationRejectReason::IDENTITY_NOT_VERIFIED)
    );
    assert_eq!(result.user_id, None);
    assert_eq!(decision.rule, Some(2));
    assert_eq!(decision.inputs.identity, Some(IdentityMethod::MANUAL_ENTRY));
}

#[test]
fn first_and_middle_name_are_one_field_for_a_drivers_license() {
    // The registry holds the two given names in the first name.
    let voter = registry_voter(
        "a",
        &[("firstName", "Juan Carlos Santos"), ("middleName", "")],
    );

    let (result, decision) = verify(
        vec![voter.clone()],
        &built_in(),
        None,
        Some("driversLicense"),
    );
    assert_eq!(result.application_status, ApplicationStatus::ACCEPTED);
    assert_eq!(
        result.fields_match.unwrap().get("firstName.middleName"),
        Some(&true)
    );
    assert_eq!(
        decision.inputs.fields.get("firstName"),
        Some(&FieldMatch::MATCHES)
    );
    assert_eq!(decision.inputs.fields.get("middleName"), None);

    let (result, _) = verify(vec![voter], &built_in(), None, Some("philippinePassport"));
    assert_eq!(result.application_status, ApplicationStatus::REJECTED);
    assert_eq!(result.mismatches, Some(2));
}

#[test]
fn several_matching_voters_go_to_manual_review() {
    let (result, decision) = verify(
        vec![registry_voter("b", &[]), registry_voter("a", &[])],
        &built_in(),
        None,
        None,
    );

    assert_eq!(result.application_status, ApplicationStatus::PENDING);
    assert_eq!(result.user_id, None);
    assert_eq!(
        result.rejection_reason,
        Some(ApplicationRejectReason::OTHER)
    );
    assert_eq!(
        result.manual_verify_reason.as_deref(),
        Some(SEVERAL_VOTERS_MATCH_REASON)
    );
    assert_eq!(decision.accepted_candidates, 2);
}

#[test]
fn a_saved_matrix_compares_its_own_fields() {
    let matrix: ApprovalMatrix = serde_json::from_value(json!({
        "compared_fields": ["firstName", "lastName", "dateOfBirth"],
        "rules": [
            {"when": {"already_enrolled": true}, "then": {"decision": "REJECTED", "reason": "ALREADY_APPROVED"}},
            {"when": {"identity": "MANUAL_ENTRY"}, "then": {"decision": "PENDING", "reason": "IDENTITY_NOT_VERIFIED"}},
            {"when": {"differing": "none"}, "then": {"decision": "ACCEPTED"}}
        ],
        "otherwise": {"decision": "PENDING", "reason": "NO_VOTER"}
    }))
    .unwrap();
    let association = MatrixVersion {
        version: 2,
        source: MatrixSource::SAVED,
        matrix,
    };

    // The Post and the middle name differ, but this matrix doesn't compare
    // them.
    let voter = registry_voter("a", &[("embassy", "Osaka PCG"), ("middleName", "Reyes")]);
    let (result, decision) = verify(vec![voter], &association, None, None);
    assert_eq!(result.application_status, ApplicationStatus::ACCEPTED);
    assert_eq!(result.mismatches, Some(0));
    assert_eq!(decision.matrix_version, 2);
    assert_eq!(decision.matrix_source, MatrixSource::SAVED);
    assert_eq!(decision.rule, Some(3));
    assert_eq!(decision.inputs.fields.len(), 3);

    let voter = registry_voter("a", &[("lastName", "Santos")]);
    let (result, decision) = verify(vec![voter], &association, None, None);
    assert_eq!(result.application_status, ApplicationStatus::PENDING);
    assert_eq!(decision.rule, None);
}

#[test]
fn the_lookup_searches_the_compared_fields() {
    let filter = get_filter_from_applicant_data(
        "tenant".to_string(),
        Some("event".to_string()),
        None,
        None,
        "realm".to_string(),
        None,
        &[
            "firstName".to_string(),
            "dateOfBirth".to_string(),
            "embassy".to_string(),
        ],
        &applicant(None),
    )
    .unwrap();

    assert!(filter.first_name.is_some());
    assert!(filter.last_name.is_none());
    assert_eq!(
        filter.attributes,
        Some(HashMap::from([(
            "dateOfBirth".to_string(),
            "1990-01-01".to_string()
        )]))
    );
}

#[test]
fn the_decision_is_stored_with_the_application_annotations() {
    let (_, decision) = verify(vec![registry_voter("a", &[])], &built_in(), None, None);
    let mut stored = annotations(None);
    stored.decision = Some(decision);

    let value = serde_json::to_value(&stored).unwrap();

    assert_eq!(value["decision"]["matrix_version"], 1);
    assert_eq!(value["decision"]["rule"], 3);
    assert_eq!(value["decision"]["decision"], "ACCEPTED");
    assert_eq!(value["identity-method"], serde_json::Value::Null);
    let read: ApplicationAnnotations = serde_json::from_value(value).unwrap();
    assert_eq!(read.decision, stored.decision);
}

#[test]
fn annotations_without_a_decision_are_still_read() {
    let stored: ApplicationAnnotations = serde_json::from_value(json!({
        "search-attributes": SEARCH,
        "unset-attributes": "email",
    }))
    .unwrap();

    assert_eq!(stored.decision, None);
    assert_eq!(stored.identity_method, None);
    assert_eq!(
        search_attributes_list(&stored).unwrap(),
        SEARCH.split(',').collect::<Vec<_>>()
    );
}

#[test]
fn the_known_identity_documents_are_listed() {
    assert_eq!(
        id_card_types(),
        vec![
            "philippinePassport",
            "philSysID",
            "seamanBook",
            "driversLicense",
            "iBP"
        ]
    );
}
