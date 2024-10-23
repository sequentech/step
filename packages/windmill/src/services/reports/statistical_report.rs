// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_fill_up_rate,
    generate_total_number_of_expected_votes_for_contest, generate_total_number_of_under_votes,
    generate_voters_turnout, get_date_and_time,
    get_election_contests_area_results_and_total_ballot_counted,
};
use super::template_renderer::*;
use crate::postgres::area::AreaElection;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::results_area_contest::ResultsAreaContest;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::users::count_keycloak_enabled_users_by_area_id;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::Contest;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
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
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub election_date: String,
    pub post: String,
    pub country: String,
    pub geographical_region: String,
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
    pub area: AreaElection,
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

    fn get_area(&self) -> Option<AreaElection> {
        Some(self.area.clone())
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
        let area = self.get_area().unwrap();

        let realm = get_event_realm(&tenant_id, &election_event_id);
        let date_printed = get_date_and_time();

        let election = if let Some(transaction) = hasura_transaction {
            get_election_by_id(&transaction, &tenant_id, &election_event_id, &election_id)
                .await
                .map_err(|err| anyhow!("Error getting election by id: {err}"))?
                .ok_or_else(|| anyhow!("Election not found"))?
        } else {
            return Err(anyhow::anyhow!("hasura_transaction is missing"));
        };

        let election_title = election.name.clone();

        // Fetch election event data
        let start_election_event = if let Some(transaction) = hasura_transaction {
            find_scheduled_event_by_election_event_id(&transaction, &tenant_id, &election_event_id)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
                })?
        } else {
            return Err(anyhow::anyhow!("hasura_transaction is missing"));
        };

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &tenant_id,
            &election_event_id,
            Some(&election_id),
        )?;

        // extract start date from voting period
        let voting_period_start_date = match voting_period_dates.start_date {
            Some(voting_period_start_date) => voting_period_start_date,
            None => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election start date: "
                )))
            }
        };

        // extract end date from voting period
        let voting_period_end_date = match voting_period_dates.end_date {
            Some(voting_period_end_date) => voting_period_end_date,
            None => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election end date: "
                )))
            }
        };

        let election_date: String = voting_period_start_date.clone();

        let election_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election data {err}"))?;

        let registered_voters = if let Some(transaction) = keycloak_transaction {
            count_keycloak_enabled_users_by_area_id(&transaction, &realm, &area.id)
                .await
                .map_err(|err| {
                    anyhow!("Error getting count_keycloak_enabled_users_by_area_id: {err}")
                })?
        } else {
            return Err(anyhow::anyhow!("keycloak_transaction is missing"));
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
                return Err(anyhow::anyhow!("hasura_transaction is missing"));
            };

        let voters_turnout = generate_voters_turnout(&ballots_counted, &registered_voters)
            .await
            .map_err(|err| anyhow!("Error generate voters turnout {err}"))?;

        let mut elective_positions: Vec<ReportContestData> = vec![];

        for contest in contests.clone() {
            let results_area_contest = results_area_contests
                .iter()
                .find(|rac| rac.contest_id == contest.id);

            match results_area_contest {
                Some(results_area_contest) => {
                    let contest_result_data = match keycloak_transaction {
                        Some(transaction) => generate_contest_results_data(
                            &transaction,
                            &realm,
                            &contest,
                            &results_area_contest,
                            &area.id,
                        )
                        .await
                        .map_err(|err| {
                            anyhow!(
                                "Error generate contest results data for contest: {} {err}",
                                &contest.id
                            )
                        })?,
                        None => {
                            return Err(anyhow::anyhow!("keycloak_transaction is missing"));
                        }
                    };

                    elective_positions.push(contest_result_data);
                }
                None => {}
            }
        }

        Ok(UserData {
            date_printed,
            election_title,
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            election_date,
            post: election_data.post.clone(),
            country: area.name.clone().unwrap(),
            geographical_region: election_data.geographical_region.clone(),
            voting_center: election_data.voting_center.clone(),
            precinct_code: election_data.precinct_code.clone(),
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
    area: &AreaElection,
    mode: GenerateReportMode,
    hasura_transaction: Option<&Transaction<'_>>,
    keycloak_transaction: Option<&Transaction<'_>>,
) -> Result<()> {
    let template = StatisticalReportTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        area: area.clone(),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            false,
            None,
            None,
            mode,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}

//generate data for specific contest
#[instrument(err, skip_all)]
pub async fn generate_contest_results_data(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    contest: &Contest,
    results_area_contest: &ResultsAreaContest,
    area_id: &str,
) -> Result<ReportContestData> {
    let elective_position = contest.name.clone().unwrap();

    let total_expected = generate_total_number_of_expected_votes_for_contest(
        &keycloak_transaction,
        &realm,
        &contest,
        area_id,
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
