// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The electoral-log console: administrators browse an election event's records
//! and ballot box page by page and open a record. Users of the super-admin tenant
//! browse any tenant's events and run read-only SQL queries on the electoral-log
//! database, which holds every tenant's data.

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use electoral_log::adapters::console::{
    run_read_only_query, ConsoleFilters, ConsoleTable, Page, PageOrder,
    PageRequest, PersonalData, QueryResult,
};
use electoral_log::adapters::postgres::PostgresConnection;
use rocket::{http::Status, serde::json::Json};
use sequent_core::{
    services::{jwt::JwtClaims, uuid_validation::parse_uuid_v4},
    types::permissions::Permissions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::election_event::get_election_event_by_id_if_exist;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::protocol_manager::get_electoral_log_store;

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

fn unauthorized(
    status: Status,
    scope: &str,
    permissions: &[Permissions],
) -> JsonError {
    let names: Vec<String> = permissions
        .iter()
        .map(|permission| permission.to_string())
        .collect();
    ErrorResponse::new(
        status,
        &format!(
            "This needs {scope} with the permissions {}",
            names.join(", ")
        ),
        ErrorCode::Unauthorized,
    )
}

/// The tenant a request reads: the user's own, or, for users of the super-admin
/// tenant, the one the request names.
fn requested_tenant(
    claims: &JwtClaims,
    requested: Option<&str>,
) -> Result<String, JsonError> {
    match requested.map(str::trim).filter(|tenant| !tenant.is_empty()) {
        None => Ok(claims.hasura_claims.tenant_id.clone()),
        Some(tenant) => Uuid::parse_str(tenant)
            .map(|_| tenant.to_string())
            .map_err(|_| bad_request("Invalid tenant ID")),
    }
}

/// Let the request through if the user has the permissions in the tenant, which is
/// their own unless they belong to the super-admin tenant.
fn authorize_tenant(
    claims: &JwtClaims,
    tenant_id: &str,
    permissions: Vec<Permissions>,
) -> Result<(), JsonError> {
    authorize(
        claims,
        true,
        Some(tenant_id.to_string()),
        permissions.clone(),
    )
    .map_err(|(status, _)| {
        unauthorized(status, "access to the tenant", &permissions)
    })
}

/// Let the request through if the user belongs to the super-admin tenant and has
/// the permissions.
fn authorize_super_admin(
    claims: &JwtClaims,
    permissions: Vec<Permissions>,
) -> Result<(), JsonError> {
    authorize(claims, true, None, permissions.clone()).map_err(|(status, _)| {
        unauthorized(status, "a user of the super-admin tenant", &permissions)
    })
}

/// Whether the user may see voters' personal data.
fn personal_data(claims: &JwtClaims) -> PersonalData {
    let own_tenant = claims.hasura_claims.tenant_id.clone();
    match authorize_tenant(
        claims,
        &own_tenant,
        vec![Permissions::ELECTORAL_LOG_PERSONAL_DATA_READ],
    ) {
        Ok(()) => PersonalData::Shown,
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

#[derive(Debug, PartialEq, Serialize)]
pub struct ConsoleElection {
    id: String,
    presentation: Option<Value>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ConsoleEvent {
    id: String,
    presentation: Option<Value>,
    is_archived: bool,
    elections: Vec<ConsoleElection>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ConsoleTenant {
    id: String,
    slug: String,
    events: Vec<ConsoleEvent>,
}

#[derive(Debug, Serialize)]
pub struct ConsoleTenantsOutput {
    tenants: Vec<ConsoleTenant>,
}

/// Nest events under their tenants and elections under their events, keeping
/// each list's order.
fn nest_tenants(
    tenants: Vec<(String, String)>,
    events: Vec<(String, String, Option<Value>, bool)>,
    elections: Vec<(String, String, Option<Value>)>,
) -> Vec<ConsoleTenant> {
    let mut elections_by_event: HashMap<String, Vec<ConsoleElection>> =
        HashMap::new();
    for (id, event_id, presentation) in elections {
        elections_by_event
            .entry(event_id)
            .or_default()
            .push(ConsoleElection { id, presentation });
    }
    let mut events_by_tenant: HashMap<String, Vec<ConsoleEvent>> =
        HashMap::new();
    for (id, tenant_id, presentation, is_archived) in events {
        let elections = elections_by_event.remove(&id).unwrap_or_default();
        events_by_tenant
            .entry(tenant_id)
            .or_default()
            .push(ConsoleEvent {
                id,
                presentation,
                is_archived,
                elections,
            });
    }
    tenants
        .into_iter()
        .map(|(id, slug)| ConsoleTenant {
            events: events_by_tenant.remove(&id).unwrap_or_default(),
            id,
            slug,
        })
        .collect()
}

/// Every tenant, with its election events and their elections, for users of the
/// super-admin tenant to choose what to browse.
#[instrument(skip(claims))]
#[post("/electoral-log-console/tenants", format = "json")]
pub async fn electoral_log_console_tenants(
    claims: JwtClaims,
) -> Result<Json<ConsoleTenantsOutput>, JsonError> {
    authorize_super_admin(
        &claims,
        vec![Permissions::ELECTORAL_LOG_CONSOLE_READ],
    )?;
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(internal_error)?;
    let transaction = client
        .build_transaction()
        .read_only(true)
        .start()
        .await
        .map_err(internal_error)?;
    let tenants: Vec<(String, String)> = transaction
        .query(
            "SELECT id::text, slug FROM sequent_backend.tenant ORDER BY slug",
            &[],
        )
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    let events: Vec<(String, String, Option<Value>, bool)> = transaction
        .query(
            "SELECT id::text, tenant_id::text, presentation, is_archived \
             FROM sequent_backend.election_event ORDER BY created_at DESC",
            &[],
        )
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
        .collect();
    let elections: Vec<(String, String, Option<Value>)> = transaction
        .query(
            "SELECT id::text, election_event_id::text, presentation \
             FROM sequent_backend.election ORDER BY created_at",
            &[],
        )
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect();
    transaction.commit().await.map_err(internal_error)?;
    Ok(Json(ConsoleTenantsOutput {
        tenants: nest_tenants(tenants, events, elections),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ConsolePageInput {
    /// The tenant of the election event; the user's own unless given.
    tenant_id: Option<String>,
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
    let input = body.into_inner();
    let tenant_id = requested_tenant(&claims, input.tenant_id.as_deref())?;
    authorize_tenant(
        &claims,
        &tenant_id,
        vec![Permissions::ELECTORAL_LOG_CONSOLE_READ],
    )?;
    let (table, order) = table_and_order(&input.table, input.order.as_deref())?;
    let filters = input.filters.unwrap_or_default();
    check_filters(table, &filters)?;
    let board = event_board(&tenant_id, &input.election_event_id).await?;
    let store = get_electoral_log_store().await.map_err(internal_error)?;
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
    /// The tenant of the election event; the user's own unless given.
    tenant_id: Option<String>,
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
    let input = body.into_inner();
    let tenant_id = requested_tenant(&claims, input.tenant_id.as_deref())?;
    authorize_tenant(
        &claims,
        &tenant_id,
        vec![Permissions::ELECTORAL_LOG_CONSOLE_READ],
    )?;
    let board = event_board(&tenant_id, &input.election_event_id).await?;
    let store = get_electoral_log_store().await.map_err(internal_error)?;
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

/// Run a read-only SQL query on the electoral-log database, as a role that can
/// only read, with a time limit. The database holds every tenant's data, so only
/// users of the super-admin tenant query it. Queries read personal data as it is
/// stored, so they need the personal-data permission too.
#[instrument(skip(claims, body))]
#[post("/electoral-log-console/query", format = "json", data = "<body>")]
pub async fn electoral_log_console_query(
    body: Json<ConsoleQueryInput>,
    claims: JwtClaims,
) -> Result<Json<ConsoleQueryOutput>, JsonError> {
    authorize_super_admin(
        &claims,
        vec![
            Permissions::ELECTORAL_LOG_CONSOLE_QUERY,
            Permissions::ELECTORAL_LOG_PERSONAL_DATA_READ,
        ],
    )?;
    let sql = body.into_inner().sql;
    check_query(&sql)?;
    tracing::info!(
        tenant_id = %claims.hasura_claims.tenant_id,
        user_id = %claims.hasura_claims.user_id,
        sql = %sql,
        "Electoral-log console query"
    );
    let connection = PostgresConnection::from_env().map_err(internal_error)?;
    let mut client = match connection.reader_client().await {
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

    const OWN_TENANT: &str = "6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10";

    fn admin(roles: &[&str]) -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "admin", "typ": "Bearer", "azp": "admin-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": true,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": OWN_TENANT,
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": roles,
            }
        }))
        .unwrap()
    }

    #[test]
    fn requests_read_the_users_tenant_unless_they_name_one() {
        let claims = admin(&[]);
        assert_eq!(requested_tenant(&claims, None).ok().unwrap(), OWN_TENANT);
        assert_eq!(
            requested_tenant(&claims, Some(" ")).ok().unwrap(),
            OWN_TENANT
        );
        let other = Uuid::new_v4().to_string();
        assert_eq!(
            requested_tenant(&claims, Some(&other)).ok().unwrap(),
            other
        );
        assert_eq!(
            requested_tenant(&claims, Some("acme")).unwrap_err().0,
            Status::BadRequest
        );
    }

    #[test]
    fn administrators_read_their_own_tenant_with_the_permission() {
        let read = Permissions::ELECTORAL_LOG_CONSOLE_READ;
        let reader = admin(&[&read.to_string()]);
        assert!(
            authorize_tenant(&reader, OWN_TENANT, vec![read.clone()]).is_ok()
        );
        assert!(authorize_tenant(&admin(&[]), OWN_TENANT, vec![read]).is_err());
    }

    #[test]
    fn tenants_list_their_events_and_elections_in_order() {
        let label = |name: &str| Some(serde_json::json!({"name": name}));
        let tenants = nest_tenants(
            vec![("t1".into(), "acme".into()), ("t2".into(), "empty".into())],
            vec![
                ("e2".into(), "t1".into(), label("Second"), false),
                ("e1".into(), "t1".into(), label("First"), true),
            ],
            vec![
                ("a".into(), "e1".into(), label("A")),
                ("b".into(), "e1".into(), label("B")),
                ("orphan".into(), "gone".into(), None),
            ],
        );
        assert_eq!(tenants.len(), 2);
        assert_eq!(tenants[0].slug, "acme");
        let ids: Vec<&str> =
            tenants[0].events.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["e2", "e1"]);
        assert!(tenants[0].events[0].elections.is_empty());
        let elections: Vec<&str> = tenants[0].events[1]
            .elections
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(elections, ["a", "b"]);
        assert!(tenants[0].events[1].is_archived);
        assert!(tenants[1].events.is_empty());
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
