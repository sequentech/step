// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::area::get_areas_by_ids;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::connection;
use sequent_core::types::keycloak::*;
use anyhow::{anyhow, Result, Context};
use keycloak::types::{UserRepresentation, CredentialRepresentation};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::From;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(auth_headers, admin))]
pub async fn list_users(
    auth_headers: connection::AuthHeaders,
    admin: &KeycloakAdminClient,
    tenant_id: String,
    election_event_id: Option<String>,
    realm: &str,
    search: Option<String>,
    email: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<(Vec<User>, i32)> {
    let user_representations: Vec<UserRepresentation> = admin
        .client
        .realm_users_get(
            realm.clone(),
            Some(false),
            email.clone(),
            None,
            None,
            None,
            offset.clone(),
            None,
            None,
            None,
            None,
            limit.clone(),
            None,
            search.clone(),
            None,
        )
        .await
        .map_err(|err| anyhow!("{:?}", err))?;
    let count: i32 = admin
        .client
        .realm_users_count_get(
            realm, email, None, None, None, None, search, None,
        )
        .await
        .map_err(|err| anyhow!("{:?}", err))?;
    let users: Vec<User> = user_representations
        .clone()
        .into_iter()
        .map(|user| user.into())
        .collect();
    if let Some(ref some_election_event_id) = election_event_id {
        let area_ids: Vec<Option<String>> = user_representations
            .iter()
            .filter(|user| (
                user.attributes.as_ref().is_some_and(|attributes| {
                    attributes.contains_key("area-id")
                })
            ))
            .map(|user| user.id.clone())
            .collect();
        let areas_by_ids = 
            get_areas_by_ids(
                auth_headers.clone(),
                tenant_id,
                some_election_event_id.clone(),
                vec![],
            ).await
            .map_err(|err| anyhow!("{:?}", err))?
            .data
            .with_context(|| "can't find areas by ids")?
            .sequent_backend_area;
        let get_area = |user: &User| {
            let area_id = user.attributes.as_ref()?.get("area-id")?.as_str()?;
            return areas_by_ids
                .iter()
                .find_map(|area| {
                    if (area.id == area_id) {
                        Some(UserArea {
                            id: Some(area.id.clone()),
                            name: area.name.clone(),
                        })
                    } else {
                        None
                    }
                });
        };
        let users_with_area = users
            .into_iter()
            .map(|user| {
                User {
                    area: get_area(&user),
                    ..user.clone()
                }
            })
            .collect();
        Ok((users_with_area, count))
    } else {
        Ok((users, count))
    }
}
