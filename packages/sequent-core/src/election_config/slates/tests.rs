// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::ballot::Candidate;
use serde_json::json;
use std::collections::HashMap;

const PATH: &str = "slates";

fn contest(id: &str, max_votes: i64, candidates: &[&str]) -> Contest {
    Contest {
        id: id.to_string(),
        max_votes,
        candidates: candidates
            .iter()
            .map(|candidate_id| Candidate {
                id: candidate_id.to_string(),
                contest_id: id.to_string(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn contests() -> Vec<Contest> {
    vec![
        contest(
            "president",
            1,
            &["f-president", "m-president", "i-president"],
        ),
        contest(
            "trustees",
            3,
            &[
                "f-t1", "f-t2", "f-t3", "m-t1", "m-t2", "m-t3", "v-t1", "v-t2",
                "v-t3", "i-t1",
            ],
        ),
    ]
}

fn sample() -> serde_json::Value {
    json!({
        "version": 1,
        "mobile_candidate_lists": "expanded",
        "slates": [
            {
                "id": "forward",
                "name": {"en": "Forward Together", "es": "Adelante Juntos"},
                "members": {
                    "president": ["f-president"],
                    "trustees": ["f-t1", "f-t2", "f-t3"]
                }
            },
            {
                "id": "members",
                "name": {"en": "Members First"},
                "members": {
                    "president": ["m-president"],
                    "trustees": ["m-t1", "m-t2", "m-t3"]
                }
            },
            {
                "id": "voices",
                "name": {"en": "Independent Voices"},
                "members": {"trustees": ["v-t1", "v-t2", "v-t3"]}
            }
        ]
    })
}

fn problems_of(document: serde_json::Value) -> Vec<Problem> {
    parse(&document.to_string(), PATH)
        .expect_err("the configuration should be rejected")
}

fn only_problem(document: serde_json::Value) -> Problem {
    let mut problems = problems_of(document);
    assert_eq!(problems.len(), 1, "{problems:?}");
    problems.remove(0)
}

fn ballot_style(annotation: Option<String>) -> BallotStyle {
    let document = json!({
        "id": "style",
        "tenant_id": "tenant",
        "election_event_id": "event",
        "election_id": "election",
        "area_id": "area",
        "contests": [],
    });
    let mut ballot_style: BallotStyle =
        serde_json::from_value(document).expect("a minimal ballot style");
    ballot_style.contests = contests();
    ballot_style.election_annotations = annotation
        .map(|text| HashMap::from([(SLATES_ANNOTATION.to_string(), text)]));
    ballot_style
}

#[test]
fn full_and_partial_slates_parse_in_their_configured_order() {
    let config = parse(&sample().to_string(), PATH).expect("a valid sample");

    assert_eq!(config.version, SLATES_VERSION);
    assert_eq!(
        config.mobile_candidate_lists,
        MobileCandidateLists::Expanded
    );
    let ids: Vec<&str> = config
        .slates
        .iter()
        .map(|slate| slate.id.as_str())
        .collect();
    assert_eq!(ids, ["forward", "members", "voices"]);
    assert_eq!(config.slates[0].name["es"], "Adelante Juntos");
    assert_eq!(
        config.slates[2].members["trustees"],
        ["v-t1", "v-t2", "v-t3"]
    );
    assert!(
        check_references(&config, &contests(), Scope::Election, PATH)
            .is_empty()
    );
}

#[test]
fn candidate_lists_are_collapsed_on_phones_unless_configured() {
    let mut document = sample();
    document
        .as_object_mut()
        .expect("an object")
        .remove("mobile_candidate_lists");

    let config = parse(&document.to_string(), PATH).expect("a valid sample");

    assert_eq!(
        config.mobile_candidate_lists,
        MobileCandidateLists::Collapsed
    );
}

#[test]
fn a_configuration_survives_serialization() {
    let config = parse(&sample().to_string(), PATH).expect("a valid sample");
    let text = serde_json::to_string(&config).expect("serializable");

    assert_eq!(parse(&text, PATH), Ok(config));
}

#[test]
fn text_that_is_not_json_is_unreadable() {
    let problems = parse("{", PATH).expect_err("not JSON");

    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].code, Code::Unreadable);
    assert_eq!(problems[0].path, PATH);
}

#[test]
fn another_version_is_refused() {
    let mut document = sample();
    document["version"] = json!(2);

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::IncompatibleVersion);
    assert_eq!(problem.path, "slates.version");
}

#[test]
fn a_missing_version_is_refused() {
    let problem = only_problem(json!({"slates": []}));

    assert_eq!(problem.code, Code::IncompatibleVersion);
}

#[test]
fn an_unknown_field_is_refused() {
    let mut document = sample();
    document["slates"][0]["motto"] = json!("Together");

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::InvalidValue);
    assert!(problem.message.contains("motto"), "{}", problem.message);
}

#[test]
fn an_unknown_candidate_list_mode_is_refused() {
    let mut document = sample();
    document["mobile_candidate_lists"] = json!("hidden");

    assert_eq!(only_problem(document).code, Code::InvalidValue);
}

#[test]
fn a_configuration_over_the_size_limit_is_refused() {
    let text = " ".repeat(MAX_CONFIG_BYTES + 1);

    let problems = parse(&text, PATH).expect_err("too large");

    assert_eq!(problems[0].code, Code::InvalidValue);
}

#[test]
fn a_slate_id_is_a_short_plain_identifier() {
    for id in ["", "-forward", "forward together", &"a".repeat(65)] {
        let mut document = sample();
        document["slates"][0]["id"] = json!(id);

        let problem = only_problem(document);

        assert_eq!(problem.code, Code::InvalidValue, "id {id:?}");
        assert_eq!(problem.path, "slates.slates[0].id");
    }
}

#[test]
fn two_slates_cannot_share_an_id() {
    let mut document = sample();
    document["slates"][1]["id"] = json!("forward");

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::DuplicateId);
    assert_eq!(problem.path, "slates.slates[1].id");
}

#[test]
fn a_slate_needs_a_name() {
    let mut document = sample();
    document["slates"][0]["name"] = json!({});

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::MissingField);
    assert_eq!(problem.path, "slates.slates[0].name");
}

#[test]
fn a_name_needs_a_language_code() {
    let mut document = sample();
    document["slates"][0]["name"][" "] = json!("Forward Together");

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::InvalidValue);
    assert_eq!(problem.path, "slates.slates[0].name");
}

#[test]
fn a_blank_translation_is_refused() {
    let mut document = sample();
    document["slates"][0]["name"]["es"] = json!("   ");

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::MissingField);
    assert_eq!(problem.path, "slates.slates[0].name.es");
}

#[test]
fn a_name_has_a_length_limit() {
    let mut document = sample();
    document["slates"][0]["name"]["en"] = json!("a".repeat(MAX_NAME_CHARS));
    assert!(parse(&document.to_string(), PATH).is_ok());

    document["slates"][0]["name"]["en"] = json!("a".repeat(MAX_NAME_CHARS + 1));

    assert_eq!(only_problem(document).code, Code::InvalidValue);
}

#[test]
fn a_name_is_one_line_of_plain_text() {
    let mut document = sample();
    document["slates"][0]["name"]["en"] = json!("Forward\nTogether");

    assert_eq!(only_problem(document).code, Code::InvalidValue);
}

#[test]
fn two_slates_cannot_share_a_name_in_one_language() {
    let mut document = sample();
    document["slates"][1]["name"]["en"] = json!(" forward together ");

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::InvalidValue);
    assert_eq!(problem.path, "slates.slates[1].name.en");
}

#[test]
fn the_same_name_in_different_languages_is_accepted() {
    let mut document = sample();
    document["slates"][1]["name"]["fr"] = json!("Forward Together");

    assert!(parse(&document.to_string(), PATH).is_ok());
}

#[test]
fn a_slate_needs_candidates() {
    let mut document = sample();
    document["slates"][2]["members"] = json!({});

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::MissingField);
    assert_eq!(problem.path, "slates.slates[2].members");
}

#[test]
fn a_listed_contest_needs_candidates() {
    let mut document = sample();
    document["slates"][2]["members"]["trustees"] = json!([]);

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::MissingField);
    assert_eq!(problem.path, "slates.slates[2].members[\"trustees\"]");
}

#[test]
fn a_candidate_is_listed_once() {
    let mut document = sample();
    document["slates"][2]["members"]["trustees"] = json!(["v-t1", "v-t1"]);

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::DuplicateId);
    assert!(problem.message.contains("more than once"));
}

#[test]
fn a_candidate_belongs_to_one_slate() {
    let mut document = sample();
    document["slates"][2]["members"]["trustees"] = json!(["f-t1"]);

    let problem = only_problem(document);

    assert_eq!(problem.code, Code::DuplicateId);
    assert!(problem.message.contains("'forward' and 'voices'"));
}

#[test]
fn every_problem_is_reported_at_once() {
    let mut document = sample();
    document["slates"][0]["name"] = json!({});
    document["slates"][1]["id"] = json!("forward");

    assert_eq!(problems_of(document).len(), 2);
}

#[test]
fn a_slate_with_fewer_candidates_than_seats_is_accepted() {
    let mut document = sample();
    document["slates"][2]["members"]["trustees"] = json!(["v-t1"]);
    let config = parse(&document.to_string(), PATH).expect("a partial slate");

    assert!(
        check_references(&config, &contests(), Scope::Election, PATH)
            .is_empty()
    );
}

#[test]
fn an_unknown_contest_is_a_mistake_in_an_election() {
    let mut document = sample();
    document["slates"][2]["members"] = json!({"auditors": ["v-t1"]});
    let config = parse(&document.to_string(), PATH).expect("well formed");

    let problems =
        check_references(&config, &contests(), Scope::Election, PATH);

    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].code, Code::DanglingReference);
    assert_eq!(problems[0].path, "slates.slates[2].members[\"auditors\"]");
}

#[test]
fn a_contest_outside_the_ballot_style_is_skipped() {
    let mut document = sample();
    document["slates"][2]["members"] = json!({"auditors": ["v-t1"]});
    let config = parse(&document.to_string(), PATH).expect("well formed");

    assert!(
        check_references(&config, &contests(), Scope::BallotStyle, PATH)
            .is_empty()
    );
}

#[test]
fn an_unknown_candidate_is_a_mistake_in_any_scope() {
    let mut document = sample();
    document["slates"][0]["members"]["president"] = json!(["nobody"]);
    let config = parse(&document.to_string(), PATH).expect("well formed");

    for scope in [Scope::Election, Scope::BallotStyle] {
        let problems = check_references(&config, &contests(), scope, PATH);

        assert_eq!(problems.len(), 1, "{scope:?}");
        assert_eq!(problems[0].code, Code::DanglingReference);
        assert!(problems[0].message.contains("'nobody'"));
    }
}

#[test]
fn a_candidate_of_another_contest_is_not_a_member() {
    let mut document = sample();
    document["slates"][0]["members"]["president"] = json!(["i-t1"]);
    let config = parse(&document.to_string(), PATH).expect("well formed");

    let problems =
        check_references(&config, &contests(), Scope::Election, PATH);

    assert_eq!(problems.len(), 1);
}

#[test]
fn a_ballot_style_without_the_annotation_has_no_slates() {
    assert_eq!(ballot_style_slates(&ballot_style(None)), Ok(None));
}

#[test]
fn a_ballot_style_carries_its_election_slates() {
    let ballot_style = ballot_style(Some(sample().to_string()));

    let config = ballot_style_slates(&ballot_style)
        .expect("a valid configuration")
        .expect("slates");

    assert_eq!(config.slates.len(), 3);
}

#[test]
fn a_ballot_style_with_a_broken_configuration_reports_why() {
    let problems = ballot_style_slates(&ballot_style(Some("{".to_string())))
        .expect_err("not JSON");

    assert_eq!(problems[0].code, Code::Unreadable);
    assert_eq!(problems[0].path, "election_annotations[\"sequent.slates\"]");
}

#[test]
fn a_ballot_style_refuses_a_candidate_it_does_not_have() {
    let mut document = sample();
    document["slates"][0]["members"]["president"] = json!(["nobody"]);

    let problems =
        ballot_style_slates(&ballot_style(Some(document.to_string())))
            .expect_err("an unknown candidate");

    assert_eq!(problems[0].code, Code::DanglingReference);
}
