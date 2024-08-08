// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_types::*;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::{
    ballot::Annotations,
    types::date_time::*,
    util::date_time::{generate_timestamp, get_system_timezone},
};
use tracing::{info, instrument};
use velvet::pipes::generate_reports::ReportData;

const MIRU_PLUGIN_PREPEND: &str = "miru";
const MIRU_ELECTION_EVENT_ID: &str = "election-event-id";
const MIRU_ELECTION_EVENT_NAME: &str = "election-event-name";
const MIRU_ELECTION_ID: &str = "election-id";
const MIRU_ELECTION_NAME: &str = "election-name";
const MIRU_CONTEST_ID: &str = "contest-id";
const MIRU_CONTEST_NAME: &str = "contest-name";
const MIRU_CANDIDATE_ID: &str = "candidate-id";
const MIRU_CANDIDATE_NAME: &str = "candidate-name";
const MIRU_CANDIDATE_SETTING: &str = "candidate-setting";
const MIRU_CANDIDATE_AFFILIATION_ID: &str = "candidate-affiliation-id";
const MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME: &str = "candidate-affiliation-registered-name";
const MIRU_CANDIDATE_AFFILIATION_PARTY: &str = "candidate-affiliation-pary";

const ISSUE_DATE_FORMAT: &str = "%y-%m-%dT%H:%M:%S";
const OFFICIAL_STATUS_DATE_FORMAT: &str = "%y-%m-%d";

#[instrument]
pub fn prepend_miru_annotation(data: &str) -> String {
    format!("{}:{}", MIRU_PLUGIN_PREPEND, data)
}

#[instrument(err)]
pub fn find_miru_annotation(data: &str, annotations_opt: &Option<Annotations>) -> Result<String> {
    let key = prepend_miru_annotation(data);
    let annotations = annotations_opt
        .clone()
        .ok_or(anyhow!("Missing annotations"))?;
    annotations
        .get(&key)
        .ok_or(anyhow!("Can't find annotation key {}", key))
        .cloned()
}

#[instrument(err)]
pub fn render_eml_contests(report: &ReportData) -> Result<Vec<EMLContest>> {
    // Extract contest annotations
    let contest_annotations = report
        .contest
        .annotations
        .clone()
        .ok_or_else(|| anyhow!("Missing contest annotations"))?;

    // Retrieve contest name and ID from annotations
    let contest_name = find_miru_annotation(MIRU_CONTEST_NAME, &Some(contest_annotations.clone()))
        .with_context(|| {
            format!(
                "Missing contest annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_CONTEST_NAME
            )
        })?;
    let contest_id = find_miru_annotation(MIRU_CONTEST_ID, &Some(contest_annotations.clone()))
        .with_context(|| {
            format!(
                "Missing contest annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_CONTEST_ID
            )
        })?;

    let candidates: Vec<EMLCandidate> = report
        .contest_result
        .candidate_result
        .iter()
        .map(|candidate_result| -> Result<EMLCandidate> {
            // Retrieve candidate annotations
            let candidate_annotations = candidate_result
                .candidate
                .annotations
                .clone()
                .ok_or_else(|| anyhow!("Missing candidate annotations"))?;

            // Retrieve candidate name and ID from annotations
            let candidate_name =
                find_miru_annotation(MIRU_CANDIDATE_NAME, &Some(candidate_annotations.clone()))
                    .with_context(|| {
                        format!(
                            "Missing candidate annotation: '{}:{}'",
                            MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_NAME
                        )
                    })?;
            let candidate_id =
                find_miru_annotation(MIRU_CANDIDATE_ID, &Some(candidate_annotations.clone()))
                    .with_context(|| {
                        format!(
                            "Missing candidate annotation: '{}:{}'",
                            MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_ID
                        )
                    })?;

            // Build EMLCandidate structure
            Ok(EMLCandidate {
                identifier: EMLIdentifier {
                    id_number: candidate_id,
                    name: candidate_name,
                },
                status_details: candidate_annotations
                    .iter()
                    .map(|(key, _value)| EMLStatusItem {
                        setting: key.clone(),
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let selections: Vec<EMLSelection> = candidates
        .iter()
        .map(|candidate| -> Result<EMLSelection> {
            let candidate_result = report
                .contest_result
                .candidate_result
                .iter()
                .find(|cr| cr.candidate.id == candidate.identifier.id_number)
                .ok_or_else(|| {
                    anyhow!(
                        "Missing candidate result for candidate ID: {}",
                        candidate.identifier.id_number
                    )
                })?;

            Ok(EMLSelection {
                candidates: vec![candidate.clone()],
                valid_votes: candidate_result.total_count as i64,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let contests = vec![EMLContest {
        identifier: EMLIdentifier {
            id_number: contest_id,
            name: contest_name,
        },
        total_votes: EMLTotalVotes {
            count_metrics: vec![
                EMLCountMetric {
                    kind: "totalVotes".to_string(),
                    id: "total".to_string(),
                    datum: report.contest_result.total_votes as i64,
                },
                EMLCountMetric {
                    kind: "validVotes".to_string(),
                    id: "valid".to_string(),
                    datum: report.contest_result.total_valid_votes as i64,
                },
                EMLCountMetric {
                    kind: "invalidVotes".to_string(),
                    id: "invalid".to_string(),
                    datum: report.contest_result.total_invalid_votes as i64,
                },
                EMLCountMetric {
                    kind: "blankVotes".to_string(),
                    id: "blank".to_string(),
                    datum: report.contest_result.total_blank_votes as i64,
                },
            ],
            selections,
        },
    }];

    Ok(contests)
}

#[instrument(err)]
pub fn render_eml_file(
    tally_id: i64,
    transaction_id: i64,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations_opt: &Option<Annotations>,
    election_annotations_opt: &Option<Annotations>,
    report: &ReportData,
) -> Result<EMLFile> {
    let election_event_annotations = election_event_annotations_opt
        .clone()
        .ok_or(anyhow!("Missing election event annotations"))?;
    let election_annotations = election_annotations_opt
        .clone()
        .ok_or(anyhow!("Missing election event annotations"))?;

    let election_event_id = find_miru_annotation(
        MIRU_ELECTION_EVENT_ID,
        &Some(election_event_annotations.clone()),
    )
    .with_context(|| {
        format!(
            "Missing election event annotation: '{}:{}'",
            MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_ID
        )
    })?;
    let election_event_name = find_miru_annotation(
        MIRU_ELECTION_EVENT_NAME,
        &Some(election_event_annotations.clone()),
    )
    .with_context(|| {
        format!(
            "Missing election event annotation: '{}:{}'",
            MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_NAME
        )
    })?;

    let election_name =
        find_miru_annotation(MIRU_ELECTION_NAME, &Some(election_annotations.clone()))
            .with_context(|| {
                format!(
                    "Missing election annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_NAME
                )
            })?;

    let election_id = find_miru_annotation(MIRU_ELECTION_ID, &Some(election_annotations.clone()))
        .with_context(|| {
        format!(
            "Missing election annotation: '{}:{}'",
            MIRU_PLUGIN_PREPEND, MIRU_ELECTION_NAME
        )
    })?;

    //let time_zone = get_system_timezone();

    //let now_utc = Utc::now();

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
                official_status: "official".to_string(),
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
                contests: render_eml_contests(report)?,
            }],
        }],
    };
    Ok(eml_file)
}
