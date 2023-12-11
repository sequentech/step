// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::area::get_election_event_areas;
use crate::hasura::tally_session::{get_tally_session_highest_batch, insert_tally_session};
use crate::hasura::tally_session_contest::insert_tally_session_contest;
use crate::hasura::trustee::get_trustees_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::users::list_users;
use crate::tasks::insert_ballots::{insert_ballots, InsertBallotsPayload};
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use braid_messages::newtypes::BatchNumber;
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::ceremonies::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::default::Default;
use strum_macros::{Display, EnumString};
use tracing::{event, instrument, Level};

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum AudienceSelection {
    #[strum(serialize = "ALL_USERS")]
    ALL_USERS,
    #[strum(serialize = "NOT_VOTED")]
    NOT_VOTED,
    #[strum(serialize = "VOTED")]
    VOTED,
    #[strum(serialize = "SELECTED")]
    SELECTED,
}

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
enum CommunicationType {
    #[strum(serialize = "CREDENTIALS")]
    CREDENTIALS,
    #[strum(serialize = "RECEIPT")]
    RECEIPT,
}

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
enum CommunicationMethod {
    #[strum(serialize = "EMAIL")]
    EMAIL,
    #[strum(serialize = "SMS")]
    SMS,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct EmailConfig {
    subject: String,
    plaintext_body: String,
    html_body: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SmsConfig {
    message: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SendCommunicationBody {
    audience_selection: AudienceSelection,
    audience_voter_ids: Option<Vec<String>>,
    communication_type: CommunicationType,
    communication_method: CommunicationMethod,
    schedule_now: bool,
    schedule_date: Option<String>,
    email: Option<EmailConfig>,
    sms: Option<SmsConfig>,
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn send_communication(
    body: SendCommunicationBody,
    tenant_id: String,
    election_event_id: Option<String>,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;
    let client = KeycloakAdminClient::new()
        .await?;
    let realm = match election_event_id {
        Some(ref election_event_id) => {
            get_event_realm(&tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&tenant_id),
    };

    let (users, count) = list_users(
        auth_headers.clone(),
        &client,
        tenant_id.clone(),
        election_event_id.clone(),
        &realm,
        None,
        None,
        None,
        None
    )
    .await?;
    users
        .iter()
        .filter(|user| (
            body.audience_selection == AudienceSelection::ALL_USERS ||
            (
                body.audience_selection == AudienceSelection::SELECTED &&
                user.id.is_some() &&
                body
                    .audience_voter_ids
                    .as_ref()
                    .unwrap_or(&vec![])
                    .contains(&user.id.as_ref().unwrap())
            )
        ))
        .for_each(|user| {
            event!(
                Level::INFO,
                "Sending communication to user with id={:?} and email={:?}",
                id=user.id,
                email=user.email,
            );
        });
    Ok(())
}
