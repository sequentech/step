// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use sequent_core::ballot::{
    ResultsWebsiteAccess, ResultsWebsitePolicy, ResultsWebsiteStatus, ResultsWebsiteVisibilityScope,
};
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{AsRefStr, Display, EnumString};

#[derive(
    AsRefStr, Clone, Copy, Debug, Deserialize, Display, EnumString, Eq, PartialEq, Serialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResultsRouteScope {
    Event,
    Election,
}

#[derive(
    AsRefStr, Clone, Copy, Debug, Deserialize, Display, EnumString, Eq, PartialEq, Serialize,
)]
pub enum ResultsPublicationStatus {
    Publishing,
    Published,
    Failed,
    Revoked,
    Superseded,
}

#[derive(
    AsRefStr, Clone, Copy, Debug, Deserialize, Display, EnumString, Eq, PartialEq, Serialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ContestPublicationState {
    Published,
    NotPublished,
}

pub fn validate_access_visibility(
    access: ResultsWebsiteAccess,
    visibility_scope: ResultsWebsiteVisibilityScope,
) -> Result<()> {
    if access == ResultsWebsiteAccess::Public
        && visibility_scope != ResultsWebsiteVisibilityScope::FullEvent
    {
        return Err(anyhow!("Public results must use full_event visibility"));
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishResultsWebsiteInput {
    pub election_event_id: String,
    pub tally_session_id: String,
    pub tally_session_execution_id: String,
    pub results_event_id: String,
    pub route_scope: ResultsRouteScope,
    pub route_election_id: Option<String>,
    pub election_ids: Vec<String>,
    pub contest_ids: Vec<String>,
    pub access: ResultsWebsiteAccess,
    pub visibility_scope: ResultsWebsiteVisibilityScope,
}

impl PublishResultsWebsiteInput {
    pub fn validate(&self) -> Result<()> {
        if self.contest_ids.is_empty() {
            return Err(anyhow!("At least one contest must be selected"));
        }

        match (self.route_scope, self.route_election_id.is_some()) {
            (ResultsRouteScope::Event, true) => {
                return Err(anyhow!("Event route cannot include route_election_id"));
            }
            (ResultsRouteScope::Election, false) => {
                return Err(anyhow!("Election route requires route_election_id"));
            }
            _ => {}
        }

        validate_access_visibility(self.access, self.visibility_scope)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishResultsWebsiteOutput {
    pub publication_id: String,
    pub task_execution_id: String,
    pub publication_status: ResultsPublicationStatus,
    pub task_execution: TasksExecution,
    pub error_msg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigureResultsWebsitePolicyInput {
    pub election_event_id: String,
    pub status: ResultsWebsiteStatus,
    pub access: ResultsWebsiteAccess,
    pub visibility_scope: ResultsWebsiteVisibilityScope,
}

impl ConfigureResultsWebsitePolicyInput {
    pub fn validate(&self) -> Result<()> {
        validate_access_visibility(self.access, self.visibility_scope)
    }

    pub fn policy(&self) -> ResultsWebsitePolicy {
        ResultsWebsitePolicy {
            status: self.status,
            access: self.access,
            visibility_scope: self.visibility_scope,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigureResultsWebsitePolicyOutput {
    pub election_event_id: String,
    pub status: ResultsWebsiteStatus,
    pub access: ResultsWebsiteAccess,
    pub visibility_scope: ResultsWebsiteVisibilityScope,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResolveResultsPublicationInput {
    pub ee_id: String,
    pub election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResolveResultsPublicationOutput {
    pub tenant_id: String,
    pub election_event_id: String,
    pub access: ResultsWebsiteAccess,
    pub route_scope: ResultsRouteScope,
    pub election_ids: Vec<String>,
    pub publication_id: String,
    pub manifest_public_path: Option<String>,
    pub manifest_url: Option<String>,
    pub manifest: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchResultsArtifactInput {
    pub election_event_id: String,
    pub election_id: Option<String>,
    pub publication_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchResultsArtifactOutput {
    pub urls: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeResultsPublicationInput {
    pub election_event_id: String,
    pub publication_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeResultsPublicationOutput {
    pub publication_id: String,
    pub publication_status: ResultsPublicationStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshResultsPublicationIndexInput {
    pub election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshResultsPublicationIndexOutput {
    pub election_event_id: String,
    pub results_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_enums_reject_unknown_values() {
        assert!(serde_json::from_str::<ResultsWebsiteStatus>("\"unknown\"").is_err());
        assert!(serde_json::from_str::<ResultsRouteScope>("\"unknown\"").is_err());
    }

    #[test]
    fn public_area_based_policy_is_rejected() {
        let input = ConfigureResultsWebsitePolicyInput {
            election_event_id: "event-id".to_string(),
            status: ResultsWebsiteStatus::Enabled,
            access: ResultsWebsiteAccess::Public,
            visibility_scope: ResultsWebsiteVisibilityScope::AreaBased,
        };

        assert!(input.validate().is_err());
    }
}
