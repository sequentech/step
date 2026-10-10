// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use chrono::{DateTime, Local};
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::Log;
use tracing::instrument;

#[instrument]
pub fn generate_tally_initial_log(election_ids: &Vec<String>) -> Vec<Log> {
    vec![Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!(
            "Created Tally Ceremony for election ids: {:?}",
            election_ids,
        ),
    }]
}

#[instrument(skip_all)]
pub fn sort_logs(logs: &Vec<Log>) -> Vec<Log> {
    let mut sorted = logs.clone();

    sorted.sort_by(|a, b| {
        let a_date = ISO8601::to_date(&a.created_date).unwrap_or(ISO8601::now());
        let b_date = ISO8601::to_date(&b.created_date).unwrap_or(ISO8601::now());
        a_date.cmp(&b_date)
    });

    sorted
}

#[instrument]
pub fn generate_keys_initial_log(trustee_names: &Vec<String>) -> Vec<Log> {
    generate_keys_initial_log_at(trustee_names, ISO8601::now())
}

#[instrument(skip(now))]
pub fn generate_keys_initial_log_at(trustee_names: &Vec<String>, now: DateTime<Local>) -> Vec<Log> {
    vec![Log {
        created_date: ISO8601::to_string(&now),
        log_text: format!("Created Keys Ceremony with trustees: {:?}", trustee_names,),
    }]
}

#[instrument(skip(current_logs))]
pub fn append_tally_trustee_log(current_logs: &Vec<Log>, trustee_name: &str) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Restored private key for trustee {}", trustee_name,),
    });
    sort_logs(&logs)
}

#[instrument(skip(current_logs))]
pub fn append_keys_trustee_download_log(current_logs: &Vec<Log>, trustee_name: &str) -> Vec<Log> {
    append_keys_trustee_download_log_at(current_logs, trustee_name, ISO8601::now())
}

#[instrument(skip(current_logs, now))]
pub fn append_keys_trustee_download_log_at(
    current_logs: &Vec<Log>,
    trustee_name: &str,
    now: DateTime<Local>,
) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&now),
        log_text: format!("Downloaded private key for trustee {}", trustee_name,),
    });
    sort_logs(&logs)
}

#[instrument(skip(current_logs))]
pub fn append_keys_trustee_check_log(current_logs: &Vec<Log>, trustee_name: &str) -> Vec<Log> {
    append_keys_trustee_check_log_at(current_logs, trustee_name, ISO8601::now())
}

#[instrument(skip(current_logs, now))]
pub fn append_keys_trustee_check_log_at(
    current_logs: &Vec<Log>,
    trustee_name: &str,
    now: DateTime<Local>,
) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&now),
        log_text: format!("Checked private key for trustee {}", trustee_name,),
    });
    sort_logs(&logs)
}

#[instrument(skip(current_logs))]
pub fn append_tally_finished(current_logs: &Vec<Log>, election_ids: &Vec<String>) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Finished Tally Ceremony for election ids: {election_ids:?}"),
    });
    sort_logs(&logs)
}

#[instrument(skip(current_logs))]
pub fn append_tally_updated(current_logs: &Vec<Log>, election_ids: &Vec<String>) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Updated Tally Ceremony for election ids: {election_ids:?}"),
    });
    sort_logs(&logs)
}

#[instrument(skip(current_logs))]
pub fn append_tally_resumed_after_resolution(current_logs: &Vec<Log>) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: "Tally execution resumed after tie-break resolution submission".to_string(),
    });
    sort_logs(&logs)
}

#[instrument(skip(current_logs))]
pub fn append_tally_recount_log(current_logs: &Vec<Log>, election_ids: &Vec<String>) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Recount launched for election ids: {election_ids:?}"),
    });
    sort_logs(&logs)
}
