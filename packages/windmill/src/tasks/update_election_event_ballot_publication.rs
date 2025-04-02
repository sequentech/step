// SPDX-FileCopyrightText: 2023-2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::ballot_publication::{
    get_ballot_publication, get_previous_publication, get_previous_publication_election,
    get_publication_ballot_styles, insert_ballot_publication,
    soft_delete_other_ballot_publications, soft_delete_other_ballot_publications_election,
    update_ballot_publication_d,
};
use crate::hasura::election_event::get_election_event_helper;
use crate::hasura::election_event::update_election_event_status;
use crate::postgres::ballot_publication::get_ballot_publication_by_id;
use crate::postgres::election::update_election_status;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_styles::ballot_style;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::ElectoralLog;
use crate::services::pg_lock::PgLock;
use crate::types::error::Result;

use anyhow::anyhow;
use celery::error::TaskError;
use chrono::{Duration, Local};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_client_credentials;

use tracing::instrument;
use uuid::Uuid;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn update_election_event_ballot_publication(
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    user_id: String,
    username: String,
) -> Result<()> {
    let lock = PgLock::acquire(
        format!(
            "update_ballot_publication-{tenant_id}-{election_event_id}-{ballot_publication_id}"
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(60),
    )
    .await?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| format!("Error getting hasura db pool: {e:?}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| format!("Error starting hasura transaction: {e:?}"))?;

    let (is_generated, ballot_publication) = match get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await
    {
        Ok(Some(bp)) => (bp.is_generated.clone().unwrap_or(false), bp),
        _ => return Err(format!("Can't find ballot publication").into()),
    };

    if !is_generated {
        return Err(format!("Ballot publication not generated yet, can't publish.").into());
    }

    if ballot_publication.published_at.is_some() {
        return Ok(());
    }

    let auth_headers = get_client_credentials().await?;
    if let Some(election_id) = ballot_publication.election_id.clone() {
        soft_delete_other_ballot_publications_election(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            ballot_publication_id.clone(),
            election_id,
        )
        .await?;
    } else {
        soft_delete_other_ballot_publications(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            ballot_publication_id.clone(),
        )
        .await?;
    }

    let election_event =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id).await?;

    ballot_style::update_election_event_ballot_s3_files(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication,
        &election_event,
    )
    .await?;

    update_ballot_publication_d(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication_id.clone(),
        true,
        Some(ISO8601::now()),
    )
    .await?;

    let mut new_status: ElectionEventStatus =
        get_election_event_status(election_event.status).unwrap_or(Default::default());
    new_status.is_published = Some(true);
    let new_status_js = serde_json::to_value(new_status)?;

    update_election_event_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
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
        .map_err(|e| "error updating election status")?;
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .ok_or("missing bulletin board".to_string())?;

    let election_ids_str = match election_ids.len() > 1 {
        true => None,
        false => match election_ids.len() > 0 {
            true => Some(election_ids[0].clone()),
            false => None,
        },
    };

    // let electoral_log = ElectoralLog::new(board_name.as_str()).await?;
    let electoral_log = ElectoralLog::for_admin_user(
        &hasura_transaction,
        &board_name,
        &tenant_id,
        &election_event.id,
        &user_id,
        Some(username.clone()),
        election_ids_str.clone(),
        None,
    )
    .await?;
    electoral_log
        .post_election_published(
            election_event_id.clone(),
            election_ids_str,
            ballot_publication_id.clone(),
            Some(user_id),
            Some(username),
        )
        .await
        .map_err(|e| "error posting to the electoral log: {e:?}")?;

    lock.release().await?;
    Ok(())
}
