// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The electoral-log console: administrators browse an election event's records
//! and ballot box page by page, open a record, and run read-only SQL queries on
//! their tenant's electoral-log database.

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use electoral_log::adapters::console::{
    run_read_only_query, ConsoleFilters, ConsoleTable, Page, PageOrder,
    PageRequest, PersonalData, QueryResult,
};
use rocket::{http::Status, serde::json::Json};
use sequent_core::{
    services::{jwt::JwtClaims, uuid_validation::parse_uuid_v4},
    types::permissions::Permissions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::election_event::get_election_event_by_id_if_exist;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::protocol_manager::{
    get_electoral_log_router, get_electoral_log_store,
};

/// Rows of a page unless the request asks for another number.
const DEFAULT_PAGE_ROWS: i64 = 25;
/// Rows of a query's result that are returned.
const QUERY_MAX_ROWS: usize = 1_000;
/// How long a query may run. The Hasura action waits a little longer.
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
/// The longest query accepted, in characters.
const QUERY_MAX_CHARACTERS: usize = 20_000;

fn internal_error(error: impl std::fmt::Debug) -> JsonError {
    tracing::error!("Electoral-log console request failed: {error:?}");
    ErrorResponse::new(
        Status::InternalServerError,
        "Electoral-log console request failed",
        ErrorCode::InternalServerError,
    )
}

fn bad_request(message: &str) -> JsonError {
    ErrorResponse::new(
        Status::BadRequest,
        message,
        ErrorCode::InvalidElectoralLogConsoleRequest,
    )
}

/// Let the request through if the user has the permissions in their own tenant.
fn authorize_tenant(
    claims: &JwtClaims,
    permissions: Vec<Permissions>,
) -> Result<String, JsonError> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let names: Vec<String> = permissions
        .iter()
        .map(|permission| permission.to_string())
        .collect();
    authorize(claims, true, Some(tenant_id.clone()), permissions).map_err(
        |(status, _)| {
            ErrorResponse::new(
                status,
                &format!("This needs the permissions {}", names.join(", ")),
                ErrorCode::Unauthorized,
            )
        },
    )?;
    Ok(tenant_id)
}

/// Whether the user may see voters' personal data.
fn personal_data(claims: &JwtClaims) -> PersonalData {
    match authorize_tenant(
        claims,
        vec![Permissions::ELECTORAL_LOG_PERSONAL_DATA_READ],
    ) {
        Ok(_) => PersonalData::Shown,
        Err(_) => PersonalData::Hidden,
    }
}

/// The table and order a page request names.
fn table_and_order(
    table: &str,
    order: Option<&str>,
) -> Result<(ConsoleTable, PageOrder), JsonError> {
    let table = table.parse::<ConsoleTable>().map_err(|_| {
        bad_request("Unknown table: give records, ballots, voters or queue")
    })?;
    let order = order
        .map(str::parse::<PageOrder>)
        .transpose()
        .map_err(|_| {
            bad_request("Unknown order: give newest-first or oldest-first")
        })?
        .unwrap_or_default();
    Ok((table, order))
}

/// The ballot box keys elections and areas by UUID; records keep them as text.
fn check_filters(
    table: ConsoleTable,
    filters: &ConsoleFilters,
) -> Result<(), JsonError> {
    if table == ConsoleTable::Records {
        return Ok(());
    }
    for (name, value) in [
        ("election", &filters.election_id),
        ("area", &filters.area_id),
    ] {
        match value.as_deref() {
            Some(value)
                if !value.is_empty() && Uuid::parse_str(value).is_err() =>
            {
                return Err(bad_request(&format!("Invalid {name} ID filter")));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Whether a query is one the console runs.
fn check_query(sql: &str) -> Result<(), JsonError> {
    if sql.trim().is_empty() || sql.chars().count() > QUERY_MAX_CHARACTERS {
        return Err(bad_request(&format!(
            "Give a query of at most {QUERY_MAX_CHARACTERS} characters"
        )));
    }
    Ok(())
}

/// The board of an election event of the tenant.
async fn event_board(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<String, JsonError> {
    if parse_uuid_v4(election_event_id).is_err() {
        return Err(ErrorResponse::new(
            Status::BadRequest,
            "Invalid election event ID",
            ErrorCode::UuidParseFailed,
        ));
    }
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(internal_error)?;
    let transaction = client.transaction().await.map_err(internal_error)?;
    let event = get_election_event_by_id_if_exist(
        &transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(internal_error)?
    .ok_or_else(|| {
        ErrorResponse::new(
            Status::NotFound,
            "Election event not found",
            ErrorCode::ElectionEventNotFound,
        )
    })?;
    transaction.commit().await.map_err(internal_error)?;
    get_election_event_board(event.bulletin_board_reference).ok_or_else(|| {
        ErrorResponse::new(
            Status::BadRequest,
            "Election event has no electoral log",
            ErrorCode::ElectoralLogNotFound,
        )
    })
}

#[derive(Debug, Deserialize)]
pub struct ConsolePageInput {
    election_event_id: String,
    /// `records`, `ballots`, `voters` or `queue`.
    table: String,
    filters: Option<ConsoleFilters>,
    /// `newest-first` (the default) or `oldest-first`.
    order: Option<String>,
    /// The `next` of the previous page.
    after: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ConsolePageOutput {
    #[serde(flatten)]
    page: Page,
    personal_data: PersonalData,
}

/// A page of the records of an election event's electoral log, or of its ballot
/// box. Without the personal-data permission, personal columns read `hidden`.
#[instrument(skip(claims))]
#[post("/electoral-log-console/page", format = "json", data = "<body>")]
pub async fn electoral_log_console_page(
    body: Json<ConsolePageInput>,
    claims: JwtClaims,
) -> Result<Json<ConsolePageOutput>, JsonError> {
    let tenant_id = authorize_tenant(
        &claims,
        vec![Permissions::ELECTORAL_LOG_CONSOLE_READ],
    )?;
    let input = body.into_inner();
    let (table, order) = table_and_order(&input.table, input.order.as_deref())?;
    let filters = input.filters.unwrap_or_default();
    check_filters(table, &filters)?;
    let board = event_board(&tenant_id, &input.election_event_id).await?;
    let store = get_electoral_log_store(&board)
        .await
        .map_err(internal_error)?;
    let mut page = store
        .console_page(&PageRequest {
            table,
            board: &board,
            election_event_id: &input.election_event_id,
            filters: &filters,
            order,
            after: input.after.as_deref(),
            limit: input.limit.unwrap_or(DEFAULT_PAGE_ROWS),
        })
        .await
        .map_err(internal_error)?;
    let personal_data = personal_data(&claims);
    if personal_data == PersonalData::Hidden {
        page.hide_personal_data();
    }
    Ok(Json(ConsolePageOutput {
        page,
        personal_data,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ConsoleRecordInput {
    election_event_id: String,
    position: i64,
}

/// A record of an election event's electoral log with its message decoded.
#[instrument(skip(claims))]
#[post("/electoral-log-console/record", format = "json", data = "<body>")]
pub async fn electoral_log_console_record(
    body: Json<ConsoleRecordInput>,
    claims: JwtClaims,
) -> Result<Json<Value>, JsonError> {
    let tenant_id = authorize_tenant(
        &claims,
        vec![Permissions::ELECTORAL_LOG_CONSOLE_READ],
    )?;
    let input = body.into_inner();
    let board = event_board(&tenant_id, &input.election_event_id).await?;
    let store = get_electoral_log_store(&board)
        .await
        .map_err(internal_error)?;
    let record = store
        .console_record(&board, input.position)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ErrorResponse::new(
                Status::NotFound,
                "Record not found",
                ErrorCode::ElectoralLogRecordNotFound,
            )
        })?;
    let personal_data = personal_data(&claims);
    let mut json = record.to_json(personal_data).map_err(internal_error)?;
    json["personal_data"] =
        serde_json::to_value(personal_data).map_err(internal_error)?;
    Ok(Json(json))
}

#[derive(Deserialize)]
pub struct ConsoleQueryInput {
    sql: String,
}

/// A query's rows, or why it did not run.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ConsoleQueryOutput {
    Rows(QueryResult),
    Error { error: String },
}

/// Run a read-only SQL query on the tenant's electoral-log database, as a role
/// that can only read, with a time limit. Queries read personal data as they
/// are stored, so they need the personal-data permission too.
#[instrument(skip(claims, body))]
#[post("/electoral-log-console/query", format = "json", data = "<body>")]
pub async fn electoral_log_console_query(
    body: Json<ConsoleQueryInput>,
    claims: JwtClaims,
) -> Result<Json<ConsoleQueryOutput>, JsonError> {
    let tenant_id = authorize_tenant(
        &claims,
        vec![
            Permissions::ELECTORAL_LOG_CONSOLE_QUERY,
            Permissions::ELECTORAL_LOG_PERSONAL_DATA_READ,
        ],
    )?;
    let sql = body.into_inner().sql;
    check_query(&sql)?;
    tracing::info!(
        tenant_id = %tenant_id,
        user_id = %claims.hasura_claims.user_id,
        sql = %sql,
        "Electoral-log console query"
    );
    let router = get_electoral_log_router().await.map_err(internal_error)?;
    let mut client = match router.reader_client(&tenant_id).await {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(
                "Electoral-log console queries are not available: {error:?}"
            );
            return Ok(Json(ConsoleQueryOutput::Error {
                error: "Queries are not available in this environment".into(),
            }));
        }
    };
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

    #[test]
    fn queries_must_have_text_within_the_limit() {
        assert!(check_query("SELECT 1").is_ok());
        assert_eq!(check_query("  \n").unwrap_err().0, Status::BadRequest);
        let long = "x".repeat(QUERY_MAX_CHARACTERS + 1);
        assert_eq!(check_query(&long).unwrap_err().0, Status::BadRequest);
        assert!(check_query(&"é".repeat(QUERY_MAX_CHARACTERS)).is_ok());
    }

    #[test]
    fn pages_name_tables_and_orders_in_kebab_case() {
        assert_eq!(
            table_and_order("queue", Some("oldest-first")).ok().unwrap(),
            (ConsoleTable::Queue, PageOrder::OldestFirst)
        );
        assert_eq!(
            table_and_order("records", None).ok().unwrap(),
            (ConsoleTable::Records, PageOrder::NewestFirst)
        );
        for (table, order) in
            [("cast_vote", None), ("ballots", Some("newest_first"))]
        {
            let error = table_and_order(table, order).unwrap_err();
            assert_eq!(error.0, Status::BadRequest);
        }
    }

    #[test]
    fn ballot_box_filters_need_uuids() {
        let filters = ConsoleFilters {
            election_id: Some("north".into()),
            area_id: Some(String::new()),
            ..Default::default()
        };
        assert!(check_filters(ConsoleTable::Records, &filters).is_ok());
        for table in [
            ConsoleTable::Ballots,
            ConsoleTable::Voters,
            ConsoleTable::Queue,
        ] {
            assert_eq!(
                check_filters(table, &filters).unwrap_err().0,
                Status::BadRequest
            );
        }
        let valid = ConsoleFilters {
            election_id: Some(Uuid::new_v4().to_string()),
            ..filters
        };
        assert!(check_filters(ConsoleTable::Ballots, &valid).is_ok());
    }

    #[test]
    fn query_outputs_are_rows_or_an_error() {
        let error = serde_json::to_value(ConsoleQueryOutput::Error {
            error: "syntax error".into(),
        })
        .unwrap();
        assert_eq!(error, serde_json::json!({"error": "syntax error"}));
    }
}
