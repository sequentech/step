// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::ballot_publication::insert_ballot_publication;
use anyhow::{Context, Result};
use sequent_core::services::keycloak::get_client_credentials;

pub async fn add_ballot_publication(
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
    user_id: String
) -> Result<String> {
    let auth_headers = get_client_credentials().await?;
    let ballot_publication = &insert_ballot_publication(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_ids.clone(),
        user_id.clone(),
    )
    .await?
    .data
    .expect("expected data".into())
    .insert_sequent_backend_ballot_publication
    .with_context(|| "can't find inserted ballot publication")?
    .returning[0];

    Ok(ballot_publication.id.clone())
}