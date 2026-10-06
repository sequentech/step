// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::report_variables::{
    calc_voters_turnout, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    ExecutionAnnotations,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::services::cast_votes::CastVoteStatus;
use crate::services::users::{count_keycloak_enabled_users, count_keycloak_users, ListUsersFilter};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Local;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::translations::Name;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::Election;
use serde::{Deserialize, Serialize};
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ParticipationReportScope {
    ElectionEvent,
    Election,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParticipationReportStats {
    pub scope: ParticipationReportScope,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
    pub election_event_name: String,
    pub election_name: Option<String>,
    pub generated_at: String,
    pub generated_timezone: String,
    pub total_voters: i64,
    pub voted_voters: i64,
    pub participation_percentage: f64,
    pub votes_count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub execution_annotations: ExecutionAnnotations,
    pub participation: ParticipationReportStats,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct ParticipationReportTemplate {
    ids: ReportOrigins,
}

#[derive(Debug)]
struct CastVoteParticipationStats {
    voted_voters: i64,
    votes_count: i64,
}

fn get_authorized_election_alias(election: &Election, language: &str) -> Option<String> {
    election
        .presentation
        .as_ref()
        .and_then(|presentation| presentation.get("i18n"))
        .and_then(|i18n| i18n.get(language))
        .and_then(|translation| translation.get("alias"))
        .and_then(|alias| alias.as_str())
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .map(str::to_string)
}

impl ParticipationReportTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        ParticipationReportTemplate { ids }
    }

    #[instrument(skip(hasura_transaction), err)]
    async fn get_cast_vote_stats(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> Result<CastVoteParticipationStats> {
        let tenant_uuid: Uuid = parse_uuid_v4(&self.ids.tenant_id)
            .with_context(|| "Error parsing tenant_id as UUID")?;
        let election_event_uuid: Uuid = parse_uuid_v4(&self.ids.election_event_id)
            .with_context(|| "Error parsing election_event_id as UUID")?;
        let election_uuid: Option<Uuid> = self
            .ids
            .election_id
            .as_deref()
            .map(parse_uuid_v4)
            .transpose()
            .with_context(|| "Error parsing election_id as UUID")?;

        let status = CastVoteStatus::Valid.to_string();
        let statement = hasura_transaction
            .prepare(
                r#"
                SELECT
                    COUNT(*) AS votes_count,
                    COUNT(DISTINCT voter_id_string) AS voted_voters
                FROM
                    sequent_backend.cast_vote
                WHERE
                    tenant_id = $1
                    AND election_event_id = $2
                    AND ($3::uuid IS NULL OR election_id = $3::uuid)
                    AND status = $4
                "#,
            )
            .await
            .map_err(|err| anyhow!("Error preparing participation report query: {err}"))?;

        let row: Row = hasura_transaction
            .query_one(
                &statement,
                &[&tenant_uuid, &election_event_uuid, &election_uuid, &status],
            )
            .await
            .map_err(|err| anyhow!("Error running participation report query: {err}"))?;

        Ok(CastVoteParticipationStats {
            voted_voters: row.get("voted_voters"),
            votes_count: row.get("votes_count"),
        })
    }

    #[instrument(skip(hasura_transaction, keycloak_transaction), err)]
    async fn get_total_voters(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        realm: &str,
        election_alias: Option<String>,
    ) -> Result<i64> {
        match &self.ids.election_id {
            Some(election_id) => {
                let filter = ListUsersFilter {
                    tenant_id: self.ids.tenant_id.clone(),
                    election_event_id: Some(self.ids.election_event_id.clone()),
                    election_id: Some(election_id.clone()),
                    realm: realm.to_string(),
                    enabled: Some(true),
                    authorized_to_election_alias: election_alias,
                    ..ListUsersFilter::default()
                };

                Ok(
                    count_keycloak_users(hasura_transaction, keycloak_transaction, filter).await?
                        as i64,
                )
            }
            None => count_keycloak_enabled_users(keycloak_transaction, realm).await,
        }
    }
}

#[async_trait]
impl TemplateRenderer for ParticipationReportTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::PARTICIPATION_REPORT
    }

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_election_id(&self) -> Option<String> {
        self.ids.election_id.clone()
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn base_name(&self) -> String {
        "participation_report".to_string()
    }

    fn prefix(&self) -> String {
        format!("participation_report_{}", rand::random::<u64>())
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let election_event = get_election_event_by_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .with_context(|| "Error getting election event for participation report")?;
        let event_language = election_event.get_default_language();
        let election_event_name = election_event.get_name(&event_language);
        let realm = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);

        let election = match &self.ids.election_id {
            Some(election_id) => Some(
                get_election_by_id(
                    hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    election_id,
                )
                .await
                .with_context(|| "Error getting election for participation report")?
                .ok_or_else(|| anyhow!("Election {election_id} not found"))?,
            ),
            None => None,
        };
        let election_language = election
            .as_ref()
            .map(|election| election.get_default_language())
            .unwrap_or_else(|| event_language.clone());
        let election_name = election
            .as_ref()
            .map(|election| election.get_name(&election_language));
        let election_alias = election
            .as_ref()
            .and_then(|election| get_authorized_election_alias(election, &election_language));

        let cast_vote_stats = self.get_cast_vote_stats(hasura_transaction).await?;
        let total_voters = self
            .get_total_voters(
                hasura_transaction,
                keycloak_transaction,
                &realm,
                election_alias,
            )
            .await?;
        let participation_percentage =
            calc_voters_turnout(cast_vote_stats.voted_voters, total_voters)?.unwrap_or(0.0);
        let generated_at = get_date_and_time();

        Ok(UserData {
            execution_annotations: ExecutionAnnotations {
                date_printed: generated_at.clone(),
                report_hash: get_report_hash(&ReportType::PARTICIPATION_REPORT.to_string()).await?,
                app_version: get_app_version(),
                software_version: get_app_version(),
                app_hash: get_app_hash(),
                executer_username: self.ids.executer_username.clone(),
                results_hash: None,
            },
            participation: ParticipationReportStats {
                scope: if self.ids.election_id.is_some() {
                    ParticipationReportScope::Election
                } else {
                    ParticipationReportScope::ElectionEvent
                },
                tenant_id: self.ids.tenant_id.clone(),
                election_event_id: self.ids.election_event_id.clone(),
                election_id: self.ids.election_id.clone(),
                election_event_name,
                election_name,
                generated_at,
                generated_timezone: Local::now().format("%:z").to_string(),
                total_voters,
                voted_voters: cast_vote_stats.voted_voters,
                participation_percentage,
                votes_count: cast_vote_stats.votes_count,
            },
        })
    }

    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}
