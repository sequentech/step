// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use board_messages::braid::message::Message;
use sequent_core::types::ceremonies::Log;
use time::OffsetDateTime;
use tracing::instrument;

pub fn get_now_utc_unix() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

pub fn message_to_log(message: &Message) -> Log {
    let batch_number = message.statement.get_batch_number();
    let timestamp = message.statement.get_timestamp();

    Log {
        created_date: timestamp.to_string(),
        log_text: format!(
            "Added message {} for batch {}",
            message.statement.get_kind().to_string(),
            batch_number
        ),
    }
}

#[instrument(skip(messages, batch_ids), err)]
pub async fn generate_logs(
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
    Ok(logs)
}

#[instrument]
pub fn generate_tally_initial_log(election_ids: &Vec<String>) -> Vec<Log> {
    vec![Log {
        created_date: get_now_utc_unix().to_string(),
        log_text: format!("Created Tally Ceremony for election ids {:?}", election_ids,),
    }]
}

#[instrument(current_logs)]
pub fn append_tally_trustee_log(current_logs: &Vec<Log>, trustee_name: &str) -> Vec<Log> {
    let mut logs: Vec<Log> = current_logs.clone();
    logs.push(Log {
        created_date: get_now_utc_unix().to_string(),
        log_text: format!("Restored private key for trustee {}", trustee_name,),
    });
    logs
}
