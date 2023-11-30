// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use anyhow::Result;
use keycloak::types::RealmRepresentation;
use keycloak::KeycloakError;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::instrument;
use uuid::Uuid;

pub fn get_event_realm(tenant_id: &str, election_event_id: &str) -> String {
    format!("tenant-{}-event-{}", tenant_id, election_event_id)
}

pub fn get_tenant_realm(tenant_id: &str) -> String {
    format!("tenant-{}", tenant_id)
}

fn replace_uuids(input: &str) -> String {
    let uuid_regex =
        Regex::new(r"\b[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}\b")
            .unwrap();

    let mut seen_uuids = HashMap::new();

    uuid_regex
        .replace_all(input, |caps: &regex::Captures| {
            let old_uuid = caps.get(0).unwrap().as_str();
            seen_uuids
                .entry(old_uuid.to_owned())
                .or_insert_with(|| Uuid::new_v4().to_string())
                .clone()
        })
        .into_owned()
}

impl KeycloakAdminClient {
    #[instrument(skip(self))]
    pub async fn upsert_realm(
        self,
        board_name: &str,
        json_realm_config: &str,
        tenant_id: &str,
    ) -> Result<(), KeycloakError> {
        let real_get_result = self.client.realm_get(board_name).await;
        let replaced_ids_config = replace_uuids(json_realm_config);
        let mut realm: RealmRepresentation =
            serde_json::from_str(&replaced_ids_config).unwrap();
        
        // set realm name
        realm.realm = Some(board_name.into());

        // set tenant id attribute on all users
        realm.users = Some(
            realm
                .users
                .unwrap_or(vec![])
                .into_iter()
                .map(|user| {
                    let mut mod_user = user.clone();
                    let mut attributes =
                        mod_user.attributes.clone().unwrap_or(HashMap::new());
                    let tenant_attribute_js: Value =
                        json!(vec![tenant_id.to_string()]);
                    attributes.insert("tenant-id".into(), tenant_attribute_js);
                    mod_user.attributes = Some(attributes);
                    mod_user
                })
                .collect(),
        );

        match real_get_result {
            Err(_) => self.client.post(realm).await,
            Ok(_) => Ok(()),
        }
    }
}
