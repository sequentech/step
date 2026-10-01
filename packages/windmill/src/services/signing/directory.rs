// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The names signing shows for people: first and last name from the tenant
//! realm's Keycloak users, or the username without them.

use crate::services::database::get_keycloak_pool;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Pool, Transaction};
use sequent_core::services::keycloak::get_tenant_realm;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Looks up people's display names; tests use fakes.
#[async_trait]
pub trait UserDirectory: Send + Sync {
    /// The display names of `user_ids` in the tenant, by user id. Unknown
    /// ids are left out.
    async fn display_names(
        &self,
        tenant_id: Uuid,
        user_ids: &[String],
    ) -> Result<HashMap<String, String>>;
}

/// "First Last", either alone, or the username when both are empty.
pub fn display_name(first_name: Option<&str>, last_name: Option<&str>, username: &str) -> String {
    let name = [first_name, last_name]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        username.to_owned()
    } else {
        name
    }
}

/// A user of the tenant realm, as signing names them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub username: String,
    pub display_name: String,
}

/// The users `user_ids` of `realm`, read in the Keycloak database. Unknown
/// ids are left out.
pub async fn people_in_realm(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    user_ids: &[String],
) -> Result<HashMap<String, Person>> {
    Ok(keycloak_transaction
        .query(
            "SELECT u.id, u.first_name, u.last_name, u.username
             FROM user_entity AS u JOIN realm AS r ON r.id = u.realm_id
             WHERE r.name = $1 AND u.id = ANY($2)",
            &[&realm, &user_ids],
        )
        .await
        .context("Error reading the users' names")?
        .iter()
        .map(|row| {
            let id: String = row.get(0);
            let username = row
                .get::<_, Option<String>>(3)
                .unwrap_or_else(|| id.clone());
            let display_name = display_name(
                row.get::<_, Option<String>>(1).as_deref(),
                row.get::<_, Option<String>>(2).as_deref(),
                &username,
            );
            (
                id,
                Person {
                    username,
                    display_name,
                },
            )
        })
        .collect())
}

/// The display names of `user_ids` among the users of `realm`.
pub async fn display_names_in_realm(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    user_ids: &[String],
) -> Result<HashMap<String, String>> {
    Ok(people_in_realm(keycloak_transaction, realm, user_ids)
        .await?
        .into_iter()
        .map(|(id, person)| (id, person.display_name))
        .collect())
}

/// The tenant realm's users in the Keycloak database: on the given pool,
/// or on the worker's own ([`get_keycloak_pool`]) by default.
#[derive(Clone, Default)]
pub struct KeycloakUserDirectory {
    pool: Option<Arc<Pool>>,
}

impl KeycloakUserDirectory {
    pub fn on(pool: Arc<Pool>) -> Self {
        KeycloakUserDirectory { pool: Some(pool) }
    }

    /// The users `user_ids` of the tenant's realm.
    pub async fn people(
        &self,
        tenant_id: Uuid,
        user_ids: &[String],
    ) -> Result<HashMap<String, Person>> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let pool = match &self.pool {
            Some(pool) => pool.clone(),
            None => get_keycloak_pool().await,
        };
        let mut client = pool
            .get()
            .await
            .context("Error getting a Keycloak database client")?;
        let transaction = client.transaction().await?;
        people_in_realm(
            &transaction,
            &get_tenant_realm(&tenant_id.to_string()),
            user_ids,
        )
        .await
    }
}

#[async_trait]
impl UserDirectory for KeycloakUserDirectory {
    async fn display_names(
        &self,
        tenant_id: Uuid,
        user_ids: &[String],
    ) -> Result<HashMap<String, String>> {
        Ok(self
            .people(tenant_id, user_ids)
            .await?
            .into_iter()
            .map(|(id, person)| (id, person.display_name))
            .collect())
    }
}
