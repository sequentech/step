// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Typed run ownership and the JSON boundary consumed by both engine adapters.
use super::{config::Settings, files};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};

/// Server-assigned event identities, read back after import rather than guessed.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Event {
    pub election_event_id: String,
    pub election_id: String,
    pub realm: String,
    pub area_name: String,
    pub login_url: String,
}

/// Immutable worker inputs. No administrator credentials belong in this structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub settings: Settings,
    #[serde(flatten)]
    pub event: Event,
    pub style_id: Value,
    pub publication_version: Value,
    pub profile: Value,
    pub cast_query: String,
}
impl Input {
    /// Allocate shards with a short final tail, without per-voter scheduler objects.
    pub fn shards(&self) -> usize {
        self.settings
            .workload
            .count
            .div_ceil(self.settings.workload.shard_size)
    }

    /// Return the absolute first voter suffix and the finite size of one shard.
    pub fn bounds(&self, shard: usize) -> Result<(u64, usize)> {
        let w = &self.settings.workload;
        ensure!(shard < self.shards(), "Shard outside census");
        let offset = shard.checked_mul(w.shard_size).context("Shard overflow")?;
        Ok((w.start + offset as u64, w.shard_size.min(w.count - offset)))
    }

    /// Read the synthetic secret only when needed; never serialize it into a run.
    pub fn password(&self) -> Result<String> {
        let value = std::env::var(&self.settings.workload.password_env)
            .context("Set the configured synthetic password environment variable")?;
        ensure!(!value.is_empty(), "Synthetic password cannot be empty");
        Ok(value)
    }

    /// Validate host boundaries before authenticating or launching engine processes.
    pub fn validate(&self) -> Result<()> {
        self.settings.validate()?;
        ensure!(
            !self.event.election_event_id.is_empty() && !self.event.election_id.is_empty(),
            "Missing event identities"
        );
        let login = url::Url::parse(&self.event.login_url)?;
        let portal = url::Url::parse(&self.settings.target.portal_url)?;
        ensure!(
            login.origin() == portal.origin(),
            "Event login is outside the configured portal origin"
        );
        Ok(())
    }

    /// Keep the engine protocol flat while Rust reads the typed settings snapshot.
    pub fn wire(&self) -> Result<Value> {
        let mut wire = serde_json::to_value(self)?;
        let object = wire.as_object_mut().context("Input must be an object")?;
        for section in [
            serde_json::to_value(&self.settings.target)?,
            serde_json::to_value(&self.settings.workload)?,
        ] {
            object.extend(
                section
                    .as_object()
                    .context("Settings must be objects")?
                    .clone(),
            );
        }
        let target = &self.settings.target;
        let origins: BTreeSet<_> = [
            &target.portal_url,
            &target.keycloak_url,
            &target.graphql_url,
        ]
        .into_iter()
        .chain(target.storage_origins.iter())
        .map(|value| url::Url::parse(value).map(|url| url.origin().ascii_serialization()))
        .collect::<std::result::Result<_, _>>()?;
        object.insert("allowed_origins".into(), json!(origins));
        object.insert("vus".into(), json!(self.settings.workload.concurrency));
        object.insert(
            "runtime".into(),
            serde_json::to_value(&self.settings.runtime)?,
        );
        object.insert(
            "reporting".into(),
            serde_json::to_value(&self.settings.reporting)?,
        );
        let mut goals = serde_json::to_value(&self.settings.goals)?;
        if self.settings.min_casts_per_second > 0.0 {
            goals["min_casts_per_second"] = json!(self.settings.min_casts_per_second);
        }
        object.insert("goals".into(), goals);
        Ok(wire)
    }
    /// Freeze exactly the bytes whose digest workers verify before taking ownership.
    pub fn save(&self, path: &Path) -> Result<()> {
        files::save(path, &self.wire()?)
    }
}

/// Capture the portal's query at compile time; no browser recording is required.
fn query(source: &str) -> &str {
    source
        .split_once("gql`")
        .expect("Portal query starts with gql`")
        .1
        .split_once('`')
        .expect("Portal query closes its template")
        .0
        .trim()
}

/// Describe the fresh-session HTTP journey using the portal's own GraphQL selections.
pub fn protocol(settings: &Settings, event: Event) -> Input {
    let oidc = format!(
        "{}/realms/{}/protocol/openid-connect/",
        settings.target.keycloak_url.trim_end_matches('/'),
        event.realm
    );
    let mut steps = vec![
        json!({"kind":"auth", "url":format!("{oidc}auth"), "parameters":{
            "client_id":settings.workload.client_id, "redirect_uri":event.login_url,
            "response_type":"code", "response_mode":"fragment", "scope":"openid", "ui_locales":settings.workload.locale}}),
        json!({"kind":"login"}),
        json!({"kind":"token", "url":format!("{oidc}token")}),
        json!({"kind":"status", "url":settings.target.graphql_url, "payload":{
            "operationName":"GetVoterStatus", "query": query(include_str!("../../../voting-portal/src/queries/GetVoterStatus.ts")),
            "variables":{"electionEventId":event.election_event_id}}}),
    ];
    if settings.workload.mode == "vote" {
        for binding in ["event_url", "election_url", "summary_url", "style_url"] {
            steps.push(json!({"kind":"publication", "binding":binding}));
        }
        steps.push(json!({"kind":"cast"}));
    }
    for step in &mut steps {
        step["offset_ms"] = json!(0);
    }
    Input {
        settings: settings.clone(),
        event,
        style_id: Value::Null,
        publication_version: Value::Null,
        profile: json!({"steps":steps}),
        cast_query: query(include_str!(
            "../../../voting-portal/src/queries/InsertCastVote.ts"
        ))
        .into(),
    }
}
