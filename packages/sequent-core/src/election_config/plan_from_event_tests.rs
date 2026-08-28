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
