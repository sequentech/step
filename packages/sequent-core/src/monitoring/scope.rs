// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Which slice of an event a widget shows.
//!
//! The dashboard selectors — Region, Post, Country — narrow every widget that
//! follows them. The snapshot job counts each scope on its own and stores it
//! under [`ScopeKey::canonical`]; Harvest looks the same key up. Both sides
//! build the key here so they cannot disagree about it.
//!
//! The job counts these scopes, so these are the only keys there are: the
//! whole event, a region, a Post, a country, and a country within a region or
//! a Post. A Post already lies in one region, so choosing a Post drops the
//! region.

use super::config::{ScopeSelector, Widget};
use serde::{Deserialize, Serialize};

/// What the dashboard selectors hold. `None` is "All …".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeSelection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// Whether the Post comes from the page rather than from a selector.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PostPinning {
    /// The event's dashboard: Post is a selector like the others.
    #[default]
    Selectable,
    /// An election's own page: every widget shows that Post, whether or not
    /// it follows the Post selector.
    Pinned,
}

/// One scope the snapshot job counts.
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
pub struct ScopeKey {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// A widget's scope, and the dashboard selectors that could not apply to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetScope {
    pub key: ScopeKey,
    /// Selectors set on the dashboard that this widget's source cannot be
    /// narrowed by — Country, for a source that counts Posts. The portal
    /// says so rather than silently showing a wider figure.
    pub ignored: Vec<ScopeSelector>,
}

impl ScopeSelection {
    /// The scope `widget` shows: the selectors it follows and its source can
    /// be narrowed by. A pinned Post applies regardless.
    pub fn for_widget(
        &self,
        widget: &Widget,
        pinning: PostPinning,
    ) -> WidgetScope {
        let spec = widget.source.spec();
        let mut ignored = Vec::new();
        let mut pick = |selector: ScopeSelector, value: &Option<String>| {
            let value = value.clone()?;
            if !widget.follows(selector) {
                return None;
            }
            if !spec.builtin_dimensions.contains(&selector.dimension()) {
                ignored.push(selector);
                return None;
            }
            Some(value)
        };
        let post = match pinning {
            PostPinning::Pinned => self.post.clone(),
            PostPinning::Selectable => pick(ScopeSelector::Post, &self.post),
        };
        let region = match (&post, pinning) {
            (Some(_), _) | (None, PostPinning::Pinned) => None,
            (None, PostPinning::Selectable) => {
                pick(ScopeSelector::Region, &self.region)
            }
        };
        let country = pick(ScopeSelector::Country, &self.country);
        WidgetScope {
            key: ScopeKey {
                region,
                post,
                country,
            },
            ignored,
        }
    }
}

impl ScopeKey {
    pub fn event() -> Self {
        ScopeKey::default()
    }

    /// `event`, or `region=…`, `post=…`, `country=…` joined by `&`, values
    /// percent-encoded. Stable: stored as a key in snapshot manifests.
    pub fn canonical(&self) -> String {
        let parts: Vec<String> = [
            ("region", &self.region),
            ("post", &self.post),
            ("country", &self.country),
        ]
        .into_iter()
        .filter_map(|(name, value)| {
            value
                .as_ref()
                .map(|value| format!("{name}={}", encode(value)))
        })
        .collect();
        if parts.is_empty() {
            "event".to_string()
        } else {
            parts.join("&")
        }
    }
}

fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'~')
        {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod scope_tests;
