// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::{
    get_ballot_publication_by_id, get_previous_publication, get_previous_publication_election,
    insert_ballot_publication, soft_delete_other_ballot_publications, update_ballot_publication,
};
use crate::postgres::ballot_style::get_publication_ballot_styles;
use crate::postgres::election::{get_elections_ids, update_election_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::*;
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::connection;
use sequent_core::services::date::ISO8601;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};

use super::ballot_style;

#[instrument(skip(hasura_transaction), err)]
async fn get_election_ids_for_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id_opt: Option<String>,
) -> Result<Vec<String>> {
    if let Some(election_id) = election_id_opt {
        return Ok(vec![election_id]);
    }
    let elections_ids =
        get_elections_ids(hasura_transaction, &tenant_id, &election_event_id).await?;

    Ok(elections_ids)
}

#[instrument(err)]
pub async fn add_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    user_id: String,
) -> Result<String> {
    let celery_app = get_celery_app().await;

    let election_ids = get_election_ids_for_publication(
        hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        election_id.clone(),
    )
    .await?;

    let ballot_publication = insert_ballot_publication(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
        election_ids.clone(),
        user_id.clone(),
        election_id.clone(),
    )
    .await?
    .with_context(|| "can't find inserted ballot publication")?;

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

#[instrument(err)]
pub async fn update_publish_ballot(
    hasura_transaction: &Transaction<'_>,
    user_id: String,
    username: String,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<()> {
    let ballot_publication = get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .with_context(|| "Can't find ballot publication")?;

    if ballot_publication.is_generated.unwrap_or(false) == false {
        return Err(anyhow!(
            "Ballot publication not generated yet, can't publish."
        ));
    }

    if ballot_publication.published_at.is_some() {
        return Ok(());
    }

    let _result = soft_delete_other_ballot_publications(
        &hasura_transaction,
        &ballot_publication_id,
        &election_event_id,
        &tenant_id,
        ballot_publication.election_id.clone(),
    )
    .await?;

    update_ballot_publication(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
        true,
        Some(ISO8601::now()),
    )
    .await?;

    let election_event = get_election_event_by_id(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
    )
    .await?;

    let mut new_status: ElectionEventStatus =
        get_election_event_status(election_event.status).unwrap_or(Default::default());
    new_status.is_published = Some(true);
    let new_status_js = serde_json::to_value(new_status)?;

    update_election_event_status(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        new_status_js,
    )
    .await?;

    // Update elections status
    let election_ids = ballot_publication.election_ids.clone().unwrap_or(vec![]);
    for election_id in election_ids.clone() {
        update_election_status(
            &hasura_transaction,
            &election_id,
            &tenant_id.clone(),
            &election_event_id.clone(),
            true,
        )
        .await
        .with_context(|| "error updating election status")?;
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    // let electoral_log = ElectoralLog::new(board_name.as_str()).await?;
    let electoral_log = ElectoralLog::for_admin_user(
        hasura_transaction,
        &board_name,
        &tenant_id,
        &election_event.id,
        &user_id,
        Some(username.clone()),
        Some(election_ids.clone()),
        None,
    )
    .await?;
    electoral_log
        .post_election_published(
            election_event_id.clone(),
            Some(election_ids.clone()),
            ballot_publication_id.clone(),
            Some(user_id),
            Some(username),
        )
        .await
        .map_err(|e| anyhow!("error posting to the electoral log: {e}"))?;
    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_publication_json(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    election_id: Option<String>,
    limit: Option<usize>,
) -> Result<Value> {
    let ballot_style = get_publication_ballot_styles(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
        limit,
    )
    .await?;

    let ballot_style_strings: Vec<Option<String>> = ballot_style
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
        .map(|el| el.ok_or(anyhow!("Empty ballot style!")))
        .collect::<Result<Vec<_>>>()?;

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
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    limit: Option<usize>,
) -> Result<PublicationDiff> {
    let ballot_publication = get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .with_context(|| "Can't find ballot publication")?;

    let previous_publication_id = if let Some(election_id) = ballot_publication.election_id.clone()
    {
        get_previous_publication_election(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            ballot_publication.created_at.clone(),
            &election_id,
        )
        .await?
        .map(|pub_data| pub_data.id)
        .ok_or_else(|| {
            anyhow!(
                "Can't find ballot publication for election id {}",
                election_id
            )
        })
        .with_context(|| "Error retrieving previous ballot publication for election")
        .ok()
    } else {
        get_previous_publication(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            ballot_publication.created_at.clone(),
        )
        .await?
        .map(|pub_data| pub_data.id)
        .ok_or_else(|| anyhow!("Can't find ballot publication"))
        .with_context(|| "Error retrieving previous ballot publication")
        .ok()
    };

    let current_json = get_publication_json(
        &hasura_transaction,
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

    let previous = if let Some(previous_publication_id) = previous_publication_id {
        let previous_json = get_publication_json(
            &hasura_transaction,
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
