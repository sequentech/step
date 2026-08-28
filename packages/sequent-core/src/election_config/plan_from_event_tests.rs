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
