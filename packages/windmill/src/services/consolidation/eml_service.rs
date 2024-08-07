// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_types::*;
use anyhow::{anyhow, Context, Result};
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

pub fn prepend_miru_annotation(data: &str) -> String {
    format!("{}:{}", MIRU_PLUGIN_PREPEND, data)
}

pub fn find_miru_annotation(data: &str, annotations_opt: &Option<Annotations>) -> Result<String> {
    let key = prepend_miru_annotation(data);
    let annotations = annotations_opt
        .clone()
        .ok_or(anyhow!("Missing annotations"))?;
    annotations
        .get(&key)
        .ok_or(anyhow!("Can't find annotation key {}", key))
}

pub fn convert_to_eml_file(
    election_event_annotations: Option<Annotations>,
    election_annotations: Option<Annotations>,
    report: ReportData,
) -> Result<EMLFile> {
    Err(anyhow!("not implemented"))
}
