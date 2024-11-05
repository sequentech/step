// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations,
    generate_voters_turnout, get_app_hash, get_app_version, get_date_and_time, get_results_hash,
    get_total_number_of_registered_voters_for_area_id, InspectorData,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::contest::get_contest_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::results_area_contest::{get_results_area_contest, ResultsAreaContest};
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_area_id;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
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
    pub country: String,
    pub ballots_counted: i64,
    pub geographical_region: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub voters_turnout: f64,
    pub elective_positions: Vec<ReportContestData>,
    pub report_hash: String,
    pub results_hash: String,
    pub ovcs_version: String,
    pub software_version: String,
    pub system_hash: String,
    pub inspectors: Vec<InspectorData>,
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
    pub fill_up_rate: f64,
}

/// Implementation of TemplateRenderer for Manual Verification
#[derive(Debug)]
pub struct StatisticalReportTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
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
        self.election_id.clone()
    }

    fn prefix(&self) -> String {
        format!(
            "statistical_report_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
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

        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Error getting election event by id: {}", e))?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        let election_title = election.name.clone();

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

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
            Some(&election_id),
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_date: String = voting_period_start_date.clone();

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let results_hash = get_results_hash(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .unwrap_or("-".to_string());

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Error extract area data {err}"))?;

            let registered_voters = get_total_number_of_registered_voters_for_area_id(
                &keycloak_transaction,
                &realm,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error counting registered voters: {err}"))?;

            let (results_area_contests, contests) = get_election_contests_area_results(
                &hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
                &election_id,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error getting election contest, results: {err}"))?;

            let ballots_counted = count_ballots_by_area_id(
                &hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
                &election_id,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error getting counted ballots: {err}"))?;

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
                            &contest,
                            &results_area_contest,
                            &registered_voters
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

            let country = area.clone().name.unwrap_or("-".to_string());

            let report_hash = "-".to_string();

            areas.push(UserDataArea {
                date_printed: date_printed.clone(),
                election_title: election_title.clone(),
                voting_period_start: voting_period_start_date.clone(),
                voting_period_end: voting_period_end_date.clone(),
                election_date: election_date.clone(),
                post: election_general_data.post.clone(),
                country: country,
                geographical_region: election_general_data.geographical_region.clone(),
                voting_center: election_general_data.voting_center.clone(),
                precinct_code: election_general_data.precinct_code.clone(),
                registered_voters,
                ballots_counted,
                voters_turnout,
                elective_positions,
                report_hash,
                software_version: app_version.clone(),
                ovcs_version: app_version.clone(),
                system_hash: app_hash.clone(),
                results_hash: results_hash.clone(),
                inspectors: area_general_data.inspectors,
            })
        }

        Ok(UserData { areas })
    }

    #[instrument(err, skip(self))]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
            ),
        })
    }
}

/// Function to generate the manual verification report using the TemplateRenderer
#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_statistical_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = StatisticalReportTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.map(|s| s.to_string()),
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

#[instrument(err, skip_all)]
pub async fn generate_total_number_of_expected_votes_for_contest(
    contest: &Contest,
    registered_voters: &i64,
) -> Result<i64> {
    match contest.max_votes {
        Some(max_votes) => Ok(registered_voters * max_votes),
        None => Ok(registered_voters.clone()),
    }
}

#[instrument(err, skip_all)]
pub async fn generate_fill_up_rate(num_of_expected_voters: &i64, total_votes: &i64) -> Result<f64> {
    let votes = *total_votes;
    let expected_votes = *num_of_expected_voters;
    let fill_up_rate: f64 = if expected_votes == 0 {
        0.0
    } else {
        (votes as f64 / expected_votes as f64) * 100.0
    };
    Ok(fill_up_rate)
}

//generate data for specific contest
#[instrument(err)]
pub async fn generate_contest_results_data(
    contest: &Contest,
    results_area_contest: &ResultsAreaContest,
    registered_voters: &i64,
) -> Result<ReportContestData> {
    let elective_position = contest.name.clone().unwrap();

    let total_expected =
        generate_total_number_of_expected_votes_for_contest(&contest, &registered_voters)
            .await
            .map_err(|err| {
                anyhow!(
                    "Error generate total number of expected voters for contest: {} {err}",
                    &contest.id
                )
            })?;

    let results_area_contest_annotitions = results_area_contest.annotations.clone();

    let total_position = results_area_contest_annotitions
        .as_ref()
        .and_then(|annotations| annotations.get("extended_metrics"))
        .and_then(|extended_metric| extended_metric.get("votes_actually"))
        .and_then(|under_vote| under_vote.as_i64())
        .unwrap_or(-1);

    let total_undevotes = total_expected - total_position;

    let fill_up_rate = generate_fill_up_rate(&total_expected, &total_position)
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

#[instrument(err, skip_all)]
pub async fn get_election_contests_area_results(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
) -> Result<(Vec<ResultsAreaContest>, Vec<Contest>)> {
    let contests: Vec<Contest> = get_contest_by_election_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .map_err(|e| anyhow::anyhow!(format!("Error getting results contests {e:?}")))?;

    let mut results_area_contests: Vec<ResultsAreaContest> = vec![];
    for contest in contests.clone() {
        // fetch area contest for the contest of the election
        let Some(results_area_contest) = get_results_area_contest(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election_id,
            &contest.id.clone(),
            &area_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!(format!("Error getting results area contest {e:?}")))?
        else {
            continue;
        };

        results_area_contests.push(results_area_contest.clone());
    }
    Ok((results_area_contests, contests))
}
