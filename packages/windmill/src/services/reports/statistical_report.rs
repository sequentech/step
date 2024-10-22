// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_fill_up_rate,
    generate_total_number_of_expected_votes_for_contest, generate_total_number_of_under_votes,
    generate_voters_turnout, get_date_and_time,
    get_election_contests_area_results_and_total_ballot_counted,
    get_total_number_of_registered_voters_for_country,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::results_area_contest::ResultsAreaContest;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::Contest;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Struct returned by the API call for manual verification URL
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatisticalReportOutput {
    pub link: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub election_title: String,
    pub election_date: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: i64,
    pub elective_positions: Vec<ReportContestData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReportContestData {
    pub elective_position: String,
    pub total_expected: i64,
    pub total_position: i64,
    pub total_undevotes: i64,
    pub fill_up_rate: i64,
}

/// Implementation of TemplateRenderer for Manual Verification
#[derive(Debug)]
pub struct StatisticalReportTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
}

#[async_trait]
impl TemplateRenderer for StatisticalReportTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "statistical_report".to_string()
    }

    fn get_election_id(&self) -> Option<String> {
        Some(self.election_id.clone())
    }

    fn prefix(&self) -> String {
        format!("statistical_report_{}", self.election_id)
    }

    fn get_report_type() -> ReportType {
        ReportType::STATISTICAL_REPORT
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Statistical Report".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument]
    async fn prepare_user_data(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
        keycloak_transaction: Option<&Transaction<'_>>,
    ) -> Result<Self::UserData> {
        let tenant_id = self.get_tenant_id();
        let election_event_id = self.get_election_event_id();
        let election_id = self.get_election_id().unwrap();

        let realm = get_event_realm(&tenant_id, &election_event_id);
        let date_printed = get_date_and_time();

        let election = if let Some(transaction) = hasura_transaction {
            get_election_by_id(&transaction, &tenant_id, &election_event_id, &election_id)
                .await
                .map_err(|err| anyhow!("Error getting election by id: {err}"))?
                .ok_or_else(|| anyhow!("Election not found"))?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        let election_title = election.name.clone();
        let election_date = election.created_at.clone().unwrap().to_string();

        let election_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election data {err}"))?;

        let registered_voters = if let Some(transaction) = keycloak_transaction {
            get_total_number_of_registered_voters_for_country(
                &transaction,
                &realm,
                &election_data.country,
            )
            .await
            .map_err(|err| {
                anyhow!("Error getting total number of registered voters for country: {err}")
            })?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        let (ballots_counted, results_area_contests, contests) =
            if let Some(transaction) = hasura_transaction {
                get_election_contests_area_results_and_total_ballot_counted(
                    &transaction,
                    &tenant_id,
                    &election_event_id,
                    &election_id,
                )
                .await
                .map_err(|err| {
                    anyhow!("Error getting election contest, results, and counted ballots: {err}")
                })?
            } else {
                return Err(anyhow::anyhow!("Transaction is missing"));
            };

        let voters_turnout = generate_voters_turnout(&ballots_counted, &registered_voters)
            .await
            .map_err(|err| anyhow!("Error generate voters turnout {err}"))?;

        let mut elective_positions: Vec<ReportContestData> = vec![];

        for (contest) in contests.clone() {
            let results_area_contest = results_area_contests
                .iter()
                .find(|rac| rac.contest_id == contest.id)
                .unwrap();
            let contest_result_data =
                if let (Some(hasura_transaction), Some(keycloak_transaction)) =
                    (hasura_transaction, keycloak_transaction)
                {
                    generate_contest_results_data(
                        &hasura_transaction,   // Pass the unwrapped hasura transaction reference
                        &keycloak_transaction, // Pass the unwrapped keycloak transaction reference
                        &realm,
                        &tenant_id,
                        &election_event_id,
                        &contest,
                        &results_area_contest,
                    )
                    .await
                    .map_err(|err| {
                        anyhow!(
                            "Error generating contest results data for contest: {} {err}",
                            &contest.id
                        )
                    })?
                } else {
                    return Err(anyhow::anyhow!(
                        "Transaction(s) missing: Hasura or Keycloak transaction is missing"
                    ));
                };
            elective_positions.push(contest_result_data);
        }

        Ok(UserData {
            date_printed,
            election_title,
            election_date,
            post: election_data.post.clone(),
            country: election_data.country.clone(),
            voting_center: election_data.voting_center.clone(),
            precinct_code: election_data.clustered_precinct_id.clone(),
            registered_voters,
            ballots_counted,
            voters_turnout,
            elective_positions,
        })
    }

    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let temp_val: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: temp_val.to_string(),
        })
    }
}

/// Function to generate the manual verification report using the TemplateRenderer
#[instrument(err)]
pub async fn generate_statistical_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    hasura_transaction: Option<&Transaction<'_>>,
    keycloak_transaction: Option<&Transaction<'_>>,
) -> Result<()> {
    let template = StatisticalReportTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            false,
            None,
            None,
            GenerateReportMode::REAL,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}

//generate data for specific contest
#[instrument(err, skip_all)]
pub async fn generate_contest_results_data(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    tenant_id: &str,
    election_event_id: &str,
    contest: &Contest,
    results_area_contest: &ResultsAreaContest,
) -> Result<ReportContestData> {
    let elective_position = contest.name.clone().unwrap();

    let total_expected = generate_total_number_of_expected_votes_for_contest(
        &hasura_transaction,
        &keycloak_transaction,
        &realm,
        &tenant_id,
        &election_event_id,
        &contest,
    )
    .await
    .map_err(|err| {
        anyhow!(
            "Error generate total number of expected voters for contest: {} {err}",
            &contest.id
        )
    })?;

    let total_position = results_area_contest.total_votes.unwrap_or(-1);
    let total_undevotes = generate_total_number_of_under_votes(&results_area_contest)
        .await
        .map_err(|err| {
            anyhow!(
                "Error generate total number of under votes for contest: {} {err}",
                &contest.id
            )
        })?;

    let fill_up_rate = generate_fill_up_rate(&results_area_contest, &total_expected)
        .await
        .map_err(|err| {
            anyhow!(
                "Error generate fill up rate for contest: {} {err}",
                &contest.id
            )
        })?;

    Ok(ReportContestData {
        elective_position,
        total_expected,
        total_position,
        total_undevotes,
        fill_up_rate,
    })
}
