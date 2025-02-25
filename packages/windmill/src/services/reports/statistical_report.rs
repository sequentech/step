// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations,
    generate_election_area_votes_data, get_app_hash, get_app_version, get_date_and_time,
    get_report_hash, get_results_hash, ExecutionAnnotations, InspectorData,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::contest::get_contest_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::{ReportCronConfig, ReportType};
use crate::postgres::results_area_contest::get_results_area_contest;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_area_id;
use crate::services::election_dates::get_election_dates;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::hasura::core::Contest;
use sequent_core::types::results::ResultsAreaContest;
use sequent_core::types::results::*;
use sequent_core::types::results::*;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::util::temp_path::*;
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
    pub election_title: String,
    pub post: String,
    pub country: String,
    pub geographical_region: String,
    pub voting_center: String,
    pub station_id: String,
    pub station_name: String,
    pub registered_voters: Option<i64>,
    pub ballots_counted: Option<i64>,
    pub voters_turnout: Option<f64>,
    pub elective_positions: Vec<ReportContestData>,
    pub inspectors: Vec<InspectorData>,
}

/// Struct for User Data Area
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
    pub election_dates: StringifiedPeriodDates,
    pub execution_annotations: ExecutionAnnotations,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReportContestData {
    pub elective_position: String,
    pub total_expected: Option<i64>,
    pub total_position: Option<i64>,
    pub total_undervotes: Option<i64>,
    pub fill_up_rate: Option<f64>,
}

/// Implementation of TemplateRenderer for Manual Verification
#[derive(Debug)]
pub struct StatisticalReportTemplate {
    ids: ReportOrigins,
}

impl StatisticalReportTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        StatisticalReportTemplate { ids }
    }
}

#[async_trait]
impl TemplateRenderer for StatisticalReportTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn base_name(&self) -> String {
        "statistical_report".to_string()
    }

    fn get_election_id(&self) -> Option<String> {
        self.ids.election_id.clone()
    }

    fn prefix(&self) -> String {
        format!(
            "statistical_report_{}_{}_{}",
            self.ids.tenant_id,
            self.ids.election_event_id,
            self.ids.election_id.clone().unwrap_or_default()
        )
    }

    fn get_report_type(&self) -> ReportType {
        ReportType::STATISTICAL_REPORT
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);
        let date_printed = get_date_and_time();

        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Error getting election event by id: {}", e))?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        let election_cloned = election.clone();
        let election_title = election_cloned.alias.unwrap_or(election_cloned.name);

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
        })?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            Some(&election_id),
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let results_hash = get_results_hash(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .unwrap_or("-".to_string());

        let report_hash = get_report_hash(&ReportType::STATISTICAL_REPORT.to_string())
            .await
            .unwrap_or("-".to_string());

        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled events by election event_id: {}", e)
        })?;

        let election_dates = get_election_dates(&election, scheduled_events)
            .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Error extract area data {err}"))?;

            let contests = get_contest_by_election_id(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error getting contests {e:?}")))?;

            let mut elective_positions: Vec<ReportContestData> = vec![];

            let votes_data = generate_election_area_votes_data(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                election.id.as_str(),
                &area.id,
                None,
            )
            .await
            .map_err(|e| anyhow!(format!("Error generating election area votes data {e:?}")))?;

            let ballots_counted = count_ballots_by_area_id(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error getting counted ballots: {err}"))?;

            for contest in contests.clone() {
                let results_area_contest = get_results_area_contest(
                    &hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    &election_id,
                    Some(&contest.id.clone()),
                    &area.id,
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(format!("Error getting results area contest {e:?}"))
                })?;

                match results_area_contest {
                    Some(results_area_contest) => {
                        let contest_result_data = generate_contest_results_data(
                            &contest,
                            &results_area_contest,
                            &votes_data.registered_voters,
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

            areas.push(UserDataArea {
                election_title: election_title.clone(),
                post: election_general_data.post.clone(),
                country: country,
                geographical_region: election_general_data.geographical_region.clone(),
                voting_center: election_general_data.voting_center.clone(),
                station_name: election_general_data.precinct_code.clone(),
                station_id: election_general_data.pollcenter_code.clone(),
                registered_voters: votes_data.registered_voters,
                ballots_counted: Some(ballots_counted),
                voters_turnout: votes_data.voters_turnout,
                elective_positions,
                inspectors: area_general_data.inspectors,
            })
        }

        Ok(UserData {
            areas,
            election_dates,
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                app_version: app_version.clone(),
                software_version: app_version.clone(),
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: Some(results_hash),
                user_timezone: None,
            },
        })
    }

    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        if pdf::doc_renderer_backend() == pdf::DocRendererBackend::InPlace {
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
        } else {
            // If we are rendering with a lambda, the QRCode lib is
            // already included in the lambda container image.
            Ok(SystemData {
                rendered_user_template,
                file_qrcode_lib: "/assets/qrcode.min.js".to_string(),
            })
        }
    }
}

#[instrument(err, skip(contest))]
pub async fn generate_total_number_of_expected_votes_for_contest(
    contest: &Contest,
    registered_voters: &i64,
) -> Result<i64> {
    if let Some(max_votes) = contest.max_votes {
        Ok(registered_voters * max_votes)
    } else {
        Ok(*registered_voters)
    }
}

#[instrument(err, skip_all)]
pub async fn generate_fill_up_rate(num_of_expected_votes: &i64, total_votes: &i64) -> Result<f64> {
    let votes = *total_votes;
    let expected_votes = *num_of_expected_votes;
    let fill_up_rate: f64 = if expected_votes == 0 {
        0.0
    } else {
        (votes as f64 / expected_votes as f64) * 100.0
    };
    Ok(fill_up_rate.clamp(0.0, 100.0))
}

//generate data for specific contest
#[instrument(err, skip_all)]
pub async fn generate_contest_results_data(
    contest: &Contest,
    results_area_contest: &ResultsAreaContest,
    registered_voters: &Option<i64>,
) -> Result<ReportContestData> {
    let Some(elective_position) = contest.name.clone() else {
        return Err(anyhow!("Contest with empty name"));
    };

    let registered_voters = match registered_voters {
        Some(voters) => *voters,
        None => {
            return Ok(ReportContestData {
                elective_position,
                total_expected: None,
                total_position: None,
                total_undervotes: None,
                fill_up_rate: None,
            });
        }
    };

    let total_expected =
        generate_total_number_of_expected_votes_for_contest(&contest, &registered_voters)
            .await
            .map_err(|err| {
                anyhow!(
                    "Error generating total number of expected voters for contest: {} {err}",
                    &contest.id
                )
            })?;

    let results_area_contest_annotations = results_area_contest.annotations.clone();

    let results_extended_metrics = results_area_contest_annotations
        .as_ref()
        .and_then(|annotations| annotations.get("extended_metrics"));

    let total_position: i64 = results_extended_metrics
        .as_ref()
        .and_then(|extended_metric| extended_metric.get("votes_actually"))
        .and_then(|votes_actually| votes_actually.as_i64())
        .unwrap_or(-1);

    let total_undervotes = results_extended_metrics
        .as_ref()
        .and_then(|extended_metric| extended_metric.get("under_votes"))
        .and_then(|under_votes| under_votes.as_i64())
        .unwrap_or(-1);

    // Ensure total_expected and total_position are valid for fill_up_rate calculation
    let fill_up_rate = generate_fill_up_rate(&total_expected, &total_position)
        .await
        .map_err(|err| {
            anyhow!(
                "Error generating fill-up rate for contest: {} {err}",
                &contest.id
            )
        })?;

    Ok(ReportContestData {
        elective_position,
        total_expected: Some(total_expected),
        total_position: Some(total_position),
        total_undervotes: Some(total_undervotes),
        fill_up_rate: Some(fill_up_rate),
    })
}
