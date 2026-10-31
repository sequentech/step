// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

const TENANT: &str = "tenant";
const EVENT: &str = "event";

fn election(id: &str, required: bool, initialized: bool) -> Election {
    serde_json::from_value(json!({
        "id": id,
        "tenant_id": TENANT,
        "election_event_id": EVENT,
        "presentation": {
            "initialization_report_policy": if required { "required" } else { "not-required" },
            "i18n": { "en": { "name": format!("Post {id}") } },
        },
        "initialization_report_generated": initialized,
    }))
    .expect("election")
}

fn area(id: &str, parent: Option<&str>) -> Area {
    serde_json::from_value(json!({
        "id": id,
        "tenant_id": TENANT,
        "election_event_id": EVENT,
        "name": format!("Country {id}"),
        "parent_id": parent,
    }))
    .expect("area")
}

fn contest(id: &str, election_id: &str) -> Contest {
    serde_json::from_value(json!({
        "id": id,
        "tenant_id": TENANT,
        "election_event_id": EVENT,
        "election_id": election_id,
    }))
    .expect("contest")
}

fn area_contest(area_id: &str, contest_id: &str) -> AreaContest {
    AreaContest {
        id: format!("{area_id}-{contest_id}"),
        area_id: area_id.to_string(),
        contest_id: contest_id.to_string(),
    }
}

fn event_with_scope(scope: Option<&str>) -> ElectionEvent {
    let mut presentation = json!({});
    if let Some(scope) = scope {
        presentation["lifecycle_policies"] = json!({ "initialization_scope": scope });
    }
    serde_json::from_value(json!({
        "id": EVENT,
        "tenant_id": TENANT,
        "name": "Event",
        "presentation": presentation,
        "is_archived": false,
        "encryption_protocol": "RSA_AES_HYBRID",
    }))
    .expect("election event")
}

fn styled(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(post, area)| (post.to_string(), area.to_string()))
        .collect()
}

fn ids(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

/// Posts p1 (countries a, b), p2 (country c) and p3 (country d), where p3
/// doesn't require its report; `initialized_areas` lists the rows.
fn state(posts: &[(&str, bool, bool)], initialized_areas: &[(&str, &str)]) -> EventInitialization {
    let elections: Vec<Election> = posts
        .iter()
        .map(|(id, required, initialized)| election(id, *required, *initialized))
        .collect();
    let areas = vec![
        area("a", None),
        area("b", None),
        area("c", None),
        area("d", None),
    ];
    let contests = vec![
        contest("k1", "p1"),
        contest("k2", "p2"),
        contest("k3", "p3"),
    ];
    let area_contests = vec![
        area_contest("a", "k1"),
        area_contest("b", "k1"),
        area_contest("c", "k2"),
        area_contest("d", "k3"),
    ];
    let rows: Vec<(String, Option<String>)> = initialized_areas
        .iter()
        .map(|(post, area)| (post.to_string(), Some(area.to_string())))
        .collect();
    let styled = styled(&[("p1", "a"), ("p1", "b"), ("p2", "c"), ("p3", "d")]);
    EventInitialization::build(
        &elections,
        &areas,
        &area_contests,
        &contests,
        &styled,
        &rows,
        "en",
    )
    .expect("state")
}

fn scopes(current: InitializationScope, published: Option<InitializationScope>) -> ScopeCopies {
    ScopeCopies { current, published }
}

use InitializationScope::{EVENT as EVENT_SCOPE, POST, POST_AND_COUNTRY};

#[test]
fn countries_of_a_post_are_its_areas_with_a_ballot_style() {
    // `region` groups `child` and holds the contest, but only `child` has a
    // ballot style: voters vote there, not in the grouping area.
    let areas = vec![
        area("a", None),
        area("b", None),
        area("region", None),
        area("child", Some("region")),
        area("other", None),
    ];
    let contests = vec![contest("k1", "p1"), contest("k2", "p2")];
    let area_contests = vec![
        area_contest("a", "k1"),
        area_contest("b", "k1"),
        area_contest("region", "k1"),
        area_contest("other", "k2"),
    ];
    let styles = styled(&[
        ("p1", "a"),
        ("p1", "b"),
        ("p1", "child"),
        ("p2", "other"),
        ("p2", "a"),
    ]);
    assert_eq!(
        post_area_ids(
            &areas,
            &area_contests,
            &contests,
            &["p1".to_string()],
            &styles
        )
        .unwrap(),
        ids(&["a", "b", "child"])
    );
    // A ballot style of another Post doesn't make an area a country of p2
    // unless p2's contests reach it.
    assert_eq!(
        post_area_ids(
            &areas,
            &area_contests,
            &contests,
            &["p2".to_string()],
            &styles
        )
        .unwrap(),
        ids(&["other"])
    );
}

#[test]
fn refusals_name_at_most_five_then_count_the_rest() {
    let names: Vec<String> = (1..=7).map(|n| format!("P{n}")).collect();
    assert_eq!(name_list(&names[..2]), "P1, P2");
    assert_eq!(name_list(&names), "P1, P2, P3, P4, P5 and 2 more");
}

#[test]
fn post_scope_needs_only_the_posts_own_report() {
    let event = state(&[("p1", true, false), ("p2", true, false)], &[]);
    assert_eq!(
        initialization_refusal(&scopes(POST, None), &event, "p1"),
        Some(TransitionRefusal::InitializationReportRequired)
    );
    let event = state(&[("p1", true, true), ("p2", true, false)], &[]);
    assert_eq!(
        initialization_refusal(&scopes(POST, None), &event, "p1"),
        None
    );
}

#[test]
fn a_post_that_does_not_require_its_report_never_waits() {
    let event = state(&[("p1", false, false), ("p2", true, false)], &[]);
    for scope in [POST, EVENT_SCOPE, POST_AND_COUNTRY] {
        assert_eq!(
            initialization_refusal(&scopes(scope, None), &event, "p1"),
            None
        );
    }
}

#[test]
fn event_scope_waits_for_every_post_that_requires_its_report() {
    let event = state(
        &[
            ("p1", true, true),
            ("p2", true, false),
            ("p3", false, false),
        ],
        &[],
    );
    assert_eq!(
        initialization_refusal(&scopes(EVENT_SCOPE, None), &event, "p1"),
        Some(TransitionRefusal::EventNotInitialized {
            post: "Post p1".to_string(),
            election_ids: vec!["p2".to_string()],
            names: vec!["Post p2".to_string()],
        })
    );
    // p3 doesn't require its report, so nobody waits for it.
    let event = state(
        &[("p1", true, true), ("p2", true, true), ("p3", false, false)],
        &[],
    );
    assert_eq!(
        initialization_refusal(&scopes(EVENT_SCOPE, None), &event, "p1"),
        None
    );
}

#[test]
fn post_and_country_scope_waits_for_every_country_of_the_post() {
    let event = state(&[("p1", true, true), ("p2", true, false)], &[("p1", "a")]);
    assert_eq!(
        initialization_refusal(&scopes(POST_AND_COUNTRY, None), &event, "p1"),
        Some(TransitionRefusal::CountriesNotInitialized {
            post: "Post p1".to_string(),
            area_ids: vec!["b".to_string()],
            names: vec!["Country b".to_string()],
        })
    );
    // Another Post's countries don't matter, nor its report.
    let event = state(
        &[("p1", true, true), ("p2", true, false)],
        &[("p1", "a"), ("p1", "b")],
    );
    assert_eq!(
        initialization_refusal(&scopes(POST_AND_COUNTRY, None), &event, "p1"),
        None
    );
    // A row of another Post for the same area doesn't count.
    let event = state(
        &[("p1", true, true), ("p2", true, true)],
        &[("p1", "a"), ("p2", "b")],
    );
    assert!(matches!(
        initialization_refusal(&scopes(POST_AND_COUNTRY, None), &event, "p1"),
        Some(TransitionRefusal::CountriesNotInitialized { .. })
    ));
}

#[test]
fn every_scope_needs_the_posts_own_report_first() {
    let event = state(
        &[("p1", true, false), ("p2", true, true)],
        &[("p1", "a"), ("p1", "b")],
    );
    for scope in [POST, EVENT_SCOPE, POST_AND_COUNTRY] {
        assert_eq!(
            initialization_refusal(&scopes(scope, None), &event, "p1"),
            Some(TransitionRefusal::InitializationReportRequired)
        );
    }
}

#[test]
fn both_copies_must_be_satisfied() {
    // p1 is initialized with all its countries; p2 isn't initialized.
    let event = state(
        &[("p1", true, true), ("p2", true, false)],
        &[("p1", "a"), ("p1", "b")],
    );
    // A looser current copy doesn't lift the published one.
    assert!(matches!(
        initialization_refusal(&scopes(POST, Some(EVENT_SCOPE)), &event, "p1"),
        Some(TransitionRefusal::EventNotInitialized { .. })
    ));
    // A stricter current copy applies at once.
    assert!(matches!(
        initialization_refusal(&scopes(EVENT_SCOPE, Some(POST)), &event, "p1"),
        Some(TransitionRefusal::EventNotInitialized { .. })
    ));
    // The two scopes add up.
    let event = state(&[("p1", true, true), ("p2", true, true)], &[("p1", "a")]);
    assert!(matches!(
        initialization_refusal(&scopes(EVENT_SCOPE, Some(POST_AND_COUNTRY)), &event, "p1"),
        Some(TransitionRefusal::CountriesNotInitialized { .. })
    ));
    let event = state(
        &[("p1", true, true), ("p2", true, true)],
        &[("p1", "a"), ("p1", "b")],
    );
    assert_eq!(
        initialization_refusal(&scopes(EVENT_SCOPE, Some(POST_AND_COUNTRY)), &event, "p1"),
        None
    );
}

#[test]
fn copies_report_what_they_ask_for() {
    assert_eq!(scopes(POST, None).scopes(), vec![POST]);
    assert!(!scopes(POST, Some(POST)).beyond_post());
    assert!(scopes(POST, Some(EVENT_SCOPE)).beyond_post());
    assert_eq!(
        scopes(EVENT_SCOPE, Some(POST_AND_COUNTRY)).scopes(),
        vec![EVENT_SCOPE, POST_AND_COUNTRY]
    );
    assert!(scopes(POST, Some(POST_AND_COUNTRY)).per_country());
    assert!(!scopes(EVENT_SCOPE, None).per_country());
}

#[test]
fn the_current_copy_is_read_from_the_event_presentation() {
    let read = |scope: Option<&str>| scope_copies_of(&event_with_scope(scope)).unwrap();
    assert_eq!(read(None), scopes(POST, None));
    assert_eq!(read(Some("event")), scopes(EVENT_SCOPE, None));
    assert_eq!(
        read(Some("post-and-country")),
        scopes(POST_AND_COUNTRY, None)
    );
    let unreadable: ElectionEvent = serde_json::from_value(json!({
        "id": EVENT,
        "tenant_id": TENANT,
        "name": "Event",
        "presentation": { "lifecycle_policies": { "initialization_scope": "country" } },
        "is_archived": false,
        "encryption_protocol": "RSA_AES_HYBRID",
    }))
    .unwrap();
    assert!(scope_copies_of(&unreadable).is_err());
}
