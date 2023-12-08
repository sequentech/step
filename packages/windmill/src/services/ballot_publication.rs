// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::ballot_publication::insert_ballot_publication;
use crate::services::celery_app::get_celery_app;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use anyhow::{Context, Result};
use sequent_core::services::keycloak::get_client_credentials;
use tracing::{event, instrument, Level};

#[instrument]
pub async fn add_ballot_publication(
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
    user_id: String,
) -> Result<String> {
    let auth_headers = get_client_credentials().await?;
    let celery_app = get_celery_app().await;
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

    let task = celery_app
        .send_task(update_election_event_ballot_styles::new(
            tenant_id.clone(),
            election_event_id.clone(),
            ballot_publication.id.clone(),
        ))
        .await?;
    event!(
        Level::INFO,
        "Sent CREATE_ELECTION_EVENT_BALLOT_STYLES task {}",
        task.task_id
    );

    Ok(ballot_publication.id.clone())
}
