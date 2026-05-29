// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::PgConfig;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::*;
use std::convert::From;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[instrument(skip(keycloak_transaction), err)]
pub async fn get_realm_id(
    keycloak_transaction: &Transaction<'_>,
    realm_name: String,
) -> Result<String> {
    let get_realm_statement = keycloak_transaction
        .prepare(
            r#"
            SELECT
                id::VARCHAR AS id
            FROM
                realm
            WHERE
                realm.name = $1;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing get_realm_id query: {}", err))?;
    let rows: Vec<Row> = keycloak_transaction
        .query(&get_realm_statement, &[&realm_name])
        .await
        .map_err(|err| anyhow!("Error running get_realm_id query: {}", err))?;
    let realm_ids: Vec<String> = rows
        .into_iter()
        .map(|row| -> Result<String> {
            Ok(row
                .try_get::<&str, String>("id")
                .map_err(|err| anyhow!("Error getting the realm id from a row: {}", err))?)
        })
        .collect::<Result<Vec<String>>>()
        .map_err(|err| anyhow!("Error getting the realm ids: {}", err))?;
    if realm_ids.len() > 1 {
        return Err(anyhow!(
            "found too many realms with same name: {}",
            realm_ids.len()
        ));
    }
    Ok(realm_ids
        .first()
        .ok_or(anyhow!("realm not found: {realm_name}"))?
        .clone())
}
