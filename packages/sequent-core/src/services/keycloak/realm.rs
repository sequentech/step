// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::{
    keycloak::KeycloakAdminClient, replace_uuids::replace_uuids,
};
use crate::types::keycloak::TENANT_ID_ATTR_NAME;
use anyhow::Result;
use keycloak::types::RealmRepresentation;
use keycloak::KeycloakError;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::instrument;

pub fn get_event_realm(tenant_id: &str, election_event_id: &str) -> String {
    format!("tenant-{}-event-{}", tenant_id, election_event_id)
}

pub fn get_tenant_realm(tenant_id: &str) -> String {
    format!("tenant-{}", tenant_id)
}

impl KeycloakAdminClient {
    #[instrument(skip(self, json_realm_config), err)]
    pub async fn upsert_realm(
        self,
        board_name: &str,
        json_realm_config: &str,
        tenant_id: &str,
        replace_ids: bool,
    ) -> Result<(), KeycloakError> {
        let real_get_result = self.client.realm_get(board_name).await;
        let replaced_ids_config = if replace_ids {
            replace_uuids(json_realm_config, vec![])
        } else {
            json_realm_config.to_string()
        };
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
                    attributes.insert(
                        TENANT_ID_ATTR_NAME.into(),
                        tenant_attribute_js,
                    );
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
