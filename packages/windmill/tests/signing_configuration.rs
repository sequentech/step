// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing configuration of an election event bundle, without a
//! database: the `miru:area-threshold` → transmit-results rule mapping, the
//! transmission threshold, and the presets and fixtures that carry rules (the
//! janitor's client template and the student-council organization), each
//! checked against its own file so a hardcoded value fails one of them.

use sequent_core::signing::{
    RequesterSigning, SigningAction, SigningChecks, SigningRequirement, SigningRule,
};
use sequent_core::types::hasura::core::Area;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::str::FromStr;
use strum::IntoEnumIterator;
use windmill::services::consolidation::eml_generator::parse_area_threshold;
use windmill::services::consolidation::send_transmission_package_service::{
    check_transmission_signatures, TransmissionSignaturesShort,
};
use windmill::services::electoral_log::ElectoralLogAdminContext;
use windmill::services::signing::configuration::{
    area_threshold, importer_actor, map_area_thresholds, posts_short_of_sbeis,
    transmission_threshold, AreaThresholdMapping, SYSTEM_ACTOR_ID,
};
use windmill::types::miru_plugin::MiruTransmissionPackageData;

/// The janitor's client template: the preset rules and checks, and titles.
const JANITOR_PRESET: &str = include_str!("../external-bin/janitor/templates/COMELEC/signing.json");
const JANITOR_CLIENT_TENANT: &str =
    include_str!("../external-bin/janitor/templates/COMELEC/tenant.json");
const JANITOR_TENANT: &str =
    include_str!("../external-bin/janitor/templates/tenantConfigurations.hbs");
/// The second organization's fixture.
const STUDENT_COUNCIL: &str =
    include_str!("../../../scripts/dev/scenario/signing-organizations/student-council.json");

#[derive(Deserialize)]
struct Preset {
    signing_rules: Vec<SigningRule>,
    signing_checks: SigningChecks,
}

fn area(name: &str, annotations: Value) -> Area {
    serde_json::from_value(json!({
        "id": format!("8a0c8f5e-5b8e-4f62-9d35-{:012x}", name.len()),
        "tenant_id": "8a0c8f5e-5b8e-4f62-9d35-7c3f1f7a0002",
        "election_event_id": "8a0c8f5e-5b8e-4f62-9d35-7c3f1f7a0003",
        "name": name,
        "annotations": annotations,
    }))
    .unwrap()
}

fn with_threshold(name: &str, threshold: &str) -> Area {
    area(
        name,
        json!({"miru:area-threshold": threshold, "miru:area-station-id": "1"}),
    )
}

fn rule(action: SigningAction, requirement: SigningRequirement, signatures: u16) -> SigningRule {
    SigningRule {
        requirement,
        signatures,
        ..SigningRule::default_for(action)
    }
}

// -- miru:area-threshold ------------------------------------------------------

/// No transmit rule and every Post asking for the same count: the rule asks
/// for it, started and signed as in 2025 (the requester may sign, nothing
/// expires). Posts that ask for nothing don't count.
#[test]
fn equal_thresholds_without_a_transmit_rule_become_one() {
    let areas = [
        with_threshold("Madrid PE", "3"),
        with_threshold("Dili PE", "0"),
        with_threshold("Osaka PCG", "3"),
    ];
    let AreaThresholdMapping::Rule(mapped) = map_area_thresholds(None, &areas) else {
        panic!("equal thresholds map to a rule");
    };
    assert_eq!(mapped.action, SigningAction::TransmitResults);
    assert_eq!(mapped.requirement, SigningRequirement::Required);
    assert_eq!(mapped.signatures, 3);
    assert_eq!(mapped.requester_signing, RequesterSigning::Allowed);
    assert_eq!(mapped.expires_minutes, None);
    assert_eq!(mapped.validate(), Ok(()));
}

/// Posts asking for different counts keep their own: no event-wide rule,
/// and the Posts are named for the warning.
#[test]
fn differing_thresholds_keep_each_posts_own() {
    let areas = [
        with_threshold("Madrid PE", "2"),
        with_threshold("Dili PE", "3"),
        with_threshold("Osaka PCG", "0"),
    ];
    assert_eq!(
        map_area_thresholds(None, &areas),
        AreaThresholdMapping::Differing(vec![
            ("Madrid PE".to_owned(), 2),
            ("Dili PE".to_owned(), 3)
        ])
    );
}

/// A threshold above the most signatures a rule may need maps nothing.
#[test]
fn an_absurd_threshold_maps_nothing() {
    let areas = [
        with_threshold("Madrid PE", "101"),
        with_threshold("Dili PE", "101"),
    ];
    assert_eq!(
        map_area_thresholds(None, &areas),
        AreaThresholdMapping::TooLarge(vec![
            ("Madrid PE".to_owned(), 101),
            ("Dili PE".to_owned(), 101)
        ])
    );
    let AreaThresholdMapping::Rule(rule) =
        map_area_thresholds(None, &[with_threshold("Madrid PE", "100")])
    else {
        panic!("100 is the most a rule may need");
    };
    assert_eq!(rule.signatures, 100);
}

/// Rules for other actions don't stop the mapping.
#[test]
fn other_rules_leave_the_mapping_to_the_thresholds() {
    let rules = [rule(
        SigningAction::OpenVoting,
        SigningRequirement::Required,
        2,
    )];
    let mapped = map_area_thresholds(Some(&rules), &[with_threshold("Madrid PE", "4")]);
    assert!(matches!(mapped, AreaThresholdMapping::Rule(rule) if rule.signatures == 4));
}

/// An imported transmit rule wins, even one that switches signing off.
#[test]
fn an_explicit_transmit_rule_wins_over_the_thresholds() {
    for requirement in [
        SigningRequirement::Required,
        SigningRequirement::NotRequired,
    ] {
        let rules = [rule(SigningAction::TransmitResults, requirement, 1)];
        assert_eq!(
            map_area_thresholds(Some(&rules), &[with_threshold("Madrid PE", "3")]),
            AreaThresholdMapping::Nothing
        );
    }
}

/// Zero, missing, negative and unreadable thresholds ask for nothing, so a
/// 2025 bundle still imports as it did.
#[test]
fn thresholds_that_ask_for_nothing_map_to_no_rule() {
    let areas = [
        with_threshold("a", "0"),
        with_threshold("b", "-1"),
        with_threshold("c", "three"),
        area("d", json!({"miru:area-station-id": "1"})),
        area("e", Value::Null),
    ];
    assert_eq!(
        map_area_thresholds(None, &areas),
        AreaThresholdMapping::Nothing
    );
    assert_eq!(
        map_area_thresholds(Some(&[]), &[]),
        AreaThresholdMapping::Nothing
    );
}

/// Read exactly as the transmission package reads it.
#[test]
fn a_threshold_reads_as_the_transmission_package_reads_it() {
    assert_eq!(area_threshold(&with_threshold("a", "2")), Some(2));
    for unreadable in [" 2 ", "2.5", ""] {
        assert_eq!(area_threshold(&with_threshold("a", unreadable)), None);
        assert!(parse_area_threshold(unreadable).is_err());
    }
    // Annotations are text; a number is not a threshold the package reads.
    assert_eq!(
        area_threshold(&area("a", json!({"miru:area-threshold": 5}))),
        None
    );
    assert_eq!(
        area_threshold(&area("a", json!({"area-threshold": "2"}))),
        None
    );
}

/// Posts whose SBEIs are fewer than the count, for the import's warning.
#[test]
fn posts_with_fewer_sbeis_than_the_count_are_named() {
    let areas = [
        area(
            "Madrid PE",
            json!({"miru:area-trustee-users": r#"["1","2","3"]"#}),
        ),
        area("Dili PE", json!({"miru:area-trustee-users": r#"["1"]"#})),
        area("Osaka PCG", json!({})),
    ];
    assert_eq!(posts_short_of_sbeis(&areas, 2), vec!["Dili PE".to_owned()]);
    assert!(posts_short_of_sbeis(&areas, 1).is_empty());
}

// -- transmission threshold -----------------------------------------------------

/// A transmit rule that needs signatures sets the count; otherwise the
/// Post's annotation does, as before.
#[test]
fn the_transmission_threshold_is_the_rules_count_when_it_needs_signatures() {
    let required = rule(
        SigningAction::TransmitResults,
        SigningRequirement::Required,
        2,
    );
    assert_eq!(transmission_threshold(Some(&required), 3), 2);
    assert_eq!(transmission_threshold(Some(&required), -1), 2);
    let off = rule(
        SigningAction::TransmitResults,
        SigningRequirement::NotRequired,
        2,
    );
    assert_eq!(transmission_threshold(Some(&off), 3), 3);
    assert_eq!(transmission_threshold(None, 3), 3);
    assert_eq!(transmission_threshold(None, -1), -1);
}

fn package(threshold: i64, signatures: usize) -> MiruTransmissionPackageData {
    let signature = json!({"sbei_miru_id": "eb_1-01", "pub_key": "k", "signature": "s",
                           "certificate_fingerprint": "f"});
    serde_json::from_value(json!({
        "election_id": "e", "area_id": "a", "servers": [], "logs": [],
        "threshold": threshold,
        "documents": [{
            "document_ids": {"eml": "1", "xz": "2", "all_servers": "3"},
            "transaction_id": "t", "servers_sent_to": [],
            "created_at": "2028-05-08T10:00:00+08:00",
            "signatures": vec![signature; signatures],
        }],
    }))
    .unwrap()
}

/// The send refuses a package short of its threshold, with the numbers, and
/// lets one through that has enough; -1 is the 2025 "no minimum".
#[test]
fn a_package_short_of_its_threshold_is_not_sent() {
    let short = check_transmission_signatures(&package(2, 1)).unwrap_err();
    assert_eq!(
        short,
        TransmissionSignaturesShort {
            signatures: 1,
            threshold: 2
        }
    );
    assert!(
        short.to_string().contains("1 of the 2 signatures"),
        "{short}"
    );
    assert_eq!(check_transmission_signatures(&package(2, 2)), Ok(()));
    assert_eq!(check_transmission_signatures(&package(2, 3)), Ok(()));
    assert_eq!(check_transmission_signatures(&package(0, 0)), Ok(()));
    assert_eq!(check_transmission_signatures(&package(-1, 0)), Ok(()));
}

/// An import is logged for the administrator who started it, by username
/// when the token had one; a task without one is the system's.
#[test]
fn an_import_is_logged_for_its_importer_or_the_system() {
    let mut importer = ElectoralLogAdminContext {
        user_id: "admin-id".to_owned(),
        username: Some("admin".to_owned()),
        authorized_election_ids: None,
        area_id: None,
    };
    let actor = importer_actor(Some(&importer));
    assert_eq!(
        (actor.user_id.as_str(), actor.username.as_str()),
        ("admin-id", "admin")
    );
    importer.username = None;
    assert_eq!(importer_actor(Some(&importer)).username, "admin-id");
    let system = importer_actor(None);
    assert_eq!(system.user_id, SYSTEM_ACTOR_ID);
    assert_eq!(system.username, SYSTEM_ACTOR_ID);
}

// -- presets and fixtures ---------------------------------------------------------

/// Every action once, every rule valid.
fn assert_complete(name: &str, rules: &[SigningRule]) {
    let actions: Vec<SigningAction> = rules.iter().map(|rule| rule.action).collect();
    let unique: BTreeSet<String> = actions.iter().map(ToString::to_string).collect();
    assert_eq!(unique.len(), actions.len(), "{name}: an action twice");
    assert_eq!(
        unique,
        SigningAction::iter()
            .map(|action| action.to_string())
            .collect(),
        "{name}: every action has a rule"
    );
    for rule in rules {
        assert_eq!(rule.validate(), Ok(()), "{name}: {}", rule.action);
    }
}

fn find(rules: &[SigningRule], action: SigningAction) -> &SigningRule {
    rules.iter().find(|rule| rule.action == action).unwrap()
}

/// The sample preset of the ticket, as the janitor's client template sets it.
#[test]
fn the_janitor_template_carries_the_sample_preset() {
    let preset: Preset = serde_json::from_str(JANITOR_PRESET).unwrap();
    assert_complete("janitor preset", &preset.signing_rules);
    let needs = |action| {
        let rule = find(&preset.signing_rules, action);
        assert_eq!(rule.requirement, SigningRequirement::Required, "{action}");
        rule.required()
    };
    assert_eq!(needs(SigningAction::InitializeVoting), 2);
    assert_eq!(needs(SigningAction::OpenVoting), 2);
    assert_eq!(needs(SigningAction::CloseVoting), 2);
    assert_eq!(needs(SigningAction::GenerateElectionReturns), 3);
    assert_eq!(needs(SigningAction::GenerateReports), 2);
    assert_eq!(needs(SigningAction::TransmitResults), 2);
    assert_eq!(needs(SigningAction::ApproveVoter), 1);
    assert_eq!(needs(SigningAction::ApproveConfiguration), 2);
    assert_eq!(needs(SigningAction::ConfirmKeyShare), 1);
    assert_eq!(needs(SigningAction::ContributeKeyShare), 1);
    // Three SBEIs per Post sign the election returns, so whoever starts it
    // must be able to sign too.
    assert_eq!(
        find(
            &preset.signing_rules,
            SigningAction::GenerateElectionReturns
        )
        .requester_signing,
        RequesterSigning::Allowed
    );
    assert_eq!(preset.signing_checks, SigningChecks::default());
    let titles: Value =
        serde_json::from_str::<Value>(JANITOR_PRESET).unwrap()["sbei_titles"].clone();
    assert_eq!(titles.as_object().map(|titles| titles.len()), Some(3));
}

/// The janitor's shared tenant template takes the display name from the
/// client's tenant data, and renders to valid JSON with or without one.
#[test]
fn the_janitor_tenant_template_takes_the_display_name_as_a_parameter() {
    let render = |context: Value| -> Value {
        let rendered = handlebars::Handlebars::new()
            .render_template(JANITOR_TENANT, &context)
            .unwrap();
        serde_json::from_str(&rendered).unwrap()
    };
    let client: Value = serde_json::from_str(JANITOR_CLIENT_TENANT).unwrap();
    for name in [
        client["display_name"].as_str().unwrap(),
        "Riverside \"Students\" Union",
    ] {
        let tenant = render(json!({
            "UUID": "8a0c8f5e-5b8e-4f62-9d35-7c3f1f7a0009",
            "current_timestamp": "2026-10-01T00:00:00Z",
            "display_name_json": serde_json::to_string(name).unwrap(),
        }));
        assert_eq!(tenant["id"], "8a0c8f5e-5b8e-4f62-9d35-7c3f1f7a0009");
        assert_eq!(tenant["settings"]["display_name"], name);
        // Additive: the existing settings are still there.
        assert!(tenant["settings"]["i18n"]["en"].is_object());
    }
    let without = render(json!({"UUID": "u", "current_timestamp": "t"}));
    assert!(without["settings"].get("display_name").is_none());
}

/// The second organization's rules read as rules, and differ from the preset.
#[test]
fn the_student_council_has_a_configuration_of_its_own() {
    let council: Preset = serde_json::from_str(STUDENT_COUNCIL).unwrap();
    let preset: Preset = serde_json::from_str(JANITOR_PRESET).unwrap();
    assert_complete("student council", &council.signing_rules);
    assert_ne!(council.signing_rules, preset.signing_rules);
    assert_ne!(council.signing_checks, preset.signing_checks);
}

/// The fixtures' groups name permissions the platform has, and only sign
/// permissions.
#[test]
fn the_organizations_groups_hold_sign_permissions() {
    let council: Value = serde_json::from_str(STUDENT_COUNCIL).unwrap();
    let qualification: Value = serde_json::from_str(include_str!(
        "../../../scripts/dev/scenario/signing-organizations/post-qualification.json"
    ))
    .unwrap();
    let sign_permissions: BTreeSet<String> = SigningAction::iter()
        .map(|action| action.sign_permission().to_string())
        .collect();
    for organization in [council, qualification] {
        for group in organization["groups"].as_array().unwrap() {
            for permission in group["permissions"].as_array().unwrap() {
                let permission = permission.as_str().unwrap();
                assert!(Permissions::from_str(permission).is_ok(), "{permission}");
                assert!(sign_permissions.contains(permission), "{permission}");
            }
        }
    }
}
