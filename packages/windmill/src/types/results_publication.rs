// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use sequent_core::ballot::{
    ResultsWebsiteAccess, ResultsWebsitePolicy, ResultsWebsiteStatus, ResultsWebsiteVisibilityScope,
};
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResultsManifestContest {
    pub election_id: String,
    pub contest_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_id: Option<String>,
    pub publication_state: ContestPublicationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positions: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResultsManifestArtifact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_path: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResultsManifestArtifacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_sqlite: Option<ResultsManifestArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub areas: Option<HashMap<String, ResultsManifestArtifact>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResultsManifestCustomCss {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub election_event: Option<String>,
    pub elections: HashMap<String, Option<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResultsPublicationManifestDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_public_path: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResultsPublicationDocuments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ResultsPublicationManifestDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_sqlite: Option<ResultsManifestArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_sqlite: Option<HashMap<String, ResultsManifestArtifact>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResultsPublicationManifest {
    pub schema_version: u32,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_ids: Vec<String>,
    pub route_scope: ResultsRouteScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_election_id: Option<String>,
    pub publication_id: String,
    pub tally_session_id: String,
    pub tally_session_execution_id: String,
    pub results_event_id: String,
    pub version: i32,
    pub access: ResultsWebsiteAccess,
    pub visibility_scope: ResultsWebsiteVisibilityScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_locale: Option<String>,
    pub available_languages: Vec<String>,
    pub title: HashMap<String, String>,
    pub custom_css: ResultsManifestCustomCss,
    pub contests: Vec<ResultsManifestContest>,
    pub artifacts: ResultsManifestArtifacts,
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
        if self.election_ids.is_empty() {
            return Err(anyhow!("At least one election must be selected"));
        }
        if self.contest_ids.is_empty() {
            return Err(anyhow!("At least one contest must be selected"));
        }

        match (self.route_scope, self.route_election_id.as_deref()) {
            (ResultsRouteScope::Event, Some(_)) => {
                return Err(anyhow!("Event route cannot include route_election_id"));
            }
            (ResultsRouteScope::Election, None) => {
                return Err(anyhow!("Election route requires route_election_id"));
            }
            (ResultsRouteScope::Election, Some(route_election_id))
                if self.election_ids.len() != 1
                    || self.election_ids.first().map(String::as_str) != Some(route_election_id) =>
            {
                return Err(anyhow!(
                    "Election route must publish exactly its route election"
                ));
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
    pub manifest: Option<ResultsPublicationManifest>,
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

    #[test]
    fn election_route_rejects_more_than_its_route_election() {
        let input = PublishResultsWebsiteInput {
            election_event_id: "event-id".to_string(),
            tally_session_id: "session-id".to_string(),
            tally_session_execution_id: "execution-id".to_string(),
            results_event_id: "results-event-id".to_string(),
            route_scope: ResultsRouteScope::Election,
            route_election_id: Some("election-1".to_string()),
            election_ids: vec!["election-1".to_string(), "election-2".to_string()],
            contest_ids: vec!["contest-1".to_string()],
            access: ResultsWebsiteAccess::Public,
            visibility_scope: ResultsWebsiteVisibilityScope::FullEvent,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn absent_manifest_artifacts_are_omitted_instead_of_serialized_as_null(
    ) -> serde_json::Result<()> {
        assert_eq!(
            serde_json::to_value(ResultsManifestArtifacts::default())?,
            serde_json::json!({})
        );
        Ok(())
    }
}
