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
pub fn convert_to_eml_file(
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

    let election_event_id =
        find_miru_annotation(MIRU_ELECTION_EVENT_ID, &Some(election_annotations.clone()))
            .with_context(|| "")?;
    let election_event_name = find_miru_annotation(
        MIRU_ELECTION_EVENT_NAME,
        &Some(election_annotations.clone()),
    )
    .with_context(|| "")?;

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
            elections: vec![],
        }],
    };
    Ok(eml_file)
}
