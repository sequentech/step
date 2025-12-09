// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde::{Deserialize, Serialize};
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::instrument;

pub const LOGIN_EVENT_TYPE: &str = "LOGIN";
pub const LOGIN_ERR_EVENT_TYPE: &str = "LOGIN_ERROR";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Event {
    pub id: String,
    pub event_time: i64,
    pub event_type: String,
    pub user_id: Option<String>,
    pub realm_id: Option<String>,
    pub client_id: Option<String>,
    pub details_json: Option<String>,
    pub details_json_long_value: Option<String>,
    pub error: Option<String>,
    pub ip_address: Option<String>,
    pub session_id: Option<String>,
}

impl TryFrom<Row> for Event {
    type Error = anyhow::Error;
    fn try_from(row: Row) -> Result<Self> {
        Ok(Event {
            id: row.try_get("id")?,
            event_time: row.try_get("event_time")?,
            event_type: row.try_get("type")?,
            user_id: row.try_get("user_id")?,
            realm_id: row.try_get("realm_id")?,
            client_id: row.try_get("client_id")?,
            details_json: row.try_get("details_json")?,
            details_json_long_value: row.try_get("details_json_long_value")?,
            error: row.try_get("error")?,
            session_id: row.try_get("session_id")?,
            ip_address: row.try_get("ip_address")?,
        })
    }
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn list_keycloak_events_by_type(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    events_type: &str,
    event_action: Option<&str>,
) -> Result<Vec<Event>> {
    let event_action_clause = match event_action {
        Some(_) => "AND (e.details_json_long_value::json ->> 'action' IS NOT NULL AND e.details_json_long_value::json ->> 'action' = $3)".to_string(),
        None => "".to_string(),
    };
    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
        SELECT *
        FROM
            EVENT_ENTITY as e
        INNER JOIN
            realm AS ra ON ra.id = e.realm_id
        WHERE
        ra.name = $1
        AND e.type = $2
        {event_action_clause}
    "#
            )
            .as_str(),
        )
        .await
        .map_err(|err| {
            anyhow!("Error prepare list_keycloak_events_by_type query statement: {err}")
        })?;

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &events_type];

    if event_action.is_some() {
        params.push(&event_action);
    }

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("Error running list_keycloak_events_by_type query: {err}"))?;

    let events = rows
        .into_iter()
        .map(|row| -> Result<Event> { row.try_into() })
        .collect::<Result<Vec<Event>>>()
        .map_err(|err| {
            anyhow!("Error convert rows to data at list_keycloak_events_by_type: {err}")
        })?;

    Ok(events)
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_events_by_type(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    events_type: &str,
    event_error: Option<&str>,
    no_duplicate_user: bool,
    area_id: Option<&str>,
) -> Result<i64> {
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &events_type];
    let mut param_count = 2;
    let error_clause = match event_error {
        Some(_) => {
            param_count += 1;
            params.push(&event_error);
            format!("AND e.error = ${param_count}")
        }
        None => "".to_string(),
    };

    let select_str = match no_duplicate_user {
        true => "COUNT(DISTINCT e.user_id)",
        false => "COUNT(*)",
    };

    let (ua_join_clause, area_id_clause) = match area_id {
        Some(_) => {
            let next_param_number = param_count + 1;
            param_count += 2;
            params.push(&AREA_ID_ATTR_NAME);
            params.push(&area_id);
            (
                format!(
                    r#"
                INNER JOIN
                    user_attribute AS us ON us.user_id = e.user_id"#
                ),
                format!(
                    r#"
                AND us.name = ${next_param_number}
                AND us.value = ${param_count}"#
                ),
            )
        }
        None => ("".to_string(), "".to_string()),
    };

    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
                SELECT 
                {select_str}
                FROM
                    EVENT_ENTITY as e
                INNER JOIN
                    realm AS ra ON ra.id = e.realm_id
                {ua_join_clause}
                WHERE
                    ra.name = $1
                    AND e.type = $2
                    {error_clause}
                    {area_id_clause}
                "#
            )
            .as_str(),
        )
        .await
        .map_err(|err| {
            anyhow!("Error prepare list_keycloak_events_by_type query statement: {err}")
        })?;

    let row: Row = keycloak_transaction
        .query_one(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("Error running count_keycloak_events_by_type query: {err}"))?;

    let count: i64 = row.get(0);

    Ok(count)
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_password_reset_event_by_area(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
) -> Result<i64> {
    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
             SELECT COUNT(*)
            FROM (
                SELECT *
                FROM EVENT_ENTITY as e
                WHERE e.type = 'SEND_RESET_PASSWORD'
            ) AS filtered_e
            INNER JOIN
                realm AS ra ON ra.id = filtered_e.realm_id
            INNER JOIN
                user_attribute AS us ON us.user_id = filtered_e.user_id
            WHERE
                ra.name = $1
                AND us.name = $2
                AND us.value = $3
                "#
            )
            .as_str(),
        )
        .await
        .map_err(|err| {
            anyhow!(
                "Error prepare count_keycloak_password_reset_event_by_area query statement: {err}"
            )
        })?;

    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &AREA_ID_ATTR_NAME, &area_id];

    let row: Row = keycloak_transaction
        .query_one(&statement, &params.as_slice())
        .await
        .map_err(|err| {
            anyhow!("Error running count_keycloak_password_reset_event_by_area query: {err}")
        })?;

    let count: i64 = row.get(0);

    Ok(count)
}
