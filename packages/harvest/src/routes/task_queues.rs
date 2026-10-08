// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The Task Queues page of the super-admin tenant: the environment's queues, their
//! recent outcomes and throughput, their messages without the tasks' arguments, the
//! replay or discard of dead-lettered electoral-log events, and read-only SQL queries.

use crate::routes::electoral_log_console::ConsoleQueryOutput;
use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use electoral_log::adapters::console::run_read_only_query;
use pgmq_broker::inspect::{
    self, InspectError, MessageState, MessageSummary, QueueOverview,
    ThroughputBucket,
};
use rocket::{http::Status, serde::json::Json};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use tracing::instrument;
use windmill::services::celery_app::{get_celery_app, Queue};
use windmill::services::database::get_queue_reader_pool;
use windmill::services::electoral_log_dead_letter::DeadLetterOperation;
use windmill::tasks::manage_electoral_log_dead_letters::manage_electoral_log_dead_letters;

/// The period and bucket of a throughput graph unless the request gives others.
const DEFAULT_THROUGHPUT_HOURS: u32 = 24;
const DEFAULT_BUCKET_MINUTES: u32 = 15;
/// The longest period a graph covers: the default archive retention.
const MAX_THROUGHPUT_HOURS: u32 = 7 * 24;
const DEFAULT_MESSAGE_LIMIT: i64 = 50;
/// The most dead-lettered events one request replays or discards.
const MAX_DEAD_LETTER_IDS: usize = 1_000;
/// The most rows a query returns, its time limit and its longest text.
const QUERY_MAX_ROWS: usize = 1_000;
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
const QUERY_MAX_CHARACTERS: usize = 20_000;

fn internal_error(error: impl std::fmt::Debug) -> JsonError {
    tracing::error!("Task Queues request failed: {error:?}");
    ErrorResponse::new(
        Status::InternalServerError,
        "Task Queues request failed",
        ErrorCode::InternalServerError,
    )
}

fn bad_request(message: &str) -> JsonError {
    ErrorResponse::new(
        Status::BadRequest,
        message,
        ErrorCode::InvalidTaskQueuesRequest,
    )
}

fn inspect_error(error: InspectError) -> JsonError {
    match error {
        InspectError::InvalidQueueName(reason) => bad_request(&reason),
        InspectError::InvalidRange(reason) => bad_request(reason),
        error => internal_error(error),
    }
}

/// Only users of the super-admin tenant with the permissions get through.
fn authorize_super_admin(
    claims: &JwtClaims,
    permissions: Vec<Permissions>,
) -> Result<(), JsonError> {
    let names: Vec<String> = permissions
        .iter()
        .map(|permission| permission.to_string())
        .collect();
    authorize(claims, true, None, permissions).map_err(|(status, _)| {
        ErrorResponse::new(
            status,
            &format!(
                "This needs a user of the super-admin tenant with {}",
                names.join(", ")
            ),
            ErrorCode::Unauthorized,
        )
    })
}

/// One of the environment's queues.
fn known_queue(name: &str) -> Result<Queue, JsonError> {
    Queue::from_str(name).map_err(|_| bad_request("Unknown queue"))
}

async fn reader() -> Result<deadpool_postgres::Object, JsonError> {
    let pool = get_queue_reader_pool().await.map_err(|error| {
        tracing::error!("Task-queue inspection is not configured: {error:#}");
        ErrorResponse::new(
            Status::ServiceUnavailable,
            "Task-queue inspection is not configured",
            ErrorCode::TaskQueuesUnavailable,
        )
    })?;
    pool.get().await.map_err(internal_error)
}

#[derive(Serialize, Debug)]
pub struct OverviewOutput {
    queues: Vec<QueueOverview>,
}

#[instrument(skip(claims))]
#[post("/task-queues/overview", format = "json")]
pub async fn task_queues_overview(
    claims: JwtClaims,
) -> Result<Json<OverviewOutput>, JsonError> {
    authorize_super_admin(&claims, vec![Permissions::TASK_QUEUES_READ])?;
    let client = reader().await?;
    let names = Queue::all_names();
    let queues = inspect::overview(&**client, &names)
        .await
        .map_err(inspect_error)?;
    Ok(Json(OverviewOutput { queues }))
}

#[derive(Deserialize, Debug)]
pub struct ThroughputInput {
    queue: String,
    hours: Option<u32>,
    bucket_minutes: Option<u32>,
}

#[derive(Serialize, Debug)]
pub struct ThroughputOutput {
    queue: String,
    hours: u32,
    bucket_minutes: u32,
    buckets: Vec<ThroughputBucket>,
}

/// The period and bucket of a graph, in hours and minutes.
fn throughput_range(input: &ThroughputInput) -> Result<(u32, u32), JsonError> {
    let hours = input.hours.unwrap_or(DEFAULT_THROUGHPUT_HOURS);
    let bucket_minutes = input.bucket_minutes.unwrap_or(DEFAULT_BUCKET_MINUTES);
    if !(1..=MAX_THROUGHPUT_HOURS).contains(&hours) {
        return Err(bad_request(&format!(
            "Give a period of 1 to {MAX_THROUGHPUT_HOURS} hours"
        )));
    }
    if bucket_minutes == 0 || bucket_minutes > hours * 60 {
        return Err(bad_request(
            "Give buckets of at least a minute and at most the period",
        ));
    }
    Ok((hours, bucket_minutes))
}

#[instrument(skip(claims))]
#[post("/task-queues/throughput", format = "json", data = "<body>")]
pub async fn task_queues_throughput(
    body: Json<ThroughputInput>,
    claims: JwtClaims,
) -> Result<Json<ThroughputOutput>, JsonError> {
    authorize_super_admin(&claims, vec![Permissions::TASK_QUEUES_READ])?;
    let input = body.into_inner();
    let queue = known_queue(&input.queue)?;
    let (hours, bucket_minutes) = throughput_range(&input)?;
    let client = reader().await?;
    let buckets = inspect::throughput(
        &**client,
        queue.queue_name(),
        Duration::from_secs(u64::from(hours) * 3600),
        Duration::from_secs(u64::from(bucket_minutes) * 60),
    )
    .await
    .map_err(inspect_error)?;
    Ok(Json(ThroughputOutput {
        queue: queue.queue_name().into(),
        hours,
        bucket_minutes,
        buckets,
    }))
}

#[derive(Deserialize, Debug)]
pub struct MessagesInput {
    queue: String,
    /// `queued` or `archived`.
    state: String,
    /// Lists messages with a lower ID, for the next page.
    before: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct MessagesOutput {
    messages: Vec<MessageSummary>,
}

fn message_state(state: &str) -> Result<MessageState, JsonError> {
    match state {
        "queued" => Ok(MessageState::Queued),
        "archived" => Ok(MessageState::Archived),
        _ => Err(bad_request("Unknown state: give queued or archived")),
    }
}

fn message_id(id: &str) -> Result<i64, JsonError> {
    id.trim()
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| bad_request("Invalid message ID"))
}

#[instrument(skip(claims))]
#[post("/task-queues/messages", format = "json", data = "<body>")]
pub async fn task_queues_messages(
    body: Json<MessagesInput>,
    claims: JwtClaims,
) -> Result<Json<MessagesOutput>, JsonError> {
    authorize_super_admin(&claims, vec![Permissions::TASK_QUEUES_READ])?;
    let input = body.into_inner();
    let queue = known_queue(&input.queue)?;
    let state = message_state(&input.state)?;
    let before = input.before.as_deref().map(message_id).transpose()?;
    let client = reader().await?;
    let messages = inspect::messages(
        &**client,
        queue.queue_name(),
        state,
        before,
        input.limit.unwrap_or(DEFAULT_MESSAGE_LIMIT),
    )
    .await
    .map_err(inspect_error)?;
    Ok(Json(MessagesOutput { messages }))
}

#[derive(Deserialize, Debug)]
pub struct DeadLettersInput {
    /// `replay` or `discard`.
    operation: String,
    message_ids: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct DeadLettersOutput {
    task_id: String,
    requested: usize,
}

fn dead_letter_request(
    input: &DeadLettersInput,
) -> Result<(DeadLetterOperation, Vec<i64>), JsonError> {
    let operation =
        DeadLetterOperation::from_str(&input.operation).map_err(|_| {
            bad_request("Unknown operation: give replay or discard")
        })?;
    if input.message_ids.is_empty()
        || input.message_ids.len() > MAX_DEAD_LETTER_IDS
    {
        return Err(bad_request(&format!(
            "Give 1 to {MAX_DEAD_LETTER_IDS} message IDs"
        )));
    }
    let mut ids = input
        .message_ids
        .iter()
        .map(|id| message_id(id))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort_unstable();
    ids.dedup();
    Ok((operation, ids))
}

/// Queues the operation for a worker: Harvest may only enqueue tasks.
#[instrument(skip(claims))]
#[post("/task-queues/dead-letters", format = "json", data = "<body>")]
pub async fn task_queues_dead_letters(
    body: Json<DeadLettersInput>,
    claims: JwtClaims,
) -> Result<Json<DeadLettersOutput>, JsonError> {
    authorize_super_admin(
        &claims,
        vec![
            Permissions::TASK_QUEUES_READ,
            Permissions::TASK_QUEUES_WRITE,
        ],
    )?;
    let (operation, ids) = dead_letter_request(&body.into_inner())?;
    let requested = ids.len();
    let requested_by = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task = get_celery_app()
        .await
        .send_task(manage_electoral_log_dead_letters::new(
            operation,
            ids,
            requested_by,
        ))
        .await
        .map_err(internal_error)?;
    Ok(Json(DeadLettersOutput {
        task_id: task.task_id,
        requested,
    }))
}

#[derive(Deserialize, Debug)]
pub struct QueryInput {
    sql: String,
}

fn check_query(sql: &str) -> Result<(), JsonError> {
    if sql.trim().is_empty() || sql.chars().count() > QUERY_MAX_CHARACTERS {
        return Err(bad_request(&format!(
            "Give a query of at most {QUERY_MAX_CHARACTERS} characters"
        )));
    }
    Ok(())
}

/// Run a read-only SQL query on the environment's task-queue database, as its
/// reader role, with a time limit. Queries read the messages as they are stored,
/// with their tasks' arguments, so they have a permission of their own.
#[instrument(skip(claims, body))]
#[post("/task-queues/query", format = "json", data = "<body>")]
pub async fn task_queues_query(
    body: Json<QueryInput>,
    claims: JwtClaims,
) -> Result<Json<ConsoleQueryOutput>, JsonError> {
    authorize_super_admin(&claims, vec![Permissions::TASK_QUEUES_QUERY])?;
    let sql = body.into_inner().sql;
    check_query(&sql)?;
    tracing::info!(
        tenant_id = %claims.hasura_claims.tenant_id,
        user_id = %claims.hasura_claims.user_id,
        sql = %sql,
        "Task-queue query"
    );
    let mut client = reader().await?;
    let output = match run_read_only_query(
        &mut client,
        &sql,
        QUERY_MAX_ROWS,
        QUERY_TIMEOUT,
    )
    .await
    {
        Ok(result) => ConsoleQueryOutput::Rows(result),
        Err(error) => ConsoleQueryOutput::Error {
            error: format!("{error:#}"),
        },
    };
    Ok(Json(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(result: Result<impl std::fmt::Debug, JsonError>) -> Status {
        result.expect_err("expected a refusal").0
    }

    fn accepted<T>(result: Result<T, JsonError>) -> T {
        result.unwrap_or_else(|refusal| panic!("refused with {}", refusal.0))
    }

    #[test]
    fn graphs_cover_at_most_the_retention_in_buckets_within_the_period() {
        let input = |hours, bucket_minutes| ThroughputInput {
            queue: "beat".into(),
            hours,
            bucket_minutes,
        };
        assert_eq!(
            accepted(throughput_range(&input(None, None))),
            (DEFAULT_THROUGHPUT_HOURS, DEFAULT_BUCKET_MINUTES)
        );
        assert_eq!(
            accepted(throughput_range(&input(Some(1), Some(60)))),
            (1, 60)
        );
        for (hours, minutes) in [
            (Some(0), None),
            (Some(MAX_THROUGHPUT_HOURS + 1), None),
            (Some(1), Some(61)),
            (None, Some(0)),
        ] {
            assert_eq!(
                status(throughput_range(&input(hours, minutes))),
                Status::BadRequest
            );
        }
    }

    #[test]
    fn queues_states_and_ids_are_checked() {
        assert_eq!(
            accepted(known_queue("electoral_log_dead_letter_queue")),
            Queue::ElectoralLogDeadLetter
        );
        assert_eq!(status(known_queue("dev_beat")), Status::BadRequest);
        assert_eq!(accepted(message_state("queued")), MessageState::Queued);
        assert_eq!(status(message_state("waiting")), Status::BadRequest);
        assert_eq!(accepted(message_id(" 42 ")), 42);
        for invalid in ["0", "-1", "x", ""] {
            assert_eq!(status(message_id(invalid)), Status::BadRequest);
        }
    }

    #[test]
    fn dead_letter_requests_name_an_operation_and_some_ids() {
        let input = |operation: &str, ids: &[&str]| DeadLettersInput {
            operation: operation.into(),
            message_ids: ids.iter().map(|id| id.to_string()).collect(),
        };
        assert_eq!(
            accepted(dead_letter_request(&input("replay", &["3", "1", "3"]))),
            (DeadLetterOperation::Replay, vec![1, 3])
        );
        assert_eq!(
            accepted(dead_letter_request(&input("discard", &["2"]))).0,
            DeadLetterOperation::Discard
        );
        let too_many: Vec<String> = (1..=MAX_DEAD_LETTER_IDS + 1)
            .map(|id| id.to_string())
            .collect();
        for invalid in [
            input("delete", &["1"]),
            input("replay", &[]),
            input("replay", &["1", "x"]),
            DeadLettersInput {
                operation: "replay".into(),
                message_ids: too_many,
            },
        ] {
            assert_eq!(
                status(dead_letter_request(&invalid)),
                Status::BadRequest
            );
        }
    }

    #[test]
    fn queries_have_text_within_the_limit() {
        assert!(check_query("SELECT 1").is_ok());
        assert_eq!(status(check_query("  \n")), Status::BadRequest);
        let long = "x".repeat(QUERY_MAX_CHARACTERS + 1);
        assert_eq!(status(check_query(&long)), Status::BadRequest);
        assert!(check_query(&"é".repeat(QUERY_MAX_CHARACTERS)).is_ok());
    }
}
