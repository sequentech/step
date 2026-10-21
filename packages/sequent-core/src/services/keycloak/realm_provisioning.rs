// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use keycloak::types::RealmRepresentation;
use keycloak::{KeycloakAdmin, KeycloakError, KeycloakTokenSupplier};

pub(super) const API_KEY_CLIENT_ID: &str = "api-key-client";

#[derive(Clone, Copy)]
pub(super) enum RealmProvisioning {
    Create,
    Update,
}

/// Imported enablement is not operator opt-in. Existing integrations are managed
/// through the client administration API, never by a generic realm import.
pub(super) async fn prepare_api_key_client<T: KeycloakTokenSupplier>(
    admin: &KeycloakAdmin<T>,
    realm_name: &str,
    realm: &mut RealmRepresentation,
    provisioning: RealmProvisioning,
) -> Result<(), KeycloakError> {
    let Some(clients) = realm.clients.as_mut() else {
        return Ok(());
    };
    if !clients
        .iter()
        .any(|client| client.client_id.as_deref() == Some(API_KEY_CLIENT_ID))
    {
        return Ok(());
    }

    let existing = match provisioning {
        RealmProvisioning::Create => false,
        RealmProvisioning::Update => admin
            .realm_clients_get(
                realm_name,
                Some(API_KEY_CLIENT_ID.to_string()),
                None,
                None,
                None,
                Some(false),
                None,
            )
            .await?
            .iter()
            .any(|client| {
                client.client_id.as_deref() == Some(API_KEY_CLIENT_ID)
            }),
    };
    if existing {
        // Do not send any imported client configuration or credential to the
        // update endpoint. Fetching a masked secret and writing it back would
        // itself risk resetting the integration's credential.
        clients.retain(|client| {
            client.client_id.as_deref() != Some(API_KEY_CLIENT_ID)
        });
        if let Some(users) = realm.users.as_mut() {
            users.retain(|user| {
                user.service_account_client_id.as_deref()
                    != Some(API_KEY_CLIENT_ID)
            });
        }
    } else {
        for client in clients {
            if client.client_id.as_deref() == Some(API_KEY_CLIENT_ID) {
                client.enabled = Some(false);
            }
        }
    }
    Ok(())
}

/// Record administrative actions in new realms without storing request bodies.
/// An update omits both fields so the operator's existing audit policy survives.
pub(super) fn prepare_admin_event_defaults(
    realm: &mut RealmRepresentation,
    provisioning: RealmProvisioning,
) {
    match provisioning {
        RealmProvisioning::Create => {
            realm.admin_events_enabled = Some(true);
            realm.admin_events_details_enabled = Some(false);
        }
        RealmProvisioning::Update => {
            realm.admin_events_enabled = None;
            realm.admin_events_details_enabled = None;
        }
    }
}

#[cfg(test)]
#[path = "realm_provisioning_tests.rs"]
mod tests;
