// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use b3::messages::message::Message;
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::Log;
use tracing::{event, instrument, Level};

pub fn message_to_log(message: &Message) -> Log {
    let batch_number = message.statement.get_batch_number();
    let timestamp = message.statement.get_timestamp() * 1000;
    let datetime = ISO8601::timestamp_ms_utc_to_date(timestamp as i64);

    Log {
        created_date: ISO8601::to_string(&datetime),
        log_text: format!(
            "{}: Added message {} for batch {}",
            &message.sender.name,
            message.statement.get_kind().to_string(),
            batch_number
        ),
    }
}

#[instrument(skip(messages), err)]
pub fn print_messages(messages: &Vec<Message>, board_name: &str) -> Result<()> {
    let logs = messages
        .iter()
        .map(|message| message_to_log(message))
        .collect();
    let sorted_logs = sort_logs(&logs);

    event!(Level::INFO, "printing messages for board {}", board_name);
    for log in sorted_logs.iter() {
        event!(Level::INFO, "{}: {}", log.created_date, log.log_text);
    }

    Ok(())
}

#[instrument(skip(messages, batch_ids), err)]
pub fn generate_logs(
    messages: &Vec<Message>,
    next_timestamp: u64,
    batch_ids: &Vec<i64>,
) -> Result<Vec<Log>> {
    let relevant_messages: Vec<&Message> = messages
        .iter()
        .filter(|message| {
            message.statement.get_timestamp() >= next_timestamp
                && batch_ids.contains(&(message.statement.get_batch_number() as i64))
        })
        .collect();
    let logs = relevant_messages
        .iter()
        .map(|message| message_to_log(message))
        .collect();
    Ok(sort_logs(&logs))
}

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

#[instrument]
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
    vec![Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
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
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Downloaded private key for trustee {}", trustee_name,),
    });
    sort_logs(&logs)
}

#[instrument(skip(current_logs))]
pub fn append_keys_trustee_check_log(current_logs: &Vec<Log>, trustee_name: &str) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
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
