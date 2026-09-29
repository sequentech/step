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
    let yaml = format!(
        "id: w\ntitle: W\nsource: {source}\n{follows}query: {{template: summary, measures: [posts]}}\nchart: {{charts: {{k: {{type: kpi, query: data, value: posts}}}}, rows: [k]}}\n"
    );
    let yaml = if source == "voter_turnout" {
        yaml.replace("posts", "voted")
    } else {
        yaml
    };
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
        .for_widget(&widget("voter_turnout", None), PostPinning::Selectable);
    assert_eq!(scope.key, ScopeKey::event());
    assert_eq!(scope.key.canonical(), "event");
    assert!(scope.ignored.is_empty());
}

#[test]
fn a_post_drops_the_region_it_lies_in() {
    let scope = selection(Some("NCR"), Some("e1"), Some("Spain"))
        .for_widget(&widget("voter_turnout", None), PostPinning::Selectable);
    assert_eq!(scope.key.region, None);
    assert_eq!(scope.key.post.as_deref(), Some("e1"));
    assert_eq!(scope.key.canonical(), "post=e1&country=Spain");
}

#[test]
fn a_widget_ignores_the_selectors_it_does_not_follow() {
    let scope = selection(Some("NCR"), None, Some("Spain")).for_widget(
        &widget("voter_turnout", Some("[region]")),
        PostPinning::Selectable,
    );
    assert_eq!(scope.key.canonical(), "region=NCR");
    assert!(
        scope.ignored.is_empty(),
        "not following is a choice, not a limit"
    );

    let whole = selection(Some("NCR"), Some("e1"), Some("Spain")).for_widget(
        &widget("voter_turnout", Some("[]")),
        PostPinning::Selectable,
    );
    assert_eq!(whole.key, ScopeKey::event());
}

#[test]
fn a_source_that_counts_posts_cannot_be_narrowed_by_country() {
    let scope = selection(None, None, Some("Spain"))
        .for_widget(&widget("poll_status", None), PostPinning::Selectable);
    assert_eq!(scope.key, ScopeKey::event());
    assert_eq!(scope.ignored, vec![ScopeSelector::Country]);
}

#[test]
fn a_pinned_post_applies_even_to_a_widget_that_does_not_follow_it() {
    let scope = selection(Some("NCR"), Some("e1"), Some("Spain")).for_widget(
        &widget("voter_turnout", Some("[country]")),
        PostPinning::Pinned,
    );
    assert_eq!(scope.key.post.as_deref(), Some("e1"));
    assert_eq!(scope.key.region, None);
    assert_eq!(scope.key.country.as_deref(), Some("Spain"));
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
