// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::monitoring::policy::parse_widget;

fn widget(source: &str, follows: Option<&str>) -> Widget {
    let follows = follows
        .map(|list| format!("follows: {list}\n"))
        .unwrap_or_default();
    let measure = source
        .parse::<crate::monitoring::sources::DataSourceId>()
        .expect("source")
        .spec()
        .measures[0];
    let yaml = format!(
        "id: w\ntitle: W\nsource: {source}\n{follows}query: {{template: summary, measures: [{measure}]}}\nchart: {{charts: {{k: {{type: kpi, query: data, value: {measure}}}}}, rows: [k]}}\n"
    );
    let parsed = parse_widget(&yaml);
    parsed
        .value
        .unwrap_or_else(|| panic!("refused:\n{}", parsed.report))
}

fn selection(
    region: Option<&str>,
    post: Option<&str>,
    country: Option<&str>,
) -> ScopeSelection {
    ScopeSelection {
        region: region.map(str::to_string),
        post: post.map(str::to_string),
        country: country.map(str::to_string),
    }
}

#[test]
fn nothing_selected_is_the_whole_event() {
    let scope = selection(None, None, None)
        .for_widget(&widget("voter_turnout", None), &PostPinning::Selectable);
    assert_eq!(scope.key, ScopeKey::event());
    assert_eq!(scope.key.canonical(), "event");
    assert!(scope.ignored.is_empty());
}

#[test]
fn a_post_drops_the_region_it_lies_in() {
    let scope = selection(Some("NCR"), Some("e1"), Some("Spain"))
        .for_widget(&widget("voter_turnout", None), &PostPinning::Selectable);
    assert_eq!(scope.key.region, None);
    assert_eq!(scope.key.post.as_deref(), Some("e1"));
    assert_eq!(scope.key.canonical(), "post=e1&country=Spain");
}

#[test]
fn a_widget_ignores_the_selectors_it_does_not_follow() {
    let scope = selection(Some("NCR"), None, Some("Spain")).for_widget(
        &widget("voter_turnout", Some("[region]")),
        &PostPinning::Selectable,
    );
    assert_eq!(scope.key.canonical(), "region=NCR");
    assert!(
        scope.ignored.is_empty(),
        "not following is a choice, not a limit"
    );

    let whole = selection(Some("NCR"), Some("e1"), Some("Spain")).for_widget(
        &widget("voter_turnout", Some("[]")),
        &PostPinning::Selectable,
    );
    assert_eq!(whole.key, ScopeKey::event());
}

#[test]
fn a_source_that_counts_posts_cannot_be_narrowed_by_country() {
    let scope = selection(None, None, Some("Spain"))
        .for_widget(&widget("poll_status", None), &PostPinning::Selectable);
    assert_eq!(scope.key, ScopeKey::event());
    assert_eq!(scope.ignored, vec![ScopeSelector::Country]);
}

#[test]
fn a_pinned_post_applies_even_to_a_widget_that_does_not_follow_it() {
    let scope = selection(Some("NCR"), Some("e9"), Some("Spain")).for_widget(
        &widget("voter_turnout", Some("[country]")),
        &PostPinning::Pinned("e1".into()),
    );
    assert_eq!(
        scope.key.post.as_deref(),
        Some("e1"),
        "the page's Post, not whatever the selector held"
    );
    assert_eq!(scope.key.region, None);
    assert_eq!(scope.key.country.as_deref(), Some("Spain"));
}

#[test]
fn a_pinned_post_is_ignored_by_a_source_without_posts_and_says_so() {
    let scope = selection(None, None, None).for_widget(
        &widget("attack_detections", None),
        &PostPinning::Pinned("e1".into()),
    );
    assert_eq!(scope.key, ScopeKey::event());
    assert_eq!(scope.ignored, vec![ScopeSelector::Post]);
}

#[test]
fn an_empty_value_means_all() {
    let scope = selection(Some(""), Some(""), Some(""))
        .for_widget(&widget("voter_turnout", None), &PostPinning::Selectable);
    assert_eq!(scope.key, ScopeKey::event());
}

#[test]
fn a_country_within_a_region_is_its_own_scope() {
    let scope = selection(Some("NCR"), None, Some("Spain"))
        .for_widget(&widget("voter_turnout", None), &PostPinning::Selectable);
    assert_eq!(scope.key.canonical(), "region=NCR&country=Spain");
}

#[test]
fn access_security_cannot_be_narrowed_by_country() {
    let scope = selection(Some("NCR"), None, Some("Spain"))
        .for_widget(&widget("access_security", None), &PostPinning::Selectable);
    assert_eq!(scope.key.canonical(), "region=NCR");
    assert_eq!(scope.ignored, vec![ScopeSelector::Country]);
}

#[test]
fn scope_keys_serialize_without_absent_parts() {
    let key = ScopeKey {
        region: None,
        post: Some("e1".into()),
        country: None,
    };
    assert_eq!(serde_json::to_string(&key).unwrap(), r#"{"post":"e1"}"#);
}

#[test]
fn values_are_encoded_so_keys_cannot_collide() {
    let tricky = ScopeKey {
        region: Some("A&country=B".into()),
        post: None,
        country: None,
    };
    let plain = ScopeKey {
        region: Some("A".into()),
        post: None,
        country: Some("B".into()),
    };
    assert_ne!(tricky.canonical(), plain.canonical());
    assert_eq!(tricky.canonical(), "region=A%26country%3DB");
    assert_eq!(
        ScopeKey {
            region: None,
            post: None,
            country: Some("España / Madrid".into())
        }
        .canonical(),
        "country=Espa%C3%B1a%20%2F%20Madrid"
    );
}

#[test]
fn a_set_of_elections_is_keyed_by_its_ids_whatever_their_order_or_case() {
    let key = election_set_key([
        "B0EEBC99-9C0B-4EF8-BB6D-6BB9BD380A12",
        "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
    ])
    .unwrap();
    // SHA-256 of "a0eebc99-…-6bb9bd380a11,b0eebc99-…-6bb9bd380a12".
    assert_eq!(key, "b060af5b18c65887");
    assert_eq!(
        election_set_key([
            "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
            "b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12",
            "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
        ]),
        Ok(key),
        "an id listed twice is one election"
    );
    // A viewer allowed no election still has a set: the empty one.
    assert_eq!(
        election_set_key(Vec::<String>::new()),
        Ok("e3b0c44298fc1c14".to_string())
    );
}

#[test]
fn only_a_hyphenated_uuid_is_an_election_id() {
    for id in [
        "{a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11}",
        "a0eebc999c0b4ef8bb6d6bb9bd380a11",
        " a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
        "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a1g",
        "a0eebc99-9c0b-4ef8-bb6d_6bb9bd380a11",
        "",
    ] {
        assert_eq!(
            election_set_key([id]),
            Err(NotAnElectionId(id.to_string())),
            "{id:?}"
        );
    }
}

#[test]
fn a_canonical_key_reads_back_as_the_key_it_names() {
    let keys = [
        ScopeKey::event(),
        ScopeKey {
            region: Some("Asia Pacific".into()),
            ..ScopeKey::default()
        },
        ScopeKey {
            region: Some("Europe".into()),
            country: Some("Côte d'Ivoire".into()),
            ..ScopeKey::default()
        },
        ScopeKey {
            post: Some("0b6f9c52-8f7a-4d3e-9b1a-2c4d6e8f0a1b".into()),
            country: Some("Spain/Madrid".into()),
            ..ScopeKey::default()
        },
    ];
    for key in keys {
        assert_eq!(
            ScopeKey::from_canonical(&key.canonical()),
            Some(key.clone())
        );
    }
    for text in [
        "",
        "region=",
        "country=a&region=b",
        "region=a&region=b",
        "town=x",
        "region=%ZZ",
        "region=a b",
    ] {
        assert_eq!(ScopeKey::from_canonical(text), None, "{text:?}");
    }
}
