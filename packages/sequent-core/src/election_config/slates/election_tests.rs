// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Unit-test code.
// Coverage exclusion: scripts/coverage/profiles.toml.

//! Tests for [`super`]: each starts from a configuration that passes and
//! breaks exactly one rule.

use super::*;
use crate::election_config::problem::Code;
use serde_json::{json, Value};

const PATH: &str = "sequent.slates";
const ELECTION: &str = "election-officers";
const PRESIDENT: &str = "contest-president";
const TRUSTEES: &str = "contest-trustees";

fn election(annotations: Value) -> Election {
    serde_json::from_value(json!({
        "id": ELECTION,
        "tenant_id": "tenant",
        "election_event_id": "event",
        "annotations": annotations,
    }))
    .unwrap()
}

fn contest(id: &str, election_id: &str, max_votes: i64) -> Value {
    json!({
        "id": id,
        "tenant_id": "tenant",
        "election_event_id": "event",
        "election_id": election_id,
        "min_votes": 0,
        "max_votes": max_votes,
        "counting_algorithm": "plurality-at-large",
    })
}

fn contests() -> Vec<Contest> {
    serde_json::from_value(json!([
        contest(PRESIDENT, ELECTION, 1),
        contest(TRUSTEES, ELECTION, 3),
        contest("contest-elsewhere", "election-other", 1),
    ]))
    .unwrap()
}

fn candidate(id: &str, contest_id: &str) -> Value {
    json!({
        "id": id,
        "tenant_id": "tenant",
        "election_event_id": "event",
        "contest_id": contest_id,
    })
}

fn candidates() -> Vec<Candidate> {
    serde_json::from_value(json!([
        candidate("president-a", PRESIDENT),
        candidate("president-b", PRESIDENT),
        candidate("trustee-a", TRUSTEES),
        candidate("trustee-b", TRUSTEES),
        candidate("trustee-c", TRUSTEES),
        candidate("trustee-d", TRUSTEES),
        candidate("elsewhere-a", "contest-elsewhere"),
    ]))
    .unwrap()
}

/// One full slate and one trustee-only slate.
fn sound() -> Value {
    json!({
        "version": 1,
        "mobile_candidate_lists": "collapsed",
        "slates": [
            {
                "id": "forward-together",
                "name": {"en": "Forward Together", "es": "Adelante"},
                "members": {
                    PRESIDENT: ["president-a"],
                    TRUSTEES: ["trustee-a", "trustee-b"],
                },
            },
            {
                "id": "independent-voices",
                "name": {"en": "Independent Voices"},
                "members": {TRUSTEES: ["trustee-c"]},
            },
        ],
    })
}

fn check(config: &Value) -> Vec<Problem> {
    check_election(
        &election(json!({SLATES_ANNOTATION: config.to_string()})),
        &contests(),
        &candidates(),
        PATH,
    )
}

fn codes(problems: &[Problem]) -> Vec<Code> {
    problems.iter().map(|problem| problem.code).collect()
}

#[test]
fn full_and_partial_slates_are_accepted() {
    assert_eq!(check(&sound()), Vec::new());
}

#[test]
fn an_election_without_the_annotation_has_nothing_to_check() {
    for annotations in [Value::Null, json!({}), json!({"other": "value"})] {
        let problems = check_election(
            &election(annotations),
            &contests(),
            &candidates(),
            PATH,
        );
        assert_eq!(problems, Vec::new());
    }
}

#[test]
fn an_annotation_that_is_not_text_is_refused() {
    let problems = check_election(
        &election(json!({SLATES_ANNOTATION: sound()})),
        &contests(),
        &candidates(),
        PATH,
    );
    assert_eq!(codes(&problems), vec![Code::InvalidValue]);
}

#[test]
fn an_unreadable_annotation_is_refused() {
    let problems = check_election(
        &election(json!({SLATES_ANNOTATION: "{not json"})),
        &contests(),
        &candidates(),
        PATH,
    );
    assert!(!problems.is_empty());
}

#[test]
fn a_slate_needs_a_name_in_the_default_language() {
    let mut config = sound();
    config["slates"][1]["name"] = json!({"es": "Voces Independientes"});
    let problems = check(&config);
    assert_eq!(codes(&problems), vec![Code::MissingField]);
    assert_eq!(problems[0].path, "sequent.slates.slates[1].name");
}

#[test]
fn a_slate_cannot_exceed_a_contest_limit() {
    let mut config = sound();
    config["slates"][0]["members"][PRESIDENT] =
        json!(["president-a", "president-b"]);
    let problems = check(&config);
    assert_eq!(codes(&problems), vec![Code::ContestArithmetic]);

    let mut config = sound();
    config["slates"][1]["members"] = json!({PRESIDENT: ["president-b"]});
    config["slates"][0]["members"][TRUSTEES] =
        json!(["trustee-a", "trustee-b", "trustee-c", "trustee-d"]);
    assert_eq!(codes(&check(&config)), vec![Code::ContestArithmetic]);
}

#[test]
fn a_member_from_another_election_is_a_dangling_reference() {
    let mut config = sound();
    config["slates"][1]["members"] =
        json!({"contest-elsewhere": ["elsewhere-a"]});
    assert!(codes(&check(&config)).contains(&Code::DanglingReference));
}

#[test]
fn a_candidate_under_the_wrong_contest_is_refused() {
    let mut config = sound();
    config["slates"][1]["members"] = json!({PRESIDENT: ["trustee-c"]});
    assert!(codes(&check(&config)).contains(&Code::DanglingReference));
}

#[test]
fn preferential_and_cumulative_contests_cannot_carry_slates() {
    for algorithm in ["instant-runoff", "borda", "cumulative"] {
        let mut contests = contests();
        contests[1].counting_algorithm = Some(algorithm.to_string());
        let problems = check_election(
            &election(json!({SLATES_ANNOTATION: sound().to_string()})),
            &contests,
            &candidates(),
            PATH,
        );
        assert!(
            codes(&problems).contains(&Code::InvalidValue),
            "{algorithm} was accepted"
        );
    }
}

#[test]
fn options_that_are_not_candidates_cannot_be_members() {
    for flag in [
        "is_disabled",
        "is_write_in",
        "is_explicit_invalid",
        "is_explicit_blank",
        "is_category_list",
    ] {
        let mut candidates = candidates();
        candidates[2].presentation = Some(json!({flag: true}));
        let problems = check_election(
            &election(json!({SLATES_ANNOTATION: sound().to_string()})),
            &contests(),
            &candidates,
            PATH,
        );
        assert_eq!(codes(&problems), vec![Code::InvalidValue], "{flag}");
        assert_eq!(
            problems[0].path,
            format!("sequent.slates.slates[0].members[\"{TRUSTEES}\"]")
        );
    }
}

#[test]
fn canonical_form_ignores_formatting_and_key_order() {
    let compact = sound().to_string();
    let pretty = serde_json::to_string_pretty(&sound()).unwrap();
    let reordered = json!({
        "slates": sound()["slates"],
        "version": 1,
    })
    .to_string();
    let canonical = canonicalize(&compact, PATH).unwrap();
    assert_eq!(canonicalize(&pretty, PATH).unwrap(), canonical);
    assert_eq!(canonicalize(&reordered, PATH).unwrap(), canonical);
    assert_eq!(canonicalize(&canonical, PATH).unwrap(), canonical);
    assert!(canonicalize("{not json", PATH).is_err());
}

fn ballot_style(
    contest_ids: &[&str],
    config: Option<&Value>,
) -> ballot::BallotStyle {
    let contests: Vec<Contest> = contests()
        .into_iter()
        .filter(|contest| contest_ids.contains(&contest.id.as_str()))
        .collect();
    ballot::BallotStyle {
        id: "style".to_string(),
        tenant_id: "tenant".to_string(),
        election_event_id: "event".to_string(),
        election_id: ELECTION.to_string(),
        num_allowed_revotes: None,
        description: None,
        public_key: None,
        area_id: "area".to_string(),
        area_presentation: None,
        election_event_presentation: None,
        election_presentation: None,
        election_dates: None,
        election_event_annotations: None,
        area_annotations: None,
        multi_contest_encoding_mode: None,
        ballot_box_key: None,
        contests: election_contests(&contests, &candidates(), "en", PATH)
            .unwrap(),
        election_annotations: Some(
            config
                .map(|config| {
                    (SLATES_ANNOTATION.to_string(), config.to_string())
                })
                .into_iter()
                .collect(),
        ),
    }
}

#[test]
fn a_ballot_style_without_slates_has_nothing_to_check() {
    let style = ballot_style(&[PRESIDENT, TRUSTEES], None);
    assert_eq!(check_ballot_style(&style, PATH), Vec::new());
}

#[test]
fn a_contest_outside_the_ballot_style_is_not_an_error() {
    let style = ballot_style(&[TRUSTEES], Some(&sound()));
    assert_eq!(check_ballot_style(&style, PATH), Vec::new());
}

#[test]
fn a_member_missing_from_a_present_contest_is_an_error() {
    let mut config = sound();
    config["slates"][1]["members"][TRUSTEES] = json!(["trustee-gone"]);
    let style = ballot_style(&[PRESIDENT, TRUSTEES], Some(&config));
    assert!(codes(&check_ballot_style(&style, PATH))
        .contains(&Code::DanglingReference));
}

#[test]
fn a_ballot_style_enforces_contest_limits() {
    let mut config = sound();
    config["slates"][0]["members"][PRESIDENT] =
        json!(["president-a", "president-b"]);
    let style = ballot_style(&[PRESIDENT, TRUSTEES], Some(&config));
    assert_eq!(
        codes(&check_ballot_style(&style, PATH)),
        vec![Code::ContestArithmetic]
    );
}

#[test]
fn a_ballot_style_with_an_unreadable_annotation_is_refused() {
    let style =
        ballot_style(&[PRESIDENT, TRUSTEES], Some(&json!("not slates")));
    assert!(!check_ballot_style(&style, PATH).is_empty());
}
