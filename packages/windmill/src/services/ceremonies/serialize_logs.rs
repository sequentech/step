// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use b3::messages::message::Message;
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::Log;
use tracing::{event, instrument, Level};

/// Builds a [`Log`] describing who posted `message`, which statement kind it carries, and its batch.
///
/// # Panics
///
/// Panics if converting the on-board timestamp to milliseconds would overflow `u64` multiplication
/// by 1000 (`expect("timestamp millis overflow")`).
pub fn message_to_log(message: &Message) -> Log {
    let batch_number = message.statement.get_batch_number();
    let timestamp = message
        .statement
        .get_timestamp()
        .checked_mul(1000)
        .expect("timestamp millis overflow");
    let datetime = ISO8601::timestamp_ms_utc_to_date(timestamp as i64);

    Log {
        created_date: ISO8601::to_string(&datetime),
        log_text: format!(
            "{}: Added message {} for batch {}",
            &message.sender.name,
            message.statement.get_kind(),
            batch_number
        ),
    }
}

/// Emits each derived log line through the tracing pipeline.
///
/// # Errors
///
/// Always returns `Ok`; reserved for future filtering failures.
#[instrument(skip(messages), err)]
pub fn print_messages(messages: &[Message], board_name: &str) -> Result<()> {
    let logs: Vec<Log> = messages.iter().map(message_to_log).collect();
    let sorted_logs = sort_logs(&logs);

    event!(Level::INFO, "printing messages for board {}", board_name);
    for log in sorted_logs.iter() {
        event!(Level::INFO, "{}: {}", log.created_date, log.log_text);
    }

    Ok(())
}

/// Filters `messages` to those at or after `next_timestamp` with batch ids in `batch_ids`, maps
/// them with [`message_to_log`], and returns them sorted by [`sort_logs`].
///
/// # Errors
///
/// Always returns `Ok`; reserved for future validation errors.
#[instrument(skip(messages, batch_ids), err)]
pub fn generate_logs(
    messages: &[Message],
    next_timestamp: u64,
    batch_ids: &[i64],
) -> Result<Vec<Log>> {
    let relevant_messages: Vec<&Message> = messages
        .iter()
        .filter(|message| {
            message.statement.get_timestamp() >= next_timestamp
                && batch_ids.contains(&(message.statement.get_batch_number() as i64))
        })
        .collect();
    let logs: Vec<Log> = relevant_messages
        .iter()
        .map(|message| message_to_log(message))
        .collect();
    Ok(sort_logs(&logs))
}

/// Seed log line emitted when a tally ceremony row is first created for `election_ids`.
#[instrument]
pub fn generate_tally_initial_log(election_ids: &Vec<String>) -> Vec<Log> {
    vec![Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Created Tally Ceremony for election ids: {election_ids:?}",),
    }]
}

/// Returns a time-ordered copy of `logs`.
#[instrument(skip_all)]
pub fn sort_logs(logs: &[Log]) -> Vec<Log> {
    let mut sorted = logs.to_owned();

    sorted.sort_by(|a, b| {
        let a_date = ISO8601::to_date(&a.created_date).unwrap_or(ISO8601::now());
        let b_date = ISO8601::to_date(&b.created_date).unwrap_or(ISO8601::now());
        a_date.cmp(&b_date)
    });

    sorted
}

/// Seed log line emitted when a keys ceremony is created listing participating trustees.
#[instrument]
pub fn generate_keys_initial_log(trustee_names: &Vec<String>) -> Vec<Log> {
    vec![Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Created Keys Ceremony with trustees: {trustee_names:?}",),
    }]
}

/// Appends a “restored private key” line for `trustee_name` during tally trustee reconnect.
#[instrument(skip(current_logs))]
pub fn append_tally_trustee_log(current_logs: &[Log], trustee_name: &str) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.to_owned();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Restored private key for trustee {trustee_name}"),
    });
    sort_logs(&logs)
}

/// Appends a keys-ceremony log entry when a trustee downloads their encrypted private key material.
#[instrument(skip(current_logs))]
pub fn append_keys_trustee_download_log(current_logs: &[Log], trustee_name: &str) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.to_owned();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Downloaded private key for trustee {trustee_name}"),
    });
    sort_logs(&logs)
}

/// Appends a keys-ceremony log entry when a trustee confirms their key matches the board copy.
#[instrument(skip(current_logs))]
pub fn append_keys_trustee_check_log(current_logs: &[Log], trustee_name: &str) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.to_owned();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Checked private key for trustee {trustee_name}"),
    });
    sort_logs(&logs)
}

/// Appends a log line when tally processing completes for `election_ids`.
#[instrument(skip(current_logs))]
pub fn append_tally_finished(current_logs: &[Log], election_ids: &[String]) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.to_owned();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Finished Tally Ceremony for election ids: {election_ids:?}"),
    });
    sort_logs(&logs)
}

/// Appends a log line when tally ceremony metadata is refreshed for `election_ids`.
#[instrument(skip(current_logs))]
pub fn append_tally_updated(current_logs: &[Log], election_ids: &[String]) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.to_owned();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Updated Tally Ceremony for election ids: {election_ids:?}"),
    });
    sort_logs(&logs)
}

/// Appends the standard message recorded when tally execution resumes after an IRV tie resolution.
#[instrument(skip(current_logs))]
pub fn append_tally_resumed_after_resolution(current_logs: &[Log]) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.to_owned();
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: "Tally execution resumed after tie-break resolution submission".to_string(),
    });
    sort_logs(&logs)
}
