// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Read-only views of queues for operators: depth, recent outcomes, throughput and
//! messages. Messages are summarized from their Celery envelope; task arguments are
//! never returned, because they can carry voters' data.

use crate::{decode, validate_queue_name, OUTCOME_HEADER};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio_postgres::{GenericClient, Row};

/// Message headers shown to operators; any other header is left out.
pub const SHOWN_HEADERS: [&str; 3] = [
    OUTCOME_HEADER,
    "x-electoral-log-stage",
    "x-electoral-log-error",
];
/// Longest header value returned.
const MAX_HEADER_CHARS: usize = 500;
/// Arguments shown for electoral-log events: they identify the event, not the voter.
const SHOWN_EVENT_FIELDS: [&str; 3] = ["tenant_id", "election_event_id", "message_type"];
/// Outcome reported for an archived message without the header.
pub const UNKNOWN_OUTCOME: &str = "unknown";

#[derive(Debug)]
pub enum InspectError {
    InvalidQueueName(String),
    InvalidRange(&'static str),
    Database(tokio_postgres::Error),
}

impl std::fmt::Display for InspectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InspectError::InvalidQueueName(reason) => f.write_str(reason),
            InspectError::InvalidRange(reason) => f.write_str(reason),
            InspectError::Database(_) => f.write_str("task-queue database query failed"),
        }
    }
}

impl std::error::Error for InspectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InspectError::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<tokio_postgres::Error> for InspectError {
    fn from(error: tokio_postgres::Error) -> Self {
        InspectError::Database(error)
    }
}

fn checked(queue: &str) -> Result<&str, InspectError> {
    validate_queue_name(queue).map_err(InspectError::InvalidQueueName)?;
    Ok(queue)
}

/// A queue's state now and its outcomes over the last hour.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueueOverview {
    pub queue: String,
    /// Messages ready to be read.
    pub ready: i64,
    /// Messages being processed, or waiting for their ETA or retry delay.
    pub running_or_scheduled: i64,
    pub oldest_age_secs: Option<i32>,
    /// Messages ever sent to the queue.
    pub total_sent: i64,
    /// Messages archived in the last hour, by outcome.
    pub last_hour: BTreeMap<String, i64>,
}

pub async fn overview<C: GenericClient + Sync>(
    client: &C,
    queues: &[&str],
) -> Result<Vec<QueueOverview>, InspectError> {
    let mut result = Vec::with_capacity(queues.len());
    for queue in queues {
        let queue = checked(queue)?;
        let metrics = client
            .query_one(
                "SELECT queue_length, queue_visible_length, oldest_msg_age_sec, total_messages \
                 FROM pgmq.metrics($1)",
                &[&queue],
            )
            .await?;
        let queue_length: i64 = metrics.get(0);
        let ready: i64 = metrics.get(1);
        let last_hour = client
            .query(
                &format!(
                    "SELECT COALESCE(headers->>'{OUTCOME_HEADER}', '{UNKNOWN_OUTCOME}'), count(*) \
                     FROM pgmq.a_{queue} \
                     WHERE archived_at > clock_timestamp() - interval '1 hour' GROUP BY 1"
                ),
                &[],
            )
            .await?
            .iter()
            .map(|row| (row.get::<_, String>(0), row.get::<_, i64>(1)))
            .collect();
        result.push(QueueOverview {
            queue: queue.to_string(),
            ready,
            running_or_scheduled: queue_length - ready,
            oldest_age_secs: metrics.get(2),
            total_sent: metrics.get(3),
            last_hour,
        });
    }
    Ok(result)
}

/// The messages archived in one time bucket.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThroughputBucket {
    pub start: DateTime<Utc>,
    pub outcomes: BTreeMap<String, i64>,
    /// Mean seconds from the last read to archival: roughly the processing time.
    pub mean_processing_secs: Option<f64>,
    /// Mean seconds from enqueueing to the last read.
    pub mean_wait_secs: Option<f64>,
}

/// Archived messages of the last `period`, in buckets of `bucket`, oldest first.
pub async fn throughput<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    period: Duration,
    bucket: Duration,
) -> Result<Vec<ThroughputBucket>, InspectError> {
    let queue = checked(queue)?;
    if bucket.as_secs() == 0 || period < bucket {
        return Err(InspectError::InvalidRange(
            "the bucket must be at least a second and at most the period",
        ));
    }
    let rows = client
        .query(
            &format!(
                "SELECT to_timestamp(floor(extract(epoch FROM archived_at)::float8 / $2::float8) * $2::float8) AS start, \
                 COALESCE(headers->>'{OUTCOME_HEADER}', '{UNKNOWN_OUTCOME}') AS outcome, \
                 count(*), \
                 avg(extract(epoch FROM archived_at - last_read_at))::float8, \
                 avg(extract(epoch FROM last_read_at - enqueued_at))::float8 \
                 FROM pgmq.a_{queue} \
                 WHERE archived_at > clock_timestamp() - make_interval(secs => $1::float8) \
                 GROUP BY 1, 2 ORDER BY 1, 2"
            ),
            &[&period.as_secs_f64(), &bucket.as_secs_f64()],
        )
        .await?;
    Ok(buckets(&rows))
}

fn buckets(rows: &[Row]) -> Vec<ThroughputBucket> {
    // Means are weighted by each outcome's count within the bucket.
    let mut result: Vec<(ThroughputBucket, f64, f64, i64, i64)> = Vec::new();
    for row in rows {
        let start: DateTime<Utc> = row.get(0);
        let count: i64 = row.get(2);
        let processing: Option<f64> = row.get(3);
        let wait: Option<f64> = row.get(4);
        if result.last().map(|(bucket, ..)| bucket.start) != Some(start) {
            result.push((
                ThroughputBucket {
                    start,
                    outcomes: BTreeMap::new(),
                    mean_processing_secs: None,
                    mean_wait_secs: None,
                },
                0.0,
                0.0,
                0,
                0,
            ));
        }
        let (bucket, processing_sum, wait_sum, processing_count, wait_count) =
            result.last_mut().expect("a bucket was just pushed");
        bucket.outcomes.insert(row.get(1), count);
        if let Some(processing) = processing {
            *processing_sum += processing * count as f64;
            *processing_count += count;
        }
        if let Some(wait) = wait {
            *wait_sum += wait * count as f64;
            *wait_count += count;
        }
    }
    result
        .into_iter()
        .map(
            |(mut bucket, processing_sum, wait_sum, processing_count, wait_count)| {
                bucket.mean_processing_secs =
                    (processing_count > 0).then(|| processing_sum / processing_count as f64);
                bucket.mean_wait_secs = (wait_count > 0).then(|| wait_sum / wait_count as f64);
                bucket
            },
        )
        .collect()
}

/// Which of a queue's messages to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageState {
    /// Waiting, being processed or scheduled.
    Queued,
    Archived,
}

/// A message, without its task's arguments.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MessageSummary {
    pub msg_id: i64,
    pub read_count: i32,
    pub enqueued_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
    /// When the message is next visible; for archived messages, when it was last.
    pub visible_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    /// The Celery task, or `None` if the message is not a readable Celery envelope.
    pub task: Option<String>,
    pub task_id: Option<String>,
    pub retries: Option<u32>,
    pub eta: Option<DateTime<Utc>>,
    pub expires: Option<DateTime<Utc>>,
    pub size_bytes: usize,
    pub headers: BTreeMap<String, String>,
    /// For electoral-log events: the event's tenant, election event and type.
    pub event: BTreeMap<String, String>,
}

/// Up to `limit` messages, newest first, with an ID below `before` if given.
pub async fn messages<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    state: MessageState,
    before: Option<i64>,
    limit: i64,
) -> Result<Vec<MessageSummary>, InspectError> {
    let queue = checked(queue)?;
    if !(1..=500).contains(&limit) {
        return Err(InspectError::InvalidRange(
            "the limit must be between 1 and 500",
        ));
    }
    let (table, archived_at) = match state {
        MessageState::Queued => ("q", "NULL::timestamptz"),
        MessageState::Archived => ("a", "archived_at"),
    };
    let rows = client
        .query(
            &format!(
                "SELECT msg_id, read_ct, enqueued_at, last_read_at, vt, {archived_at}, \
                 message, headers FROM pgmq.{table}_{queue} \
                 WHERE $1::bigint IS NULL OR msg_id < $1 ORDER BY msg_id DESC LIMIT $2"
            ),
            &[&before, &limit],
        )
        .await?;
    Ok(rows.iter().map(summarize).collect())
}

fn summarize(row: &Row) -> MessageSummary {
    let message: Option<Value> = row.get(6);
    let headers: Option<Value> = row.get(7);
    let size_bytes = message
        .as_ref()
        .map(|message| message.to_string().len())
        .unwrap_or_default();
    let decoded = message.and_then(|message| decode(message).ok());
    MessageSummary {
        msg_id: row.get(0),
        read_count: row.get(1),
        enqueued_at: row.get(2),
        last_read_at: row.get(3),
        visible_at: row.get(4),
        archived_at: row.get(5),
        task: decoded.as_ref().map(|message| message.headers.task.clone()),
        task_id: decoded.as_ref().map(|message| message.headers.id.clone()),
        retries: decoded.as_ref().and_then(|message| message.headers.retries),
        eta: decoded.as_ref().and_then(|message| message.headers.eta),
        expires: decoded.as_ref().and_then(|message| message.headers.expires),
        size_bytes,
        headers: shown_headers(headers.as_ref()),
        event: decoded
            .map(|message| event_fields(&message.raw_body))
            .unwrap_or_default(),
    }
}

fn shown_headers(headers: Option<&Value>) -> BTreeMap<String, String> {
    SHOWN_HEADERS
        .iter()
        .filter_map(|name| {
            let value = headers?.get(*name)?;
            let text = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            Some((
                name.to_string(),
                text.chars().take(MAX_HEADER_CHARS).collect(),
            ))
        })
        .collect()
}

/// The identifying fields of an electoral-log event in a task body, `[args, {input}, …]`.
fn event_fields(body: &[u8]) -> BTreeMap<String, String> {
    let Ok(body) = serde_json::from_slice::<Value>(body) else {
        return BTreeMap::new();
    };
    let Some(input) = body.get(1).and_then(|kwargs| kwargs.get("input")) else {
        return BTreeMap::new();
    };
    SHOWN_EVENT_FIELDS
        .iter()
        .filter_map(|field| {
            let value = input.get(*field)?.as_str()?;
            Some((
                field.to_string(),
                value.chars().take(MAX_HEADER_CHARS).collect(),
            ))
        })
        .collect()
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn only_listed_headers_are_shown_and_long_values_are_cut() {
        let headers = serde_json::json!({
            OUTCOME_HEADER: "failed",
            "x-electoral-log-error": "é".repeat(MAX_HEADER_CHARS + 10),
            "authorization": "secret",
        });
        let shown = shown_headers(Some(&headers));
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[OUTCOME_HEADER], "failed");
        assert_eq!(
            shown["x-electoral-log-error"].chars().count(),
            MAX_HEADER_CHARS
        );
        assert!(shown_headers(None).is_empty());
    }

    #[test]
    fn events_show_their_identifiers_but_not_the_voter() {
        let body = serde_json::to_vec(&serde_json::json!([[], {"input": {
            "tenant_id": "tenant",
            "election_event_id": "event",
            "message_type": "LOGIN",
            "user_id": "voter",
            "username": "voter@example.com",
            "body": "secret",
        }}, {}]))
        .unwrap();
        let fields = event_fields(&body);
        assert_eq!(
            fields.keys().collect::<Vec<_>>(),
            ["election_event_id", "message_type", "tenant_id"]
        );
        assert!(event_fields(b"not json").is_empty());
        assert!(event_fields(b"[[], {}]").is_empty());
    }
}
