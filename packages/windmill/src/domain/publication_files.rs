// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Layout of the private objects of a prepared ballot publication. Every
//! preparation attempt writes immutable objects under its own root.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fmt::Display;
use uuid::Uuid;

/// The `ballot_publication` annotation that holds the root of its objects.
pub const FILES_ANNOTATION: &str = "ballot_files_v1";

/// A published, non-deleted ballot style in a voter's area, with the object
/// root of its publication and the live policy of its election.
#[derive(Clone, Debug, PartialEq)]
pub struct PublishedBallotStyle {
    pub id: Uuid,
    pub election_id: Uuid,
    pub publication_id: Uuid,
    pub root: Option<String>,
    pub status: Option<Value>,
    pub num_allowed_revotes: Option<i64>,
    pub voting_channels: Option<Value>,
}

pub fn publication_root(tenant: Uuid, event: Uuid, publication: Uuid, attempt: Uuid) -> String {
    format!("tenant-{tenant}/event-{event}/publication-{publication}/{attempt}")
}

pub fn validate_publication_root(
    root: &str,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
) -> Result<()> {
    let prefix = format!("tenant-{tenant}/event-{event}/publication-{publication}/");
    let attempt = root
        .strip_prefix(&prefix)
        .context("Publication object scope mismatch")?;
    Uuid::parse_str(attempt).context("Invalid publication object version")?;
    Ok(())
}

pub fn event_key(root: &str) -> String {
    format!("{root}/event.json")
}

pub fn election_key(root: &str, election: impl Display) -> String {
    format!("{root}/election-{election}.json")
}

pub fn summary_key(root: &str, style: impl Display) -> String {
    format!("{root}/summary-{style}.json")
}

pub fn style_key(root: &str, style: impl Display) -> String {
    format!("{root}/style-{style}.json")
}

/// Split only the shared JSON value, preserving the exact original EML bytes.
pub fn split_event_presentation(eml: &str) -> Result<(String, String, String)> {
    #[derive(serde::Deserialize)]
    struct Envelope<'a> {
        #[serde(borrow)]
        election_event_presentation: Option<&'a serde_json::value::RawValue>,
    }
    let parsed: Envelope<'_> = serde_json::from_str(eml)?;
    let Some(raw) = parsed.election_event_presentation else {
        return Ok((eml.to_owned(), String::new(), String::new()));
    };
    let value = raw.get();
    let offset = value.as_ptr() as usize - eml.as_ptr() as usize;
    Ok((
        eml[..offset].to_owned(),
        value.to_owned(),
        eml[offset + value.len()..].to_owned(),
    ))
}
