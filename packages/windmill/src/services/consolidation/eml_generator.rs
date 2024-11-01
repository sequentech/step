use crate::types::miru_plugin::{MiruCcsServer, MiruTallySessionData};

// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_types::*;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::{
    ballot::*,
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    types::{
        date_time::*,
        hasura::core::{self, ElectionEvent, Trustee},
    },
    util::date_time::{generate_timestamp, get_system_timezone},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString, ToString};
use tracing::{info, instrument};
use velvet::pipes::{do_tally::ContestResult, generate_reports::ReportData};

pub const MIRU_PLUGIN_PREPEND: &str = "miru";
pub const MIRU_ELECTION_EVENT_ID: &str = "election-event-id";
pub const MIRU_ELECTION_EVENT_NAME: &str = "election-event-name";
const MIRU_ELECTION_ID: &str = "election-id";
const MIRU_ELECTION_NAME: &str = "election-name";
const MIRU_CONTEST_ID: &str = "contest-id";
const MIRU_CONTEST_NAME: &str = "contest-name";
const MIRU_CANDIDATE_ID: &str = "candidate-id";
const MIRU_CANDIDATE_NAME: &str = "candidate-name";
const MIRU_CANDIDATE_SETTING: &str = "candidate-setting";
const MIRU_CANDIDATE_AFFILIATION_ID: &str = "candidate-affiliation-id";
const MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME: &str = "candidate-affiliation-registered-name";
const MIRU_CANDIDATE_AFFILIATION_PARTY: &str = "candidate-affiliation-party";
pub const MIRU_AREA_CCS_SERVERS: &str = "area-ccs-servers";
pub const MIRU_AREA_STATION_ID: &str = "area-station-id";
pub const MIRU_AREA_THRESHOLD: &str = "area-threshold";
pub const MIRU_AREA_TRUSTEE_USERS: &str = "area-trustee-users";
pub const MIRU_TALLY_SESSION_DATA: &str = "tally-session-data";
pub const MIRU_TRUSTEE_ID: &str = "trustee-id";
pub const MIRU_TRUSTEE_NAME: &str = "trustee-name";

const ISSUE_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
const OFFICIAL_STATUS_DATE_FORMAT: &str = "%Y-%m-%d";

/*COMELEC ELECTION DATA -> to be change if revice different keys  */
pub const MIRU_GEOGRAPHICAL_REGION: &str = "geographical_region";
pub const MIRU_VOTING_CENTER: &str = "voting_center";
pub const MIRU_PRECINCT_CODE: &str = "precinct_code";
/**/

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, EnumString, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OfficialStatus {
    OFFICIAL,
}

pub trait GetMetrics {
    fn get_metrics(&self) -> Vec<EMLCountMetric>;
}

// TODO: review
impl GetMetrics for ContestResult {
    #[instrument(skip_all)]
    fn get_metrics(&self) -> Vec<EMLCountMetric> {
        let extended_metrics = self.extended_metrics.clone().unwrap_or_default();

        vec![
            EMLCountMetric {
                kind: "Total Number of Over Votes".into(),
                id: "OV".into(),
                datum: extended_metrics.over_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Under Votes".into(),
                id: "UV".into(),
                datum: extended_metrics.under_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Votes Actually".into(),
                id: "VV".into(),
                datum: extended_metrics.votes_actually as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Registered Voters".into(),
                id: "RV".into(),
                datum: self.census as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Expected Votes".into(),
                id: "EV".into(),
                datum: extended_metrics.expected_votes as i64,
            },
            EMLCountMetric {
                kind: "Number of Zero Outs Executed".into(),
                id: "RZ".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Scanned Ballots".into(),
                id: "TB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Valid Ballots".into(),
                id: "VB".into(),
                datum: self.total_valid_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Stamped Ballots".into(),
                id: "SB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Ballots In Ballot Box".into(),
                id: "BB".into(),
                datum: self.total_votes as i64,
            },
            EMLCountMetric {
                kind: "Abstentions".into(),
                id: "AB".into(),
                datum: self.total_blank_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Invalid Ballots".into(),
                id: "IB".into(),
                datum: self.total_invalid_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Misread Ballots".into(),
                id: "MB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Fake Ballots".into(),
                id: "FB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Previously Casted Ballots".into(),
                id: "PB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Returned Ballots".into(),
                id: "RB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Rejected Ballots".into(),
                id: "JB".into(),
                datum: 0,
            },
        ]
    }
}

pub trait ValidateAnnotations {
    fn get_valid_annotations(&self) -> Result<Annotations>;
}

#[instrument(err)]
fn check_annotations_exist(keys: Vec<String>, annotations: &Annotations) -> Result<()> {
    for key in keys {
        if !annotations.contains_key(&key) {
            return Err(anyhow!("Annotation: missing key {}", key));
        }
    }
    Ok(())
}

impl ValidateAnnotations for Trustee {
    #[instrument(err)]
    fn get_valid_annotations(&self) -> Result<Annotations> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing trustee annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_TRUSTEE_ID),
                prepend_miru_annotation(MIRU_TRUSTEE_NAME),
            ],
            &annotations,
        )
        .with_context(|| "Trustee: ")?;
        Ok(annotations)
    }
}

impl ValidateAnnotations for ElectionEvent {
    #[instrument(err)]
    fn get_valid_annotations(&self) -> Result<Annotations> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing election event annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_ELECTION_EVENT_ID),
                prepend_miru_annotation(MIRU_ELECTION_EVENT_NAME),
            ],
            &annotations,
        )
        .with_context(|| "Election Event: ")?;
        Ok(annotations)
    }
}

impl ValidateAnnotations for core::Election {
    #[instrument(err)]
    fn get_valid_annotations(&self) -> Result<Annotations> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing election annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_ELECTION_ID),
                prepend_miru_annotation(MIRU_ELECTION_NAME),
            ],
            &annotations,
        )
        .with_context(|| "Contest: ")?;
        Ok(annotations)
    }
}

impl ValidateAnnotations for core::Area {
    #[instrument(err)]
    fn get_valid_annotations(&self) -> Result<Annotations> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing area annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_AREA_CCS_SERVERS),
                prepend_miru_annotation(MIRU_AREA_STATION_ID),
                prepend_miru_annotation(MIRU_AREA_THRESHOLD),
                prepend_miru_annotation(MIRU_AREA_TRUSTEE_USERS),
                // prepend_miru_annotation(MIRU_GEOGRAPHICAL_REGION), //TODO: uncomment when exist
                // prepend_miru_annotation(MIRU_VOTING_CENTER),
                // prepend_miru_annotation(MIRU_PRECINCT_CODE),
            ],
            &annotations,
        )
        .with_context(|| "Area: ")?;

        let trustee_users_js = find_miru_annotation(MIRU_AREA_TRUSTEE_USERS, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_AREA_TRUSTEE_USERS
                )
            })?;
        let _trustee_users: Vec<String> =
            deserialize_str(&trustee_users_js).map_err(|err| anyhow!("{}", err))?;

        let ccs_servers_js = find_miru_annotation(MIRU_AREA_CCS_SERVERS, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_AREA_CCS_SERVERS
                )
            })?;
        let _ccs_servers: Vec<MiruCcsServer> =
            deserialize_str(&ccs_servers_js).map_err(|err| anyhow!("{}", err))?;
        Ok(annotations)
    }
}

impl ValidateAnnotations for core::TallySession {
    #[instrument(err)]
    fn get_valid_annotations(&self) -> Result<Annotations> {
        let Some(annotations_js) = self.annotations.clone() else {
            info!("Tally session has empty annotations");
            return Ok(HashMap::new());
        };

        let annotations: Annotations = deserialize_value(annotations_js)?;

        let Ok(_) = check_annotations_exist(
            vec![prepend_miru_annotation(MIRU_TALLY_SESSION_DATA)],
            &annotations,
        )
        .with_context(|| "Tally Session: ") else {
            info!("Tally session doesn't have miru annotations yet");
            return Ok(annotations);
        };

        let Ok(tally_session_data_js) = find_miru_annotation(MIRU_TALLY_SESSION_DATA, &annotations)
            .with_context(|| {
                format!(
                    "Missing tally session annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA
                )
            })
        else {
            info!("Tally session doesn't have miru annotations yet");
            return Ok(annotations);
        };
        let _ccs_servers: MiruTallySessionData =
            deserialize_str(&tally_session_data_js).map_err(|err| anyhow!("{}", err))?;

        Ok(annotations)
    }
}

impl ValidateAnnotations for Contest {
    #[instrument(err)]
    fn get_valid_annotations(&self) -> Result<Annotations> {
        let annotations = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing contest annotations"))?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_CONTEST_NAME),
                prepend_miru_annotation(MIRU_CONTEST_ID),
            ],
            &annotations,
        )
        .with_context(|| "Contest: ")?;
        Ok(annotations)
    }
}

impl ValidateAnnotations for Candidate {
    #[instrument(err)]
    fn get_valid_annotations(&self) -> Result<Annotations> {
        let annotations = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing candidate annotations"))?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_CANDIDATE_ID),
                prepend_miru_annotation(MIRU_CANDIDATE_NAME),
                prepend_miru_annotation(MIRU_CANDIDATE_SETTING),
                prepend_miru_annotation(MIRU_CANDIDATE_AFFILIATION_ID),
                prepend_miru_annotation(MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME),
                prepend_miru_annotation(MIRU_CANDIDATE_AFFILIATION_PARTY),
            ],
            &annotations,
        )
        .with_context(|| "Candidate: ")?;
        Ok(annotations)
    }
}

#[instrument]
pub fn prepend_miru_annotation(data: &str) -> String {
    format!("{}:{}", MIRU_PLUGIN_PREPEND, data)
}

#[instrument(err)]
pub fn find_miru_annotation(data: &str, annotations: &Annotations) -> Result<String> {
    let key = prepend_miru_annotation(data);
    annotations
        .get(&key)
        .ok_or(anyhow!("Can't find annotation key {}", key))
        .cloned()
}

#[instrument(err)]
pub fn find_miru_annotation_opt(data: &str, annotations: &Annotations) -> Result<Option<String>> {
    let key = prepend_miru_annotation(data);
    Ok(annotations.get(&key).cloned())
}

#[instrument(err)]
pub fn render_eml_contest(report: &ReportData) -> Result<EMLContest> {
    // Extract contest annotations
    let contest_annotations = report
        .contest
        .get_valid_annotations()
        .with_context(|| "render_eml_contest: ")?;

    // Retrieve contest name and ID from annotations
    let contest_name =
        find_miru_annotation(MIRU_CONTEST_NAME, &contest_annotations).with_context(|| {
            format!(
                "Missing contest annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_CONTEST_NAME
            )
        })?;
    let contest_id =
        find_miru_annotation(MIRU_CONTEST_ID, &contest_annotations).with_context(|| {
            format!(
                "Missing contest annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_CONTEST_ID
            )
        })?;
    let count_metrics = report.contest_result.get_metrics();

    let selections: Vec<EMLSelection> = report
        .contest_result
        .candidate_result
        .iter()
        .map(|candidate_result| -> Result<EMLSelection> {
            // Retrieve candidate annotations
            let candidate_annotations = candidate_result
                .candidate
                .get_valid_annotations()
                .with_context(|| "render_eml_contest: ")?;

            // Retrieve candidate name and ID from annotations
            let candidate_name = find_miru_annotation(MIRU_CANDIDATE_NAME, &candidate_annotations)
                .with_context(|| {
                    format!(
                        "Missing candidate annotation: '{}:{}'",
                        MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_NAME
                    )
                })?;
            let candidate_id = find_miru_annotation(MIRU_CANDIDATE_ID, &candidate_annotations)
                .with_context(|| {
                    format!(
                        "Missing candidate annotation: '{}:{}'",
                        MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_ID
                    )
                })?;
            let candidate_setting =
                find_miru_annotation(MIRU_CANDIDATE_SETTING, &candidate_annotations).with_context(
                    || {
                        format!(
                            "Missing candidate annotation: '{}:{}'",
                            MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_SETTING
                        )
                    },
                )?;
            let candidate_affiliation_id =
                find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_ID, &candidate_annotations)
                    .with_context(|| {
                        format!(
                            "Missing candidate annotation: '{}:{}'",
                            MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_AFFILIATION_ID
                        )
                    })?;
            let candidate_affiliation_registered_name = find_miru_annotation(
                MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME,
                &candidate_annotations,
            )
            .with_context(|| {
                format!(
                    "Missing candidate annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME
                )
            })?;
            let candidate_affiliation_party =
                find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_PARTY, &candidate_annotations)
                    .with_context(|| {
                        format!(
                            "Missing candidate annotation: '{}:{}'",
                            MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_AFFILIATION_PARTY
                        )
                    })?;

            let candidate = EMLCandidate {
                identifier: EMLIdentifier {
                    id_number: candidate_id,
                    name: candidate_name,
                },
                status_details: vec![EMLStatusItem {
                    setting: candidate_setting.clone(),
                }],
                affiliation: EMLAffiliation {
                    identifier: EMLIdentifier {
                        id_number: candidate_affiliation_id,
                        name: candidate_affiliation_registered_name,
                    },
                    party: candidate_affiliation_party,
                },
            };
            Ok(EMLSelection {
                candidates: vec![candidate.clone()],
                valid_votes: candidate_result.total_count as i64,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let contests = EMLContest {
        identifier: EMLIdentifier {
            id_number: contest_id,
            name: contest_name,
        },
        total_votes: EMLTotalVotes {
            count_metrics,
            selections,
        },
    };

    Ok(contests)
}

#[instrument(err)]
pub fn render_eml_file(
    tally_id: &str,
    transaction_id: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &Annotations,
    election_annotations: &Annotations,
    reports: &Vec<ReportData>,
) -> Result<EMLFile> {
    let election_event_id =
        find_miru_annotation(MIRU_ELECTION_EVENT_ID, election_event_annotations).with_context(
            || {
                format!(
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_ID
                )
            },
        )?;
    let election_event_name =
        find_miru_annotation(MIRU_ELECTION_EVENT_NAME, election_event_annotations).with_context(
            || {
                format!(
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_NAME
                )
            },
        )?;

    let election_name = find_miru_annotation(MIRU_ELECTION_NAME, election_annotations)
        .with_context(|| {
            format!(
                "Missing election annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_ELECTION_NAME
            )
        })?;

    let election_id =
        find_miru_annotation(MIRU_ELECTION_ID, election_annotations).with_context(|| {
            format!(
                "Missing election annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_ELECTION_NAME
            )
        })?;

    let issue_date = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(ISSUE_DATE_FORMAT.to_string())),
        Some(date_time.clone()),
    );
    let official_status_date = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(OFFICIAL_STATUS_DATE_FORMAT.to_string())),
        Some(date_time.clone()),
    );

    let eml_file = EMLFile {
        id: tally_id.to_string(),
        header: EMLHeader {
            transaction_id: transaction_id.to_string(),
            issue_date: issue_date,
            official_status_detail: EMLOfficialStatusDetail {
                official_status: OfficialStatus::OFFICIAL.to_string(),
                status_date: official_status_date,
            },
        },
        counts: vec![EMLCount {
            identifier: EMLIdentifier {
                id_number: election_event_id,
                name: election_event_name,
            },
            elections: vec![EMLElection {
                identifier: EMLIdentifier {
                    id_number: election_id,
                    name: election_name,
                },
                contests: reports
                    .into_iter()
                    .map(|report| Ok(render_eml_contest(report)?))
                    .collect::<Result<Vec<_>>>()
                    .with_context(|| "Error rendering EML Contest")?,
            }],
        }],
    };
    Ok(eml_file)
}
