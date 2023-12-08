// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use celery::error::TaskError;
use sequent_core::services::keycloak;
use tracing::{event, instrument, Level};

use crate::hasura::area::get_election_event_areas;
use crate::hasura::ballot_publication::*;
use crate::services::ballot_style::create_ballot_style;
use crate::services::celery_app::get_celery_app;
use crate::types::error::Result;

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn update_election_event_ballot_styles(
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;

    let ballot_publication = &get_ballot_publication(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication_id.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find election event areas")?
    .sequent_backend_ballot_publication[0];

    let areas = get_election_event_areas(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        vec![],
    )
    .await?
    .data
    .with_context(|| "can't find election event areas")?;
    let celery_app = get_celery_app().await;

    for area in areas.sequent_backend_area.iter() {
        create_ballot_style(
            auth_headers.clone(),
            area.id.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            ballot_publication.election_ids.clone().unwrap_or(vec![]),
            ballot_publication.id.clone(),
        )
        .await?;
    }
    update_ballot_publication_d(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication_id.clone(),
        true,
    )
    .await?;

    Ok(())
}
