// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::area::get_election_event_areas;
use crate::hasura::tally_session::{get_tally_session_highest_batch, insert_tally_session};
use crate::hasura::tally_session_contest::insert_tally_session_contest;
use crate::hasura::trustee::get_trustees_by_id;
use crate::services::celery_app::get_celery_app;
use crate::tasks::insert_ballots::{insert_ballots, InsertBallotsPayload};
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use braid_messages::newtypes::BatchNumber;
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::default::Default;
use tracing::{event, instrument, Level};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub enum SendCommunicationAudience {
    AllCensus,
    NotVoted,
    Voted,
    UserIds(Vec<String>),
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub enum SendCommunicationMethod {
    Email,
    Sms,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub enum SendCommunicationMethods {
    FromTemplate,
    Custom(Vec<SendCommunicationMethod>),
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub enum SendCommunicationLanguages {
    FromTemplate,
    Custom(Vec<String>),
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub enum SendCommunicationTemplate {
    Id((String, SendCommunicationMethods, SendCommunicationLanguages)),
    Custom(
        (
            HashMap<String, String>,
            Vec<SendCommunicationMethod>,
            Vec<String>,
        ),
    ),
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SendCommunicationBody {
    audience: SendCommunicationAudience,
    template: SendCommunicationTemplate,
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn send_communication(
    body: SendCommunicationBody,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;
    // TODO
    Ok(())
}
