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

/// The usernames of a realm's people (service accounts excluded), or `None`
/// when no realm has that name.
#[instrument(err, skip(keycloak_transaction))]
pub async fn realm_usernames(
    keycloak_transaction: &Transaction<'_>,
    realm_name: &str,
) -> Result<Option<std::collections::BTreeSet<String>>> {
    let realm = keycloak_transaction
        .query_opt("SELECT id FROM realm WHERE name = $1", &[&realm_name])
        .await
        .map_err(|err| anyhow!("Error reading the realm: {err}"))?;
    let Some(realm) = realm else {
        return Ok(None);
    };
    let realm_id: String = realm.get("id");
    let rows = keycloak_transaction
        .query(
            "SELECT username FROM user_entity
             WHERE realm_id = $1 AND service_account_client_link IS NULL",
            &[&realm_id],
        )
        .await
        .map_err(|err| anyhow!("Error reading the realm's users: {err}"))?;
    Ok(Some(
        rows.into_iter().map(|row| row.get("username")).collect(),
    ))
}

/// The groups of each of `usernames` that exists in the realm, by lower-cased
/// username; a username with no account is absent from the map.
#[instrument(err, skip(keycloak_transaction, usernames))]
pub async fn user_groups(
    keycloak_transaction: &Transaction<'_>,
    realm_name: &str,
    usernames: &[String],
) -> Result<std::collections::BTreeMap<String, std::collections::BTreeSet<String>>> {
    let lowered: Vec<String> = usernames.iter().map(|u| u.to_lowercase()).collect();
    let rows = keycloak_transaction
        .query(
            r#"
            SELECT user_entity.username, keycloak_group.name AS group_name
            FROM user_entity
            JOIN realm ON realm.id = user_entity.realm_id
            LEFT JOIN user_group_membership ON user_group_membership.user_id = user_entity.id
            LEFT JOIN keycloak_group ON keycloak_group.id = user_group_membership.group_id
            WHERE realm.name = $1 AND user_entity.username = ANY($2);
            "#,
            &[&realm_name, &lowered],
        )
        .await
        .map_err(|err| anyhow!("Error reading the accounts' groups: {err}"))?;
    let mut groups: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        Default::default();
    for row in rows {
        let entry = groups.entry(row.get::<_, String>("username")).or_default();
        if let Some(group) = row.get::<_, Option<String>>("group_name") {
            entry.insert(group);
        }
    }
    Ok(groups)
}

/// The Scanovate `liveness-secret` of each authenticator configuration of a
/// realm used only by Scanovate executions, read from Keycloak's storage
/// (its Admin API masks it), each with its configuration's id and
/// `liveness-url` from the same statement, so the export can tell whether the
/// configuration changed since the realm export read it. No other setting,
/// configuration or realm is read.
#[instrument(skip(keycloak_transaction), err)]
pub async fn get_scanovate_liveness_secrets(
    keycloak_transaction: &Transaction<'_>,
    realm_name: &str,
) -> Result<Vec<sequent_core::types::integration_secrets::StoredIntegrationSecret>> {
    use sequent_core::types::integration_secrets::{
        IntegrationSecretValue, StoredIntegrationSecret, SCANOVATE_AUTHENTICATOR_PROVIDER,
        SCANOVATE_LIVENESS_SECRET_KEY, SCANOVATE_LIVENESS_URL_KEY,
    };
    let rows = keycloak_transaction
        .query(
            r#"
            SELECT
                config.id::VARCHAR AS config_id,
                config.alias AS config_alias,
                execution.authenticator AS provider_id,
                secret.value AS secret_value,
                endpoint.value AS endpoint
            FROM realm
            JOIN authentication_execution AS execution
                ON execution.realm_id = realm.id
            JOIN authenticator_config AS config
                ON config.id = execution.auth_config
                AND config.realm_id = realm.id
            JOIN authenticator_config_entry AS secret
                ON secret.authenticator_id = config.id
                AND secret.name = $2
            LEFT JOIN authenticator_config_entry AS endpoint
                ON endpoint.authenticator_id = config.id
                AND endpoint.name = $3
            WHERE realm.name = $1
                AND execution.authenticator = $4
                AND NOT EXISTS (
                    SELECT 1 FROM authentication_execution AS other
                    WHERE other.auth_config = config.id
                        AND other.authenticator IS DISTINCT FROM $4
                )
            "#,
            &[
                &realm_name,
                &SCANOVATE_LIVENESS_SECRET_KEY,
                &SCANOVATE_LIVENESS_URL_KEY,
                &SCANOVATE_AUTHENTICATOR_PROVIDER,
            ],
        )
        .await
        // The statement's error never carries row values.
        .map_err(|err| anyhow!("Error reading the realm's Scanovate configurations: {err}"))?;
    rows.into_iter()
        .map(|row| -> Result<StoredIntegrationSecret> {
            Ok(StoredIntegrationSecret {
                config_id: row.try_get("config_id")?,
                config_alias: row.try_get("config_alias")?,
                provider_id: row.try_get("provider_id")?,
                secret_key: SCANOVATE_LIVENESS_SECRET_KEY.to_string(),
                value: IntegrationSecretValue::new(
                    row.try_get::<_, Option<String>>("secret_value")?
                        .unwrap_or_default(),
                ),
                endpoint: row.try_get("endpoint")?,
            })
        })
        .collect::<Result<Vec<_>>>()
        .context("Error reading a Scanovate configuration row")
}
