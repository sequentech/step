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
use crate::postgres::area::get_areas_by_election_id;
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

/// Struct for User Data Area
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub date_printed: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub election_date: String,
    pub post: String,
    pub area_id: String,
    pub geographical_region: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub voters_turnout: i64,
    pub elective_positions: Vec<ReportContestData>,
}

/// Struct for User Data Area
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
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

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm = get_event_realm(&self.tenant_id, &self.election_event_id);
        let date_printed = get_date_and_time();

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        let election_title = election.name.clone();
        let election_date = election.created_at.clone().unwrap().to_string();

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
        })?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.tenant_id,
            &self.election_event_id,
            Some(&self.election_id),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_date: String = voting_period_start_date.clone();

        let election_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election data {err}"))?;

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            let registered_voters =
                count_keycloak_enabled_users_by_area_id(&keycloak_transaction, &realm, &area.id)
                    .await
                    .map_err(|err| anyhow!("Error counting registered voters: {err}"))?;

            let (ballots_counted, results_area_contests, contests) =
                get_election_contests_area_results_and_total_ballot_counted(
                    &keycloak_transaction,
                    &self.tenant_id,
                    &self.election_event_id,
                    &self.election_id,
                )
                .await
                .map_err(|err| {
                    anyhow!("Error getting election contest, results, and counted ballots: {err}")
                })?;

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
                        let contest_result_data = generate_contest_results_data(
                            &self.tenant_id,
                            &realm,
                            &self.election_event_id,
                            &contest,
                            &results_area_contest,
                            &results_area_contest.id,
                            &hasura_transaction,
                            &keycloak_transaction,
                        )
                        .await
                        .map_err(|err| {
                            anyhow!(
                                "Error generate contest results data for contest_id={contest_id}: {err}",
                                contest_id=&contest.id
                            )
                        })?;
                        elective_positions.push(contest_result_data);
                    }
                    None => {}
                }
            }
            areas.push(UserDataArea {
                date_printed: date_printed.clone(),
                election_title: election_title.clone(),
                voting_period_start: voting_period_start_date.clone(),
                voting_period_end: voting_period_end_date.clone(),
                election_date: election_date.clone(),
                post: election_data.post.clone(),
                area_id: election_data.area_id.clone(),
                geographical_region: election_data.geographical_region.clone(),
                voting_center: election_data.voting_center.clone(),
                precinct_code: election_data.precinct_code.clone(),
                registered_voters,
                voters_turnout,
                elective_positions,
            })
        }

        Ok(UserData { areas })
    }

    #[instrument(err, skip(self))]
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
#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_statistical_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
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
            mode,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}

//generate data for specific contest
#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_contest_results_data(
    tenant_id: &str,
    realm: &str,
    election_event_id: &str,
    contest: &Contest,
    results_area_contest: &ResultsAreaContest,
    area_id: &str,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<ReportContestData> {
    let elective_position = contest.name.clone().unwrap();

    let total_expected = generate_total_number_of_expected_votes_for_contest(
        &hasura_transaction,
        &keycloak_transaction,
        &realm,
        tenant_id,
        election_event_id,
        contest,
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
