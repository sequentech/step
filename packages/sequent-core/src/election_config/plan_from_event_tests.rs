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

// -- what does not look like an export -------------------------------------

#[test]
fn json_with_no_election_event_is_refused_with_what_to_open_instead() {
    for document in [
        serde_json::json!({}),
        serde_json::json!({"election_event": "not an object"}),
        serde_json::json!([1, 2]),
    ] {
        let refused = plan_from_event(&document).expect_err("not an export");
        assert_eq!(refused.problems.len(), 1);
        assert!(
            refused.problems[0]
                .message
                .contains("export_election_event"),
            "{}",
            refused.problems[0].message
        );
        assert!(refused.has_errors());
    }
}

/// An export with the awkward parts in it: a blank translation, a contest whose
/// description is only in the flat column, a tally configuration, an area whose
/// parent is missing, and IVR annotations nobody could parse.
fn awkward() -> Value {
    serde_json::json!({
        "election_event": {
            "external_id": "union-2027",
            "presentation": {
                "i18n": {
                    "en": {"name": "Union Election"},
                    "es": {"name": ""},
                    "fr": {"description": "Tous"}
                }
            },
            "annotations": {
                "ivr:config": "{not json",
                "ivr:prompts": "also not json",
                "ivr:phone-number": "   "
            }
        },
        "areas": [
            {"id": "a1", "external_id": "north", "name": "North"},
            {"id": "a2", "external_id": "south", "name": "South",
             "parent_id": "gone"},
            "not an area"
        ],
        "elections": [
            {"id": "e1", "external_id": "officers",
             "presentation": {"i18n": {"en": {"name": "Officers"}}}}
        ],
        "contests": [
            {"id": "c1", "external_id": "president", "election_id": "e1",
             "description": "Elects the president",
             "tally_configuration": {"counting_algorithm": "plurality-at-large"},
             "presentation": {"i18n": {"en": {"name": "President"}}}}
        ],
        "candidates": [
            {"id": "k1", "external_id": "alice", "contest_id": "c1",
             "image_document_id": "photo-alice",
             "presentation": {"i18n": {"en": {"name": "Alice"}}}},
            {"id": "k2", "external_id": "bob", "contest_id": "c1",
             "image_document_id": "photo-bob",
             "presentation": {"i18n": {"en": {"name": "Bob"}}}}
        ],
        "support_materials": [
            {"external_id": "guide", "document_id": "doc-guide", "kind": "PDF",
             "is_hidden": true,
             "presentation": {"i18n": {"en": {"title": "Voter guide"}}}},
            {"external_id": "faq", "document_id": "doc-faq", "kind": "PDF"},
            "not a row"
        ]
    })
}

#[test]
fn a_blank_translation_is_no_translation() {
    let read = plan_from_event(&awkward()).expect("reads");
    assert_eq!(
        read.plan.name.by_language.keys().collect::<Vec<_>>(),
        ["en"],
        "the blank Spanish name is not a name"
    );
    assert_eq!(
        read.plan
            .description
            .by_language
            .get("fr")
            .map(String::as_str),
        Some("Tous")
    );
}

#[test]
fn telephone_annotations_nobody_could_parse_are_no_telephone_configuration() {
    let read = plan_from_event(&awkward()).expect("reads");
    assert!(read.plan.ivr.is_none());
}

#[test]
fn an_area_whose_parent_is_not_in_the_export_comes_back_without_one() {
    let read = plan_from_event(&awkward()).expect("reads");
    assert_eq!(read.plan.areas.len(), 2);
    assert!(read.plan.areas[1].parent_external_id.is_none());
    assert!(read
        .report
        .problems
        .iter()
        .any(|problem| problem.message.contains("`south` names a parent")));
}

#[test]
fn a_contests_description_falls_back_to_the_flat_column() {
    let read = plan_from_event(&awkward()).expect("reads");
    let contest = &read.plan.elections[0].contests[0];
    assert_eq!(
        contest
            .description
            .by_language
            .get("en")
            .map(String::as_str),
        Some("Elects the president")
    );
}

// -- what travels beside the document ---------------------------------------

fn filled(
    beside: &Beside,
) -> (Blueprint, Report, crate::election_config::sources::Sources) {
    let document = awkward();
    let mut read = plan_from_event(&document).expect("reads");
    let mut report = Report::default();
    let sources =
        fill_from_archive(&mut read.plan, &document, beside, &mut report);
    (read.plan, report, sources)
}

#[test]
fn a_photograph_the_archive_lacks_is_named_and_the_rest_are_matched() {
    let (plan, report, _) = filled(&Beside {
        voters: None,
        files: vec![
            // The platform's tempfile prefix, which is why the match is unanchored.
            (
                "images/enGgihs9azd5document_photo-alice_alice.jpg".to_string(),
                vec![1, 2, 3],
            ),
            // The right marker in the wrong folder is not a photograph.
            ("elsewhere/document_photo-bob_bob.jpg".to_string(), vec![4]),
        ],
    });

    let candidates = &plan.elections[0].contests[0].candidates;
    let alice = candidates[0].image.as_ref().expect("alice's photograph");
    assert_eq!(alice.file_name, "alice.jpg");
    assert_eq!(alice.bytes, vec![1, 2, 3]);
    assert!(candidates[1].image.is_none());

    let said = report
        .problems
        .iter()
        .find(|problem| problem.path == "elections")
        .expect("the missing one is named");
    assert!(said.message.contains("a candidate"), "{}", said.message);
    assert!(said.message.contains("bob"), "{}", said.message);
}

#[test]
fn several_missing_photographs_are_counted() {
    let (_, report, _) = filled(&Beside::default());
    let said = report
        .problems
        .iter()
        .find(|problem| problem.path == "elections")
        .expect("both are named");
    assert!(said.message.contains("2 candidates"), "{}", said.message);
    assert!(said.message.contains("alice, bob"), "{}", said.message);
}

#[test]
fn support_material_is_read_with_its_file_or_named_as_missing() {
    let (plan, report, _) = filled(&Beside {
        voters: None,
        files: vec![(
            "export_S3_files/xyz_document_doc-guide_guide.pdf".to_string(),
            b"%PDF".to_vec(),
        )],
    });

    assert_eq!(
        plan.materials.len(),
        2,
        "the row that is not one is skipped"
    );
    let guide = &plan.materials[0];
    assert_eq!(guide.external_id, "guide");
    assert_eq!(guide.file_name, "guide.pdf");
    assert_eq!(guide.bytes, b"%PDF".to_vec());
    assert_eq!(guide.kind, "PDF");
    assert!(guide.is_hidden);
    assert_eq!(
        guide.title.by_language.get("en").map(String::as_str),
        Some("Voter guide")
    );
    let faq = &plan.materials[1];
    assert_eq!(faq.file_name, "");
    assert!(faq.bytes.is_empty());
    assert!(!faq.is_hidden);

    let said = report
        .problems
        .iter()
        .find(|problem| problem.path == "materials")
        .expect("the missing one is named");
    assert!(said.message.ends_with(": faq"), "{}", said.message);
}

#[test]
fn a_census_that_cannot_be_read_leaves_the_plan_with_nobody_and_says_so() {
    for (csv, fragment) in [
        ("", "could not be read"),
        ("email,first_name\na@b.org,Ada\n", "username"),
    ] {
        let (_, report, sources) = filled(&Beside {
            voters: Some(csv.to_string()),
            files: Vec::new(),
        });
        assert!(sources.census.is_none(), "{csv:?}");
        assert!(
            report
                .problems
                .iter()
                .any(|problem| problem.path == "voters"
                    && problem.message.contains(fragment)),
            "{csv:?}: {report}"
        );
    }
}

#[test]
fn a_census_from_the_export_resolves_areas_by_name_and_keeps_its_own_columns() {
    let (_, _, sources) = filled(&Beside {
        voters: Some(
            "username,email,area_name,id,branch\nada,a@b.org,North,7,west\ngrace,,Atlantis,8,\n"
                .to_string(),
        ),
        files: Vec::new(),
    });
    let census = sources.census.expect("a census");
    census.rewind().unwrap();
    let voters = census.next_batch(10).unwrap();
    assert_eq!(voters.len(), 2);
    assert_eq!(voters[0].area_external_id, "north");
    assert_eq!(
        voters[0].extra.get("branch").map(String::as_str),
        Some("west")
    );
    assert!(!voters[0].extra.contains_key("id"));
    // Kept as written so validation can point at it.
    assert_eq!(voters[1].area_external_id, "Atlantis");
    assert!(voters[1].extra.is_empty(), "a blank cell is not a value");
}

#[test]
fn a_realm_profile_in_an_unexpected_shape_is_nothing_to_compare_with() {
    let census = "username,branch_code\nada,B-14\n";
    let with_config = |config: Value| {
        serde_json::json!({
            "keycloak_event_realm": {
                "components": {
                    "org.keycloak.userprofile.UserProfileProvider": [
                        {"config": {"kc.user.profile.config": config}}
                    ]
                }
            }
        })
    };
    let said = |document: &Value, csv: &str| {
        let mut report = Report::default();
        check_census_against_profile(document, csv, &mut report);
        report.problems.len()
    };

    // A bare string rather than a one-element list is read the same way.
    let profile =
        serde_json::json!({"attributes": [{"name": "other"}]}).to_string();
    assert_eq!(said(&with_config(Value::String(profile)), census), 1);
    // Anything else is not a profile.
    assert_eq!(said(&with_config(serde_json::json!(42)), census), 0);
    // Nor is a string that is not JSON.
    assert_eq!(said(&with_config(serde_json::json!("{nope")), census), 0);
    // And an unreadable census is not this check's to report.
    let profile = serde_json::json!({"attributes": []}).to_string();
    assert_eq!(said(&with_config(Value::String(profile)), ""), 0);
}

#[test]
fn two_columns_the_realm_never_declared_are_named_together() {
    let profile = serde_json::json!({"attributes": []}).to_string();
    let document = serde_json::json!({
        "keycloak_event_realm": {
            "components": {
                "org.keycloak.userprofile.UserProfileProvider": [
                    {"config": {"kc.user.profile.config": profile}}
                ]
            }
        }
    });
    let mut report = Report::default();
    check_census_against_profile(
        &document,
        "username,branch,seniority\nada,w,3\n",
        &mut report,
    );
    let said = &report.problems[0].message;
    assert!(said.contains("columns"), "{said}");
    assert!(said.contains("dropping them: branch, seniority"), "{said}");
}
