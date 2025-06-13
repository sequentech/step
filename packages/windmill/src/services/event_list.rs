// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::{insert_new_scheduled_event, insert_scheduled_event};
use anyhow::Result;
use deadpool_postgres::Transaction;
use electoral_log::client::types::OrderDirection;
use rocket::http::Status;
use sequent_core::services::keycloak;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as Jsonb;
use std::{collections::HashMap, convert::TryFrom};
use strum_macros::EnumString;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetEventListOutput {
    election: String,
    schedule: Option<String>,
    task_id: Option<String>,
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    event_type: Option<String>,
    id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EventListOutput {
    pub items: Vec<GetEventListOutput>,
    pub total: i32,
}

#[derive(Debug, Deserialize, Hash, PartialEq, Eq, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OrderField {
    Election,
    EventType,
    TenantId,
    Schedule,
    Id,
}

#[derive(Debug, Deserialize)]
pub struct GetEventListInput {
    pub tenant_id: String,
    pub election_event_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filter: Option<HashMap<OrderField, String>>,
    pub order_by: Option<HashMap<OrderField, OrderDirection>>,
}

impl TryFrom<(ScheduledEvent, ElectionEvent)> for GetEventListOutput {
    type Error = String;

    fn try_from(event: (ScheduledEvent, ElectionEvent)) -> Result<Self, Self::Error> {
        let (event_data, election) = event;
        Ok(GetEventListOutput {
            election: event_data
                .event_payload
                .as_ref()
                .and_then(|payload| payload.get("election_id"))
                .and_then(|id| id.as_str())
                .unwrap_or_default()
                .to_string(),
            schedule: event_data
                .cron_config
                .map_or(None, |cc| Some(cc.scheduled_date?.to_string())),
            task_id: event_data.task_id.clone(),
            tenant_id: event_data.tenant_id,
            election_event_id: event_data.election_event_id,
            event_type: event_data
                .event_processor
                .map_or(None, |ep| Some(ep.to_string())),
            id: Some(event_data.id),
        })
    }
}
