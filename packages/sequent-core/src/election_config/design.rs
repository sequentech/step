// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A ballot design's digest: what one area's voters are shown for one
//! election, hashed so that the same design gives the same digest in the
//! Election Architect, before import, and in windmill, at publication.
//!
//! The input is the ballot style the platform builds
//! ([`crate::ballot_style::create_ballot_style`]), not a second description of
//! a ballot. Three things in a style change on their way through an import
//! without the design changing, and the digest leaves them out:
//!
//! * Ids. The importer renumbers every UUID, so each id is replaced by the
//!   entity's stable key: an area's name (the voters CSV resolves areas by
//!   name, so names are unique), an election's, contest's or candidate's
//!   external id, and an image document's file name.
//! * The tenant, the event id, the public key, timestamps and the election's
//!   dates. The schedule is configuration, and the manifest covers it through
//!   the scheduled events file; the dates a style carries also record when
//!   voting actually started, which is not.
//! * The order of contests and candidates. A style lists them in database id
//!   order, which the renumbering shuffles; voters see `sort_order` and the
//!   ordering policy, which stay in.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::ballot::BallotStyle;
use crate::types::hasura::core::{Area, Candidate, Contest, Election};

use super::problem::{Code, Problem};

/// Keys dropped at every depth.
const VOLATILE_FIELDS: &[&str] = &[
    "tenant_id",
    "election_event_id",
    "public_key",
    "created_at",
    "last_updated_at",
    "election_dates",
];

/// Lists whose order is the database's rather than the voter's.
const UNORDERED_LISTS: &[&str] = &["contests", "candidates"];

/// The kinds of entity a style refers to by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind {
    Area,
    Election,
    Contest,
    Candidate,
    Document,
}

impl EntityKind {
    fn prefix(self) -> &'static str {
        match self {
            EntityKind::Area => "area",
            EntityKind::Election => "election",
            EntityKind::Contest => "contest",
            EntityKind::Candidate => "candidate",
            EntityKind::Document => "document",
        }
    }
}

/// Every id a style can carry, and the stable key that replaces it.
#[derive(Debug, Clone, Default)]
pub struct DesignKeys {
    by_id: BTreeMap<String, (EntityKind, String)>,
}

impl DesignKeys {
    /// The keys of an event's entities. An election, contest or candidate
    /// without an external id, or an area without a name, has no key that
    /// survives an import, so its designs can't be digested.
    pub fn of_entities(
        areas: &[Area],
        elections: &[Election],
        contests: &[Contest],
        candidates: &[Candidate],
    ) -> Result<Self, Problem> {
        let mut keys = DesignKeys::default();
        for area in areas {
            keys.insert(EntityKind::Area, &area.id, area.name.as_deref())?;
        }
        for election in elections {
            keys.insert(
                EntityKind::Election,
                &election.id,
                election.external_id.as_deref(),
            )?;
        }
        for contest in contests {
            keys.insert(
                EntityKind::Contest,
                &contest.id,
                contest.external_id.as_deref(),
            )?;
        }
        for candidate in candidates {
            keys.insert(
                EntityKind::Candidate,
                &candidate.id,
                candidate.external_id.as_deref(),
            )?;
        }
        Ok(keys)
    }

    /// An uploaded file a style may name, such as a candidate's photograph.
    pub fn with_document(mut self, id: &str, file_name: &str) -> Self {
        self.by_id.insert(
            id.to_string(),
            (EntityKind::Document, file_name.to_string()),
        );
        self
    }

    fn insert(
        &mut self,
        kind: EntityKind,
        id: &str,
        key: Option<&str>,
    ) -> Result<(), Problem> {
        match key.map(str::trim).filter(|key| !key.is_empty()) {
            Some(key) => {
                self.by_id.insert(id.to_string(), (kind, key.to_string()));
                Ok(())
            }
            None => Err(no_stable_key(kind, id)),
        }
    }

    /// The key of `id`, if it is one of the event's entities.
    pub fn key(&self, id: &str) -> Option<&str> {
        self.by_id.get(id).map(|(_, key)| key.as_str())
    }

    fn replacement(&self, text: &str) -> Option<String> {
        if let Some((kind, key)) = self.by_id.get(text) {
            return Some(format!("{}:{key}", kind.prefix()));
        }
        public_file_name(text)
            .map(|name| format!("{}:{name}", EntityKind::Document.prefix()))
    }
}

/// `tenant-<id>/document-<id>/<name>`, the bucket-relative path of an
/// uploaded file, as its file name. Both ids change on import.
fn public_file_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("tenant-")?;
    let (_, rest) = rest.split_once("/document-")?;
    let (_, name) = rest.split_once('/')?;
    (!name.is_empty() && !name.contains('/')).then_some(name)
}

fn no_stable_key(kind: EntityKind, id: &str) -> Problem {
    let what = match kind {
        EntityKind::Area => "name",
        _ => "external id",
    };
    Problem::error(
        Code::MissingField,
        format!("{}s", kind.prefix()),
        format!(
            "{} {id} has no {what}, so its ballot designs can't be identified \
             after an import",
            kind.prefix()
        ),
    )
    .id("design.no-stable-key")
    .detail("kind", kind.prefix())
    .detail("id", id)
}

/// One ballot design in a configuration: an area's ballot for an election.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallotDesign {
    /// The area's name.
    pub area: String,
    /// The election's external id.
    pub election: String,
    /// Starts at 1 and goes up by one each time the digest changes.
    pub version: u32,
    pub sha256: String,
}

/// A design's digest, before it is given a version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignDigest {
    pub area: String,
    pub election: String,
    pub sha256: String,
}

/// The digest of one ballot style.
pub fn ballot_design_digest(
    style: &BallotStyle,
    keys: &DesignKeys,
) -> Result<DesignDigest, Problem> {
    let key_of = |kind: EntityKind, id: &str| {
        keys.key(id)
            .map(str::to_string)
            .ok_or_else(|| no_stable_key(kind, id))
    };
    let area = key_of(EntityKind::Area, &style.area_id)?;
    let election = key_of(EntityKind::Election, &style.election_id)?;

    let value = serde_json::to_value(style).map_err(|error| {
        Problem::error(
            Code::InvalidValue,
            "ballot_styles",
            format!("a ballot style could not be read: {error}"),
        )
        .id("design.unreadable-style")
        .detail("reason", error)
    })?;
    let mut normalised = normalise(value, keys);
    if let Value::Object(map) = &mut normalised {
        // The style's own id: windmill mints a v4 one per publication.
        map.remove("id");
    }

    let mut bytes = Vec::new();
    write_sorted(&normalised, &mut bytes);
    Ok(DesignDigest {
        area,
        election,
        sha256: hex::encode(Sha256::digest(&bytes)),
    })
}

/// Every style's digest, ordered by area and election.
pub fn ballot_design_digests(
    styles: &[BallotStyle],
    keys: &DesignKeys,
) -> Result<Vec<DesignDigest>, Problem> {
    let mut digests = styles
        .iter()
        .map(|style| ballot_design_digest(style, keys))
        .collect::<Result<Vec<_>, _>>()?;
    digests.sort_by(|left, right| {
        (&left.area, &left.election).cmp(&(&right.area, &right.election))
    });
    Ok(digests)
}

/// The designs of a new revision, numbered against the previous revision's:
/// an unchanged design keeps its version, a changed one goes up by one and a
/// new one starts at 1.
pub fn versioned(
    previous: &[BallotDesign],
    current: Vec<DesignDigest>,
) -> Vec<BallotDesign> {
    current
        .into_iter()
        .map(|digest| {
            let before = previous.iter().find(|design| {
                design.area == digest.area && design.election == digest.election
            });
            let version = match before {
                Some(design) if design.sha256 == digest.sha256 => design.version,
                Some(design) => design.version.saturating_add(1),
                None => 1,
            };
            BallotDesign {
                area: digest.area,
                election: digest.election,
                version,
                sha256: digest.sha256,
            }
        })
        .collect()
}

fn normalise(value: Value, keys: &DesignKeys) -> Value {
    match value {
        Value::String(text) => {
            Value::String(keys.replacement(&text).unwrap_or(text))
        }
        Value::Array(items) => Value::Array(
            items.into_iter().map(|item| normalise(item, keys)).collect(),
        ),
        Value::Object(map) => {
            let mut out = Map::new();
            for (name, nested) in map {
                if VOLATILE_FIELDS.contains(&name.as_str()) {
                    continue;
                }
                let name = keys.replacement(&name).unwrap_or(name);
                let mut nested = normalise(nested, keys);
                if UNORDERED_LISTS.contains(&name.as_str()) {
                    sort_by_key(&mut nested);
                }
                out.insert(name, nested);
            }
            Value::Object(out)
        }
        other => other,
    }
}

/// Orders a list of entities by their (already replaced) id, and by their
/// whole content when two share one.
fn sort_by_key(list: &mut Value) {
    if let Value::Array(items) = list {
        let mut keyed: Vec<(String, Vec<u8>, Value)> = items
            .drain(..)
            .map(|item| {
                let id = item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let mut text = Vec::new();
                write_sorted(&item, &mut text);
                (id, text, item)
            })
            .collect();
        keyed.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
        items.extend(keyed.into_iter().map(|(_, _, item)| item));
    }
}

/// JSON with every object's keys in byte order. Unlike
/// [`crate::signing::canonical_json`] it accepts floats, which a ballot's
/// presentation may hold; `serde_json` writes each f64 one way.
fn write_sorted(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_sorted(item, out);
            }
            out.push(b']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by_key(|(name, _)| *name);
            out.push(b'{');
            for (index, (name, nested)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                out.extend(Value::String(name.clone()).to_string().as_bytes());
                out.push(b':');
                write_sorted(nested, out);
            }
            out.push(b'}');
        }
        scalar => out.extend(scalar.to_string().as_bytes()),
    }
}

#[cfg(test)]
#[path = "design_tests.rs"]
mod design_tests;
