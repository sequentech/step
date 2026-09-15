// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::SUPPORT_MATERIALS_ACKNOWLEDGED_ATTR_NAME;
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::collections::HashMap;
use std::time::Duration;
use tracing::instrument;

/// Records that a voter has read and acknowledged the Election Event's
/// Support Materials, as the list of Support Material document ids they
/// acknowledged. Mirrors how `VOTED_CHANNEL` is written in
/// `windmill::tasks::process_cast_vote::mark_voted_via_internet`: a plain
/// Keycloak user attribute, scoped per voter per Election Event since each
/// Election Event has its own realm.
#[instrument(skip(document_ids), err)]
pub async fn acknowledge_support_materials(
    realm: &str,
    voter_id: &str,
    document_ids: Vec<String>,
) -> Result<()> {
    let mut attributes = HashMap::new();
    attributes.insert(
        SUPPORT_MATERIALS_ACKNOWLEDGED_ATTR_NAME.to_string(),
        document_ids,
    );

    retry_with_exponential_backoff(
        || async {
            let client = KeycloakAdminClient::new()
                .await
                .map_err(|err| format!("Error obtaining Keycloak client: {err:?}"))?;
            client
                .edit_user(
                    realm,
                    voter_id,
                    None,
                    Some(attributes.clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|err| {
                    format!("Error editing voter support materials acknowledgment: {err:?}")
                })
        },
        3,
        Duration::from_millis(500),
    )
    .await
    .map(|_| ())
    .map_err(|err| {
        anyhow!("Error editing voter support materials acknowledgment after retries: {err}")
    })
}

/// Returns the Support Material document ids the voter has acknowledged for
/// this Election Event (empty if none).
#[instrument(err)]
pub async fn get_support_materials_acknowledgment(
    realm: &str,
    voter_id: &str,
) -> Result<Vec<String>> {
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error obtaining Keycloak client: {err:?}"))?;
    let user = client.get_user(realm, voter_id).await?;
    Ok(user
        .attributes
        .and_then(|attributes| {
            attributes
                .get(SUPPORT_MATERIALS_ACKNOWLEDGED_ATTR_NAME)
                .cloned()
        })
        .unwrap_or_default())
}
