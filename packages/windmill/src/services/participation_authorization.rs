// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Bounded authorization for queued commands, persisted only in server-owned columns.
use anyhow::{bail, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::templates::{AudienceSelection, SendTemplateBody};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const PARTICIPATION_AUTHORIZATION: &str = "participation_authorization";
pub const REAUTHORIZE_PARTICIPATION: &str =
    "Participation authorization is missing or invalid; create a new authorized export or send";

#[derive(Debug)]
pub struct ParticipationReauthorization;
impl std::fmt::Display for ParticipationReauthorization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(REAUTHORIZE_PARTICIPATION)
    }
}
impl std::error::Error for ParticipationReauthorization {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParticipationCommand {
    VoterExport,
    EventArchive,
    BulkSend,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipationAuthorization {
    pub tenant_id: String,
    pub election_event_id: String,
    pub actor_id: String,
    pub command: ParticipationCommand,
    pub command_id: String,
    /// A snapshot, including an explicit empty set. Never an open-ended All.
    pub election_ids: Vec<String>,
}

impl ParticipationAuthorization {
    pub fn validate(
        &self,
        tenant_id: &str,
        event_id: &str,
        actor_id: &str,
        command: ParticipationCommand,
        command_id: &str,
    ) -> Result<()> {
        if self.tenant_id != tenant_id
            || self.election_event_id != event_id
            || self.actor_id != actor_id
            || self.actor_id.is_empty()
            || self.command != command
            || self.command_id != command_id
        {
            bail!(ParticipationReauthorization);
        }
        Ok(())
    }

    pub fn validate_election(&self, election_id: Option<&str>) -> Result<()> {
        if election_id.is_some_and(|id| !self.election_ids.iter().any(|allowed| allowed == id)) {
            bail!("Election outside the authorized participation snapshot");
        }
        Ok(())
    }
}

pub fn needs_participation(body: &SendTemplateBody) -> bool {
    matches!(
        body.audience_selection,
        Some(AudienceSelection::VOTED | AudienceSelection::NOT_VOTED)
    )
}

pub async fn export_authorization(
    tx: &Transaction<'_>,
    tenant_id: &str,
    task_id: &str,
) -> Result<ParticipationAuthorization> {
    let value: Option<Value> = tx.query_one(
        "SELECT participation_authorization FROM sequent_backend.tasks_execution WHERE tenant_id=$1 AND id=$2",
        &[&Uuid::parse_str(tenant_id)?, &Uuid::parse_str(task_id)?],
    ).await?.try_get(0)?;
    serde_json::from_value(value.context(ParticipationReauthorization)?)
        .context(ParticipationReauthorization)
}

pub struct AuthorizedBulkSend {
    pub body: SendTemplateBody,
    pub election_event_id: Option<Uuid>,
    pub actor_id: Option<String>,
    pub participation: Option<ParticipationAuthorization>,
}

/// Loads the immutable original command, never authority supplied in its broker body.
pub async fn authorized_bulk_send(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    send_id: &Uuid,
) -> Result<Option<AuthorizedBulkSend>> {
    let Some(row) = tx.query_opt(
        "SELECT event_payload,election_event_id,created_by,participation_authorization FROM sequent_backend.scheduled_event WHERE tenant_id=$1 AND id=$2 AND event_processor='SEND_TEMPLATE'",
        &[tenant_id,send_id],
    ).await? else { return Ok(None); };
    let payload: Option<Value> = row.try_get("event_payload")?;
    let body: SendTemplateBody =
        serde_json::from_value(payload.context("Missing bulk-send content")?)?;
    let election_event_id: Option<Uuid> = row.try_get("election_event_id")?;
    let actor_id: Option<String> = row.try_get("created_by")?;
    let grant: Option<Value> = row.try_get("participation_authorization")?;
    let participation = grant
        .map(serde_json::from_value::<ParticipationAuthorization>)
        .transpose()
        .context(ParticipationReauthorization)?;
    if needs_participation(&body) {
        let authorization = participation
            .as_ref()
            .context(ParticipationReauthorization)?;
        authorization.validate(
            &tenant_id.to_string(),
            &election_event_id
                .context(ParticipationReauthorization)?
                .to_string(),
            actor_id.as_deref().context(ParticipationReauthorization)?,
            ParticipationCommand::BulkSend,
            &send_id.to_string(),
        )?;
    }
    if body.send_id.as_deref() != Some(send_id.to_string().as_str()) {
        bail!("Stored send ID does not match its command");
    }
    Ok(Some(AuthorizedBulkSend {
        body,
        election_event_id,
        actor_id,
        participation,
    }))
}
