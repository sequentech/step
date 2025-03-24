// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::ballot_publication::{
    get_ballot_publication, get_previous_publication, get_previous_publication_election,
    get_publication_ballot_styles, insert_ballot_publication,
    soft_delete_other_ballot_publications, soft_delete_other_ballot_publications_election,
    update_ballot_publication_d,
};
use crate::hasura::election::get_all_elections_for_event;
use crate::hasura::election_event::get_election_event_helper;
use crate::hasura::election_event::update_election_event_status;
use crate::postgres::election::update_election_status;
use crate::services::ballot_styles::ballot_publication::get_ballot_publication::GetBallotPublicationSequentBackendBallotPublication;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::*;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_client_credentials;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};

#[instrument(skip(auth_headers), err)]
async fn get_ballot_publication_by_id(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<GetBallotPublicationSequentBackendBallotPublication> {
    let ballot_publication = (&get_ballot_publication(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication_id.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find ballot publication")?
    .sequent_backend_ballot_publication)
        .get(0)
        .clone()
        .ok_or(anyhow!("Can't find ballot publication"))?
        .clone();

    Ok(ballot_publication)
}

#[instrument(skip(auth_headers), err)]
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

#[instrument(err)]
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
        election_id.clone(),
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

#[instrument(skip(auth_headers), err)]
async fn get_publication_json(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    election_id: Option<String>,
    limit: Option<usize>,
) -> Result<Value> {
    let ballot_style_strings: Vec<Option<String>> = get_publication_ballot_styles(
        auth_headers,
        tenant_id,
        election_event_id,
        ballot_publication_id,
        limit,
    )
    .await?
    .data
    .with_context(|| "can't find ballot styles")?
    .sequent_backend_ballot_style
    .into_iter()
    .filter(|ballot_style| {
        election_id
            .clone()
            .map(|id| ballot_style.election_id == id)
            .unwrap_or(true)
    })
    .map(|style| style.ballot_eml.clone())
    .collect();

    let val_arr: Vec<Value> = ballot_style_strings
        .iter()
        .map(|el| el.clone().map(|val| deserialize_str(&val).ok()).flatten())
        .filter(|el| el.is_some())
        .map(|el| el.unwrap())
        .collect();

    Ok(serde_json::Value::Array(val_arr))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationStyles {
    ballot_publication_id: String,
    ballot_styles: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationDiff {
    current: PublicationStyles,
    previous: Option<PublicationStyles>,
}

#[instrument(err)]
pub async fn get_ballot_publication_diff(
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    limit: Option<usize>,
) -> Result<PublicationDiff> {
    let auth_headers = get_client_credentials().await?;

    let ballot_publication = get_ballot_publication_by_id(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication_id.clone(),
    )
    .await?;

    let previous_publication_id_opt =
        if let Some(election_id) = ballot_publication.election_id.clone() {
            let previous_publication_data = &get_previous_publication_election(
                auth_headers.clone(),
                tenant_id.clone(),
                election_event_id.clone(),
                ballot_publication.created_at.clone(),
                election_id,
            )
            .await?
            .data
            .with_context(|| "can't find ballot publication")?
            .sequent_backend_ballot_publication;
            previous_publication_data.get(0).map(|val| val.id.clone())
        } else {
            let previous_publication_data = &get_previous_publication(
                auth_headers.clone(),
                tenant_id.clone(),
                election_event_id.clone(),
                ballot_publication.created_at.clone(),
            )
            .await?
            .data
            .with_context(|| "can't find ballot publication")?
            .sequent_backend_ballot_publication;
            previous_publication_data.get(0).map(|val| val.id.clone())
        };

    let current_json = get_publication_json(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication.id.clone(),
        ballot_publication.election_id.clone(),
        limit,
    )
    .await?;

    let current = PublicationStyles {
        ballot_publication_id: ballot_publication_id.clone(),
        ballot_styles: current_json,
    };

    let previous = if let Some(previous_publication_id) = previous_publication_id_opt {
        let previous_json = get_publication_json(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            previous_publication_id.clone(),
            ballot_publication.election_id.clone(),
            limit,
        )
        .await?;

        Some(PublicationStyles {
            ballot_publication_id: previous_publication_id.clone(),
            ballot_styles: previous_json,
        })
    } else {
        None
    };

    Ok(PublicationDiff { current, previous })
}
