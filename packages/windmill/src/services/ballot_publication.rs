// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::ballot_publication::insert_ballot_publication;
use crate::hasura::election::get_all_elections_for_event;
use crate::services::celery_app::get_celery_app;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use anyhow::{Context, Result};
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_client_credentials;
use tracing::{event, instrument, Level};

#[instrument]
async fn get_election_ids_for_publication(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_id_opt: Option<String>,
) -> Result<Vec<String>> {
    if election_id_opt.is_some() {
        return Ok(vec![election_id_opt.unwrap()]);
    }
    let elections =
        get_all_elections_for_event(auth_headers, tenant_id.clone(), election_event_id.clone())
            .await?
            .data
            .expect("expected data")
            .sequent_backend_election;

    Ok(elections
        .into_iter()
        .map(|election| election.id.clone())
        .collect())
}

#[instrument]
pub async fn add_ballot_publication(
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    user_id: String,
) -> Result<String> {
    let auth_headers = get_client_credentials().await?;
    let celery_app = get_celery_app().await;

    let election_ids = get_election_ids_for_publication(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_id.clone(),
    )
    .await?;

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
