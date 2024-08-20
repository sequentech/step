// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::date::ISO8601;
use anyhow::Result;
use board_messages::braid::message::Message;
use sequent_core::types::ceremonies::Log;
use serde_json::value::Value;
use tracing::{event, instrument, Level};

// pub fn message_to_log(message: &Message) -> Log {
//     let batch_number = message.statement.get_batch_number();
//     let timestamp = message.statement.get_timestamp() * 1000;
//     let datetime = ISO8601::timestamp_ms_utc_to_date(timestamp as i64);

//     Log {
//         created_date: ISO8601::to_string(&datetime),
//         log_text: format!(
//             "{}: Added message {} for batch {}",
//             &message.sender.name,
//             message.statement.get_kind().to_string(),
//             batch_number
//         ),
//     }
// }

// #[instrument(skip(messages), err)]
// pub fn print_messages(messages: &Vec<Message>, board_name: &str) -> Result<()> {
//     let logs = messages
//         .iter()
//         .map(|message| message_to_log(message))
//         .collect();
//     let sorted_logs = sort_logs(&logs);

//     event!(Level::INFO, "printing messages for board {}", board_name);
//     for log in sorted_logs.iter() {
//         event!(Level::INFO, "{}: {}", log.created_date, log.log_text);
//     }

//     Ok(())
// }

// #[instrument(skip(messages, batch_ids), err)]
// pub fn generate_logs(
//     messages: &Vec<Message>,
//     next_timestamp: u64,
//     batch_ids: &Vec<i64>,
// ) -> Result<Vec<Log>> {
//     let relevant_messages: Vec<&Message> = messages
//         .iter()
//         .filter(|message| {
//             message.statement.get_timestamp() >= next_timestamp
//                 && batch_ids.contains(&(message.statement.get_batch_number() as i64))
//         })
//         .collect();
//     let logs = relevant_messages
//         .iter()
//         .map(|message| message_to_log(message))
//         .collect();
//     Ok(sort_logs(&logs))
// }

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
pub fn general_start_log() -> Vec<Log> {
    vec![Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("Task started"),
    }]
}

#[instrument(skip(current_logs))]
pub fn append_general_log(current_logs: &Option<Value>, message: &str) -> Vec<Log> {
    let value = current_logs.clone().unwrap_or(Value::Array(vec![]));
    let mut logs: Vec<Log> = serde_json::from_value(value).unwrap_or_else(|_| Vec::new());
    logs.push(Log {
        created_date: ISO8601::to_string(&ISO8601::now()),
        log_text: format!("{}", message),
    });
    sort_logs(&logs)
}

//TODO: maybe create more specific ones that accepts arguments
