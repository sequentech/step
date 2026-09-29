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
use sha2::{Digest, Sha256};

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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PostPinning {
    /// The event's dashboard: Post is a selector like the others.
    #[default]
    Selectable,
    /// An election's own page: every widget shows this Post, whether or not
    /// it follows the Post selector, and the Post selector is not read.
    Pinned(String),
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
    /// Selectors set on the dashboard — or a Post pinned by the page — that
    /// this widget's source cannot be narrowed by: Country, for a source
    /// that counts Posts. The portal says so rather than silently showing a
    /// wider figure.
    pub ignored: Vec<ScopeSelector>,
}

impl ScopeSelection {
    /// The scope `widget` shows: the selectors it follows and its source can
    /// be narrowed by. A pinned Post applies whether or not the widget
    /// follows the Post selector. An empty value is "All".
    pub fn for_widget(
        &self,
        widget: &Widget,
        pinning: &PostPinning,
    ) -> WidgetScope {
        let spec = widget.source.spec();
        let mut ignored = Vec::new();
        let mut narrow = |selector: ScopeSelector,
                          value: Option<&String>,
                          followed: bool| {
            let value = value.filter(|value| !value.is_empty())?;
            if !followed {
                return None;
            }
            if !spec.builtin_dimensions.contains(&selector.dimension()) {
                ignored.push(selector);
                return None;
            }
            Some(value.clone())
        };
        let post = match pinning {
            PostPinning::Pinned(post) => {
                narrow(ScopeSelector::Post, Some(post), true)
            }
            PostPinning::Selectable => narrow(
                ScopeSelector::Post,
                self.post.as_ref(),
                widget.follows(ScopeSelector::Post),
            ),
        };
        // A Post lies in one region, so choosing one drops the region; on a
        // Post's own page the region selector is not offered.
        let region = match (&post, pinning) {
            (Some(_), _) | (None, PostPinning::Pinned(_)) => None,
            (None, PostPinning::Selectable) => narrow(
                ScopeSelector::Region,
                self.region.as_ref(),
                widget.follows(ScopeSelector::Region),
            ),
        };
        let country = narrow(
            ScopeSelector::Country,
            self.country.as_ref(),
            widget.follows(ScopeSelector::Country),
        );
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
    /// percent-encoded. Stable: snapshot figures are stored under it.
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

/// The key of a set of elections a viewer may see: the first 16 hex digits
/// of the SHA-256 of their ids, lowercase, ascending and comma-separated,
/// each id once. Election ids are UUIDs, whose lowercase text sorts as their
/// bytes do, so the monitoring tables can check a stored key against its ids.
pub fn election_set_key<I, S>(ids: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut ids: Vec<String> = ids
        .into_iter()
        .map(|id| id.as_ref().to_ascii_lowercase())
        .collect();
    ids.sort();
    ids.dedup();
    let digest = Sha256::digest(ids.join(",").as_bytes());
    hex::encode(digest)[..16].to_string()
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
