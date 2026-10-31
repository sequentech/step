// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The two configurations every timezone and lifecycle behaviour is checked
//! under (VOTE-LIFECYCLE design §10), for database tests:
//!
//! - [`comelec_preset`]: the janitor's client preset
//!   (`external-bin/janitor/templates/COMELEC/lifecycle.json`), read as
//!   shipped: 104 Posts, 80 configured zones, `Asia/Manila` primary, log
//!   times in the primary. The list includes 27 synthetic additions and is
//!   not an authoritative Annex A list. Initialization per Post and country, unsigned
//!   scheduled closes run as the system.
//! - [`madrid_association`]: an association in Madrid with an office in the
//!   Canary Islands: `Europe/Madrid` primary, `Atlantic/Canary`, log times
//!   per election, the product's default policies.
//!
//! Include it with
//! `#[path = "support/lifecycle_configurations.rs"] mod lifecycle_configurations;`.
//! Tests derive their expectations from these values (and the tz database),
//! never from literal instants.

#![allow(dead_code)]

use serde::Deserialize;
use serde_json::{json, Value};

const COMELEC_PRESET: &str =
    include_str!("../../external-bin/janitor/templates/COMELEC/lifecycle.json");

/// An election of a configuration (a Post, or an office).
#[derive(Debug, Clone)]
pub struct Post {
    /// The name voters and admins see.
    pub name: String,
    /// What the schedule CSV names it by (`election_alias`).
    pub alias: String,
    /// The election's own zone; `None` uses the event's primary.
    pub time_zone: Option<String>,
}

/// An event configuration: its presentation and its elections.
#[derive(Debug, Clone)]
pub struct Configuration {
    pub label: &'static str,
    /// `presentation.timezones` (and `lifecycle_policies` when the
    /// configuration sets them), as stored on the election event.
    pub presentation: Value,
    pub posts: Vec<Post>,
}

impl Configuration {
    pub fn primary(&self) -> String {
        self.presentation["timezones"]["primary"]
            .as_str()
            .expect("a primary zone")
            .to_string()
    }

    pub fn configured(&self) -> Vec<String> {
        self.presentation["timezones"]["configured"]
            .as_array()
            .expect("configured zones")
            .iter()
            .map(|zone| zone.as_str().expect("a zone name").to_string())
            .collect()
    }

    /// The zone a Post's schedule is in: its own, else the primary.
    pub fn zone_of(&self, post: &Post) -> String {
        post.time_zone.clone().unwrap_or_else(|| self.primary())
    }

    /// The election presentation of a Post: its name and alias in English
    /// and its zone.
    pub fn election_presentation(&self, post: &Post) -> Value {
        json!({
            "language_conf": {"default_language_code": "en", "enabled_language_codes": ["en"]},
            "i18n": {"en": {"name": post.name, "alias": post.alias}},
            "timezone": post.time_zone,
        })
    }
}

#[derive(Deserialize)]
struct Preset {
    timezones: Value,
    lifecycle_policies: Value,
    posts: Vec<PresetPost>,
}

#[derive(Deserialize)]
struct PresetPost {
    post: String,
    timezone: String,
}

/// The janitor's COMELEC preset with its 104 Posts.
pub fn comelec_preset() -> Configuration {
    let preset: Preset = serde_json::from_str(COMELEC_PRESET).expect("the COMELEC preset");
    Configuration {
        label: "COMELEC preset",
        presentation: json!({
            "timezones": preset.timezones,
            "lifecycle_policies": preset.lifecycle_policies,
        }),
        posts: preset
            .posts
            .into_iter()
            .map(|post| Post {
                alias: format!("{} - General Election", post.post),
                name: post.post,
                time_zone: Some(post.timezone),
            })
            .collect(),
    }
}

/// The Madrid association: two councils' offices and a board, the Canary
/// Islands office in its own zone, the others in the primary.
pub fn madrid_association() -> Configuration {
    Configuration {
        label: "Madrid association",
        presentation: json!({
            "timezones": {
                "configured": ["Europe/Madrid", "Atlantic/Canary"],
                "primary": "Europe/Madrid",
                "logs": "election",
            },
        }),
        posts: vec![
            Post {
                name: "Council: Central office".to_string(),
                alias: "council-central".to_string(),
                time_zone: None,
            },
            Post {
                name: "Council: Canary Islands office".to_string(),
                alias: "council-canary".to_string(),
                time_zone: Some("Atlantic/Canary".to_string()),
            },
            Post {
                name: "Pension board".to_string(),
                alias: "pension-board".to_string(),
                time_zone: None,
            },
        ],
    }
}
