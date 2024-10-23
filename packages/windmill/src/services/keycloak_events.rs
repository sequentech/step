// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Event {
    id: String,
    event_time: i64,
    event_type: String,
    user_id: Option<String>,
    realm_id: Option<String>,
    client_id: Option<String>,
    details_json: Option<String>,
    details_json_long_value: Option<String>,
    error: Option<String>,
    ip_address: Option<String>,
    session_id: Option<String>,
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
