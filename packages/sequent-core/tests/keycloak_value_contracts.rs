// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Pure identity configuration contracts, including operator-facing validation
//! errors. Password limits are checked without allocating oversized passwords.

#![cfg(feature = "keycloak")]

use sequent_core::services::keycloak::*;
use sequent_core::types::keycloak::*;
use serde_json::json;

#[test]
fn user_attributes_distinguish_missing_empty_and_multivalued_fields() {
    let mut user = User::default();
    assert_eq!(user.get_mobile_phone(), None);
    assert_eq!(user.get_area_id(), None);
    assert_eq!(user.get_authorized_election_ids(), None);
    assert_eq!(user.get_attribute_val(&"locale".into()), None);
    assert_eq!(user.get_attribute_multival(&"locale".into()), None);
    assert_eq!(user.get_votes_info_by_election_id(), None);
    user.attributes = Some(
        [
            (MOBILE_PHONE_ATTR_NAME.into(), vec!["+15555550100".into()]),
            (AREA_ID_ATTR_NAME.into(), vec!["north".into()]),
            (
                AUTHORIZED_ELECTION_IDS_NAME.into(),
                vec!["mayor".into(), "council".into()],
            ),
            ("locale".into(), vec!["en".into(), "fr".into()]),
            ("empty".into(), vec![]),
        ]
        .into(),
    );
    assert_eq!(user.get_mobile_phone().as_deref(), Some("+15555550100"));
    assert_eq!(user.get_area_id().as_deref(), Some("north"));
    assert_eq!(
        user.get_authorized_election_ids(),
        Some(vec!["mayor".into(), "council".into()])
    );
    assert_eq!(
        user.get_attribute_val(&"locale".into()).as_deref(),
        Some("en")
    );
    assert_eq!(
        user.get_attribute_multival(&"locale".into()).as_deref(),
        Some("en|fr")
    );
    assert_eq!(user.get_attribute_val(&"empty".into()), None);
    assert_eq!(
        user.get_attribute_multival(&"empty".into()).as_deref(),
        Some("")
    );
    user.votes_info = Some(vec![VotesInfo {
        election_id: "mayor".into(),
        num_votes: 2,
        last_voted_at: "2026-10-01T09:00:00Z".into(),
    }]);
    let votes = user.get_votes_info_by_election_id().unwrap();
    assert_eq!(votes["mayor"].num_votes, 2);
    assert!(!votes.contains_key("council"));
}

#[test]
fn password_generation_limits_reject_impossible_policies_before_allocating() {
    use PasswordPolicyGenerationError::*;
    for (policy, expected) in [
        ("", NotConfigured), ("digits(1)", MinimumLengthMissing),
        ("length(0) and digits(1)", MinimumLengthOutOfRange),
        ("length(257) and digits(1)", MinimumLengthOutOfRange),
        ("length(8) and maxLength(257) and digits(1)", MaximumLengthOutOfRange),
        ("length(8) and maxLength(7) and digits(1)", MinimumExceedsMaximum),
        ("length(8)", CharacterClassMissing),
        ("length(1) and maxLength(2) and digits(3)", MaximumTooSmallForRequiredCharacters),
        // This total exceeds i32::MAX. Narrowing it first would wrap and let
        // an impossible allocation pass as a valid 256-character policy.
        ("length(1) and maxLength(256) and digits(2147483647) and upperCase(1)", MaximumTooSmallForRequiredCharacters),
        ("length(1) and digits(257)", MaximumTooSmallForRequiredCharacters),
    ] {
        let parsed = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(policy));
        assert_eq!(parsed.validate_for_generation(), Err(expected), "{policy}");
    }
    let supported = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
        "length(12) and digits(2)",
    ));
    supported.validate_for_generation().unwrap();
    let generated = supported.generate_password().unwrap();
    assert_eq!(generated.len(), 12);
    assert!(generated.bytes().all(|byte| byte.is_ascii_digit()));
}

#[test]
fn password_error_codes_and_messages_identify_the_rule_without_echoing_the_password(
) {
    use PasswordPolicyRule::*;
    for (rule, key, message) in [
        (
            MinimumLength,
            "minimumLength",
            "Password is shorter than the realm policy minimum",
        ),
        (
            MaximumLength,
            "maximumLength",
            "Password is longer than the realm policy maximum",
        ),
        (
            Uppercase,
            "uppercase",
            "Password does not contain enough uppercase characters",
        ),
        (
            Lowercase,
            "lowercase",
            "Password does not contain enough lowercase characters",
        ),
        (Digits, "digits", "Password does not contain enough digits"),
        (
            SpecialCharacters,
            "specialCharacters",
            "Password does not contain enough special characters",
        ),
    ] {
        let error = PasswordPolicyViolation {
            rule,
            required_count: 2,
        };
        assert_eq!(rule.as_str(), key);
        assert_eq!(error.to_string(), message);
    }
    use PasswordPolicyGenerationError::*;
    for (error, message) in [
        (NotConfigured, "Password policy is not configured"),
        (MinimumLengthMissing, "Password policy must include a minimum length for generation"),
        (MinimumLengthOutOfRange, "Minimum password length must be between 1 and 256"),
        (MaximumLengthOutOfRange, "Maximum password length must be between 1 and 256"),
        (MinimumExceedsMaximum, "Minimum password length cannot exceed maximum password length"),
        (CharacterClassMissing, "Password policy must include at least one character class"),
        (MaximumTooSmallForRequiredCharacters, "Maximum password length is too small for the required character classes"),
    ] { assert_eq!(error.to_string(), message); }
}

#[test]
fn managed_policy_changes_preserve_escaped_regex_and_existing_character_requirements(
) {
    let original =
        r#"regexPattern(^a\)b and c$) and length(12) and specialChars(3)"#;
    let policy = RealmPasswordPolicy {
        include_special_characters: true,
        ..Default::default()
    };
    let merged = policy.merge_into_keycloak_policy(Some(original)).unwrap();
    assert!(merged.contains(r#"regexPattern(^a\)b and c$)"#));
    assert!(merged.contains("specialChars(3)"));
}

#[test]
fn formatted_user_profiles_preserve_required_roles_and_scope_selectors() {
    let raw = serde_json::from_value(json!({"attributes": [
        {"name": "${phone}", "displayName": "Phone", "permissions": {"edit": ["admin"]}, "required": {"roles": ["user"], "scopes": ["vote"]},
            "selector": {"scopes": ["vote"]}, "multivalued": true},
        {"name": "locale", "multivalued": false, "permissions": {"edit": ["admin"]}}
    ], "groups": [{"name": "contact", "displayHeader": "Contact"}]})).unwrap();
    let formatted =
        KeycloakAdminClient::get_formatted_user_profile_configuration(raw);
    assert_eq!(formatted.attributes.len(), 2);
    assert_eq!(
        formatted.attributes[0].required.as_ref().unwrap().roles,
        Some(vec!["user".into()])
    );
    assert_eq!(
        formatted.attributes[0].selector.as_ref().unwrap().scopes,
        Some(vec!["vote".into()])
    );
    assert_eq!(KeycloakAdminClient::get_attribute_name(&None), None);
}
