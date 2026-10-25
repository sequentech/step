// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

/// The realm and the census, in one archive, checked against each other.
///
/// The owner's own example of what cross-checking should be: two artifacts that
/// arrived together, compared where both are in hand. Nothing is stored about what
/// a census *ought* to contain, so there is nothing to keep in step.
#[test]
fn a_census_column_the_realm_never_declared_is_worth_saying() {
    let realm = |names: &[&str]| {
        let attributes: Vec<Value> = names
            .iter()
            .map(|name| serde_json::json!({"name": name}))
            .collect();
        let profile = serde_json::json!({"attributes": attributes}).to_string();
        serde_json::json!({
            "keycloak_event_realm": {
                "components": {
                    "org.keycloak.userprofile.UserProfileProvider": [
                        {"config": {"kc.user.profile.config": [profile]}}
                    ]
                }
            }
        })
    };

    let census = "username,area_name,branch_code\nada,North,B-14\n";
    let named = |document: &Value| -> Vec<String> {
        let mut report = Report::default();
        check_census_against_profile(document, census, &mut report);
        report.problems.into_iter().filter_map(|p| p.id).collect()
    };

    // Declared: nothing to say. This is what the wizard's own build produces, so a
    // check that fired here would fire on every export it writes.
    assert!(named(&realm(&["branch_code"])).is_empty());

    // Not declared: the platform this came from was dropping the column, silently,
    // and somebody can still ask for a better export.
    assert_eq!(
        named(&realm(&["something_else"])),
        vec!["census.column-not-declared".to_string()]
    );

    // `username` and `area_name` are the platform's own and are never declared as
    // custom attributes. Naming them would make the warning worthless, so the
    // sentence has to mention `branch_code` and nothing else.
    let mut report = Report::default();
    check_census_against_profile(&realm(&[]), census, &mut report);
    let said = &report.problems[0].message;
    assert!(said.contains("branch_code"), "{said}");
    assert!(!said.contains("username"), "{said}");
    assert!(!said.contains("area_name"), "{said}");

    // No realm in the export at all is ordinary: there is nothing to compare with.
    assert!(named(&serde_json::json!({})).is_empty());
}

/// An export with no identifier gets one derived from its name.
///
/// **Reported as an import that left the identifier blank.** An export without one
/// is ordinary — the platform keys events by UUID and does not always write the
/// external identifier — and `validate_plan` refuses a plan that has none, so the
/// old warning told somebody to go and invent a slug for a plan that could not be
/// built until they did. The wizard already derives one from the name for a plan
/// started from nothing; this is the same rule at the other door.
#[test]
fn an_export_with_no_identifier_gets_one_from_its_name() {
    let document = serde_json::json!({
        "election_event": {
            "presentation": {"i18n": {"en": {"name": "Union Election 2027"}}}
        }
    });

    let read = plan_from_event(&document).expect("a readable export");
    assert_eq!(read.plan.external_id, "union-election-2027");
    // Said out loud, because the plan now carries a value the file did not — and
    // it decides every generated identifier in the build.
    assert!(read
        .report
        .problems
        .iter()
        .any(|problem| problem.message.contains("derived from its name")));
}

/// And an export that *does* name one keeps it, untouched.
#[test]
fn an_export_that_names_an_identifier_keeps_it() {
    let document = serde_json::json!({
        "election_event": {
            "external_id": "theirs-2027",
            "presentation": {"i18n": {"en": {"name": "Union Election 2027"}}}
        }
    });

    let read = plan_from_event(&document).expect("a readable export");
    assert_eq!(read.plan.external_id, "theirs-2027");
}

/// A name that slugifies to nothing still gets a usable identifier.
#[test]
fn an_export_with_no_name_either_gets_the_fallback() {
    let document = serde_json::json!({"election_event": {}});

    let read = plan_from_event(&document).expect("a readable export");
    assert_eq!(read.plan.external_id, "election-event");
}

/// A plan built rather than deserialised starts at two trustees, not nought.
///
/// **`#[derive(Default)]` and `#[serde(default)]` disagreed.** The derive gave
/// `trustee_threshold: 0` while the serde default gives 2, and every import path
/// builds its plan with `..Blueprint::default()` — so an imported election event
/// arrived asking for a trustee minimum of nought and warning about it on the same
/// screen. `Default` is now defined as the serde defaults, so the two cannot drift.
#[test]
fn a_default_plan_asks_for_two_trustees() {
    assert_eq!(Blueprint::default().trustee_threshold, 2);

    // Through the import door, which is where it was seen.
    let read = plan_from_event(&serde_json::json!({"election_event": {}}))
        .expect("a readable export");
    assert_eq!(read.plan.trustee_threshold, 2);
}

/// An export whose areas carry no identifier gets one derived from each name.
///
/// **Reported as "the area identifier still didn't load, it was empty".** The
/// platform keys areas by UUID and does not always write the external identifier,
/// and every consequence of a blank one is worse than a derived one: `check_areas`
/// refuses the plan, a voter cannot be resolved to an area without it, and since
/// the Areas screen started showing the identifier it is an empty box somebody has
/// to fill in by hand. Same rule as the event's own identifier.
#[test]
fn areas_with_no_identifier_get_one_from_their_names() {
    let document = serde_json::json!({
        "election_event": {"external_id": "union-2027"},
        "elections": [], "contests": [], "candidates": [], "area_contests": [],
        "areas": [
            {"id": "11111111-1111-1111-1111-111111111111", "name": "North Region"},
            {"id": "22222222-2222-2222-2222-222222222222", "name": "South Region"},
        ],
    });

    let read = plan_from_event(&document).expect("a readable export");
    assert_eq!(
        read.plan
            .areas
            .iter()
            .map(|area| area.external_id.as_str())
            .collect::<Vec<_>>(),
        vec!["north-region", "south-region"]
    );
    assert!(read.report.problems.iter().any(|problem| problem
        .message
        .contains("derived from each area's name")));
}

/// Two areas of one name get identifiers that differ.
///
/// `check_unique_identifiers` refuses a build whose areas share one, so deriving
/// the same slug twice would trade a blank field for a refused plan.
#[test]
fn two_areas_of_one_name_get_different_identifiers() {
    let document = serde_json::json!({
        "election_event": {"external_id": "union-2027"},
        "elections": [], "contests": [], "candidates": [], "area_contests": [],
        "areas": [
            {"id": "1", "name": "Local"},
            {"id": "2", "name": "Local"},
            {"id": "3", "name": ""},
        ],
    });

    let read = plan_from_event(&document).expect("a readable export");
    let ids: Vec<&str> = read
        .plan
        .areas
        .iter()
        .map(|area| area.external_id.as_str())
        .collect();
    // And an area with no name at all is named for its row rather than for the
    // plan-wide `election-event` fallback, which says nothing about which one it is.
    assert_eq!(ids, vec!["local", "local-2", "area-3"]);
}

/// An area the export *does* name keeps its identifier, and a child still finds it.
#[test]
fn a_derived_identifier_is_what_a_child_area_points_at() {
    let document = serde_json::json!({
        "election_event": {"external_id": "union-2027"},
        "elections": [], "contests": [], "candidates": [], "area_contests": [],
        "areas": [
            {"id": "outer", "name": "North Region"},
            {"id": "inner", "name": "North Local 1", "parent_id": "outer"},
            {"id": "kept", "name": "South", "external_id": "theirs-south"},
        ],
    });

    let read = plan_from_event(&document).expect("a readable export");
    let areas = &read.plan.areas;
    assert_eq!(areas[0].external_id, "north-region");
    // The parent is resolved through the same map the derivation filled, so the
    // child points at the derived identifier rather than at nothing.
    assert_eq!(areas[1].parent_external_id.as_deref(), Some("north-region"));
    // And an identifier the export gave is untouched.
    assert_eq!(areas[2].external_id, "theirs-south");
}

/// An Admin Portal export: two areas with no `external_id`, a contest linked to
/// one of them, and the fields a round trip used to drop.
fn admin_export() -> Value {
    serde_json::json!({
        "election_event": {
            "id": "event-uuid",
            "external_id": "postqual",
            "encryption_protocol": "WBRAID",
            "annotations": {
                "miru:event-code": "PQ-2026",
                "ivr:phone-number": "+15550100"
            },
            "presentation": {
                "i18n": {"en": {"name": "Postqual", "common.next": "Onward"}},
                "language_conf": {
                    "enabled_language_codes": ["en", "fil"],
                    "default_language_code": "en"
                },
                "audio_instructions_policy": "enabled",
                "batch_anonymity_policy": "enabled",
                "receipts": {"checks_period_policy": "enabled"},
                "voter_accessibility_settings_policy": "enabled",
                "materials": {"activated": true, "policy": "show-on-start"}
            }
        },
        "keycloak_event_realm": {
            "realm": "tenant-t-event-event-uuid",
            "supportedLocales": ["en", "fil"],
            "authenticationFlows": [{"alias": "scanovate"}]
        },
        "version": "v11.0.0-rc.3",
        "areas": [
            {
                "id": "north-uuid",
                "name": "North Region",
                "description": "Northern precincts",
                "annotations": {"miru:area-code": "N-1"}
            },
            {"id": "south-uuid", "name": "South Region"}
        ],
        "elections": [{
            "id": "election-uuid",
            "external_id": "president",
            "annotations": {"miru:election-code": "E-1"},
            "presentation": {
                "i18n": {"en": {"name": "President"}},
                "start_screen_title_policy": "election"
            }
        }],
        "contests": [{
            "id": "contest-uuid",
            "election_id": "election-uuid",
            "external_id": "president-contest",
            "max_votes": 1,
            "winning_candidates_num": 1,
            "voting_type": "non-preferential",
            "counting_algorithm": "plurality-at-large",
            "min_votes": 0,
            "is_encrypted": true,
            "annotations": {
                "miru:contest-code": "C-1",
                "ivr:i18n": "{\"en\":{\"prompt\":\"President\"}}"
            },
            "tally_configuration": {"tie_breaking_policy": "random"},
            "presentation": {
                "i18n": {"en": {"name": "President"}},
                "columns": null,
                "max_selections_per_type": null,
                "enable_checkable_lists": "allow-selecting-candidates"
            }
        }],
        "candidates": [{
            "id": "candidate-uuid",
            "contest_id": "contest-uuid",
            "external_id": "ada",
            "annotations": {"miru:candidate-code": "001"},
            "presentation": {"i18n": {"en": {"name": "Ada"}}}
        }],
        "area_contests": [
            {"area_id": "north-uuid", "contest_id": "contest-uuid"}
        ]
    })
}

fn first_contest(plan: &Blueprint) -> &PlannedContest {
    &plan.elections[0].contests[0]
}

fn annotation<'a>(
    annotations: &'a BTreeMap<String, String>,
    key: &str,
) -> Option<&'a str> {
    annotations.get(key).map(String::as_str)
}

/// A contest names the area by the identifier `areas()` derived.
///
/// **Reported as "Contest area unknown — nothing has the identifier ''".** The
/// areas got a derived identifier and the contests were linked through the
/// export's own, which an Admin Portal export never writes — so every contest
/// served an area called nothing.
#[test]
fn a_contest_serves_the_area_by_its_derived_identifier() {
    let read = plan_from_event(&admin_export()).expect("a readable export");
    assert_eq!(first_contest(&read.plan).areas, vec!["north-region"]);
}

/// A `null` in a contest's presentation does not wipe out its other rules.
///
/// An Admin Portal export writes `columns: null` and
/// `max_selections_per_type: null`, which no `Layout` accepts — and one refused
/// key used to turn the whole contest into the defaults, so a random tie-break
/// came back as external-procedure and checkable lists came back disabled.
#[test]
fn a_null_in_a_contest_keeps_the_rules_beside_it() {
    let read = plan_from_event(&admin_export()).expect("a readable export");
    let overrides = &first_contest(&read.plan).overrides;
    assert_eq!(
        overrides.tally.tie_breaking_policy.as_deref(),
        Some("random")
    );
    assert_eq!(
        overrides.layout.enable_checkable_lists.as_deref(),
        Some("allow-selecting-candidates")
    );
}

/// A contest's ballot rules come back, under the names the export gives them.
///
/// The export writes `over_vote_policy`; `Policies` calls the field
/// `over_vote`. Read as one bag, the rule was ignored and every imported contest
/// came back on the defaults.
#[test]
fn a_contests_ballot_rules_come_back() {
    let mut document = admin_export();
    document["contests"][0]["presentation"]["over_vote_policy"] =
        serde_json::json!("allowed");
    document["contests"][0]["presentation"]["candidates_order"] =
        serde_json::json!("random");

    let read = plan_from_event(&document).expect("a readable export");
    let policies = &first_contest(&read.plan).overrides.policies;
    assert_eq!(
        policies.over_vote,
        Some(super::super::policy::OverVote::Allowed)
    );
    assert_eq!(
        policies.candidates_order,
        Some(super::super::policy::CandidatesOrder::Random)
    );
}

/// A tie-break override alone survives saving the plan.
///
/// `TallyPatch::is_empty` did not look at it, so `skip_serializing_if` dropped
/// a contest's overrides whenever the tie-break was the only one.
#[test]
fn a_tie_break_override_alone_survives_saving_the_plan() {
    let contest = PlannedContest {
        external_id: "c".to_string(),
        overrides: Overrides {
            tally: super::super::policy::TallyPatch {
                tie_breaking_policy: Some("random".to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
        ..PlannedContest::default()
    };
    let saved = serde_json::to_value(&contest).unwrap();
    let back: PlannedContest = serde_json::from_value(saved).unwrap();
    assert_eq!(
        back.overrides.tally.tie_breaking_policy.as_deref(),
        Some("random")
    );
}

/// Every entity's annotations come back, except the ones the plan models.
///
/// COMELEC's Miru transmission reads `miru:*` keys, and nothing in a plan
/// carried them. The IVR keys stay out because the plan already has fields for
/// them and writes them again.
#[test]
fn annotations_come_back_on_every_entity() {
    let read = plan_from_event(&admin_export()).expect("a readable export");
    let plan = &read.plan;
    let contest = first_contest(plan);

    assert_eq!(
        annotation(&plan.annotations, "miru:event-code"),
        Some("PQ-2026")
    );
    assert!(!plan.annotations.contains_key("ivr:phone-number"));
    assert_eq!(
        annotation(&plan.elections[0].annotations, "miru:election-code"),
        Some("E-1")
    );
    assert_eq!(
        annotation(&contest.annotations, "miru:contest-code"),
        Some("C-1")
    );
    assert!(!contest.annotations.contains_key("ivr:i18n"));
    assert_eq!(
        annotation(&contest.candidates[0].annotations, "miru:candidate-code"),
        Some("001")
    );
    assert_eq!(
        annotation(&plan.areas[0].annotations, "miru:area-code"),
        Some("N-1")
    );
    assert!(plan.areas[1].annotations.is_empty());
}

/// The fields a rebuild used to reset to the template's values.
#[test]
fn the_protocol_and_the_titles_come_back() {
    let read = plan_from_event(&admin_export()).expect("a readable export");
    let plan = &read.plan;
    assert_eq!(plan.encryption_protocol.as_deref(), Some("WBRAID"));
    assert_eq!(plan.areas[0].description, "Northern precincts");
    assert_eq!(plan.elections[0].start_screen_title_policy, "election");
    assert_eq!(plan.materials_activated, Some(true));
    assert_eq!(
        plan.i18n
            .get("en")
            .and_then(|keys| keys.get("common.next"))
            .map(String::as_str),
        Some("Onward")
    );
    // The event's name is a field of its own, not a wording override.
    assert!(!plan
        .i18n
        .get("en")
        .is_some_and(|keys| keys.contains_key("name")));
}

/// The export's event row and realm become the plan's base export.
///
/// That is what carries the presentation rules the plan has no field for and
/// the realm's own flows and locales, through the mechanism `build` already has
/// for a base export. What the plan *does* model is left out of it, so an edit
/// made in the wizard is not quietly undone by the base.
#[test]
fn the_export_becomes_the_plans_base() {
    let read = plan_from_event(&admin_export()).expect("a readable export");
    let base = read.plan.base_export.expect("a base export");

    assert_eq!(
        base.pointer("/keycloak_event_realm/authenticationFlows/0/alias"),
        Some(&serde_json::json!("scanovate"))
    );
    assert_eq!(
        base.pointer(
            "/election_event/presentation/receipts/checks_period_policy"
        ),
        Some(&serde_json::json!("enabled"))
    );
    // Kept, because `build_realm` swaps it out of the realm's URLs.
    assert_eq!(
        base.pointer("/election_event/id"),
        Some(&serde_json::json!("event-uuid"))
    );
    assert_eq!(
        base.get("version"),
        Some(&serde_json::json!("v11.0.0-rc.3"))
    );
    assert!(base.pointer("/election_event/annotations").is_none());
    assert!(base.pointer("/election_event/description").is_none());
    assert!(base.pointer("/election_event/presentation/i18n").is_none());
    // Per-entity rows are not a base: `build` would spread the first of each
    // over all of them.
    assert!(base.get("contests").is_none());
}

/// What a plan read from an Admin Portal export builds back into.
///
/// The round trip the report was about: import, rebuild, re-import. Built the
/// way `compile_plan` builds, minus the plan validator — an export carries no
/// trustees or contacts, and that is said elsewhere.
#[test]
fn an_admin_export_rebuilds_with_what_it_carried() {
    use crate::election_config::build::{build, BuildOptions};
    use crate::election_config::render::TemplateSet;
    use crate::election_config::sources::Sources;

    let plan = plan_from_event(&admin_export())
        .expect("a readable export")
        .plan;
    let sources = Sources::from_plan(&plan);
    let workbook = super::super::architect::to_workbook(&plan, &sources)
        .expect("the plan compiles to rows");
    let options = BuildOptions {
        base_export: plan.base_export.clone(),
        ..BuildOptions::default()
    };
    let bundle = match build(
        &workbook,
        &TemplateSet::builtin().unwrap(),
        &options,
        &sources,
    ) {
        Ok(bundle) => bundle,
        Err(report) => panic!("expected a clean build, got:\n{report}"),
    };
    let export = &bundle.export;
    let at = |pointer: &str| export.pointer(pointer).cloned();
    let said = |text: &str| Some(serde_json::json!(text));

    let event = "/election_event";
    assert_eq!(at(&format!("{event}/encryption_protocol")), said("WBRAID"));
    assert_eq!(
        at(&format!("{event}/annotations/miru:event-code")),
        said("PQ-2026")
    );
    // The IVR key went through its own field and came back as itself.
    assert_eq!(
        at(&format!("{event}/annotations/ivr:phone-number")),
        said("+15550100")
    );
    for (path, value) in [
        ("audio_instructions_policy", "enabled"),
        ("batch_anonymity_policy", "enabled"),
        ("receipts/checks_period_policy", "enabled"),
        ("voter_accessibility_settings_policy", "enabled"),
        ("materials/policy", "show-on-start"),
        ("i18n/en/common.next", "Onward"),
    ] {
        assert_eq!(
            at(&format!("{event}/presentation/{path}")),
            said(value),
            "{path}"
        );
    }
    assert_eq!(
        at(&format!("{event}/presentation/materials/activated")),
        Some(serde_json::json!(true))
    );

    // The platform that exported it can import it back.
    assert_eq!(at("/version"), said("v11.0.0-rc.3"));

    assert_eq!(
        at("/elections/0/annotations/miru:election-code"),
        said("E-1")
    );
    assert_eq!(
        at("/elections/0/presentation/start_screen_title_policy"),
        said("election")
    );

    assert_eq!(at("/contests/0/annotations/miru:contest-code"), said("C-1"));
    assert_eq!(
        at("/contests/0/annotations/ivr:i18n"),
        said("{\"en\":{\"prompt\":\"President\"}}")
    );
    assert_eq!(
        at("/contests/0/tally_configuration/tie_breaking_policy"),
        said("random")
    );
    assert_eq!(
        at("/contests/0/presentation/enable_checkable_lists"),
        said("allow-selecting-candidates")
    );

    assert_eq!(
        at("/candidates/0/annotations/miru:candidate-code"),
        said("001")
    );

    assert_eq!(at("/areas/0/annotations/miru:area-code"), said("N-1"));
    assert_eq!(at("/areas/0/description"), said("Northern precincts"));
    // The contest is linked to the area the export linked it to.
    let north = at("/areas/0/id");
    assert!(north.is_some());
    assert_eq!(at("/area_contests/0/area_id"), north);

    // The realm is the export's, patched rather than replaced.
    assert_eq!(
        at("/keycloak_event_realm/authenticationFlows/0/alias"),
        said("scanovate")
    );
    // The login page's languages follow the event's, as on any build.
    assert_eq!(
        at("/keycloak_event_realm/supportedLocales"),
        Some(serde_json::json!(["en", "fil"]))
    );
}

/// An export with no realm and an otherwise empty event gives no base.
#[test]
fn a_bare_export_gives_no_base() {
    let read = plan_from_event(&serde_json::json!({"election_event": {}}))
        .expect("a readable export");
    assert!(read.plan.base_export.is_none());
}
