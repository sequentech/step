use sequent_core::types::hasura::core::ElectionEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value as Jsonb;
use std::{collections::HashMap, convert::TryFrom};
use sequent_core::services::keycloak;
use anyhow::Result;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use crate::{hasura, postgres::tenant::{get_tenant_settings, Schedule}, types::resources::OrderDirection};
use tracing::{instrument, info};
use crate::postgres::{election_event::get_election_event_by_id, scheduled_event::{find_scheduled_event_by_election_event_id, PostgresScheduledEvent}};
use strum_macros::EnumString;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetEventListOutput {
    election: String,
    schedule: Option<String>,
    id: Option<String>,
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    event_type: String,
    receivers: Vec<String>,
    template: Jsonb,
    name: Option<String>,
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
    Name,
    TenantId,
    Schedule,
    Receivers,
    Template
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

impl TryFrom<(PostgresScheduledEvent, ElectionEvent)> for GetEventListOutput {
    type Error = String;

    fn try_from(event: (PostgresScheduledEvent, ElectionEvent)) -> Result<Self, Self::Error> {
        let (event_data, election) = event;
        Ok(GetEventListOutput {
            election: election.name,
            schedule: event_data.cron_config.map_or(None, |cc| Some(cc.scheduled_date?.to_string())),
            id: event_data.task_id,
            tenant_id: event_data.tenant_id,
            election_event_id: event_data.election_event_id,
            event_type: "event".to_string(),
            receivers: Vec::new(),
            template: serde_json::json!({}),
            name: event_data.event_processor.map_or(None, |ep| Some(ep.to_string())),
        })
    }
}

impl TryFrom<(Schedule, ElectionEvent)> for GetEventListOutput {
    type Error = String;

    fn try_from(event: (Schedule, ElectionEvent)) -> Result<Self, Self::Error> {
        let (event_data, election) = event;
        Ok(GetEventListOutput {
            election: election.name,
            schedule: Some(event_data.date),
            id: Some(event_data.id),
            tenant_id: Some(election.tenant_id),
            election_event_id: Some(election.id),
            event_type: "global event".to_string(),
            receivers: Vec::new(),
            template: serde_json::json!({}),
            name: Some(event_data.name),
        })
    }
}

#[instrument(skip(hasura_transaction), err(Debug))]
pub async fn get_all_scheduled_events_from_db(
    hasura_transaction: &Transaction<'_>,
    input: GetEventListInput,
) -> Result<EventListOutput, (Status, String)> {

    let tenant_settings = get_tenant_settings(hasura_transaction, input.tenant_id.as_str()).await.map_err(|err| {
        (Status::InternalServerError, format!("Failed to get tenant settings: {}", err))
    })?;
    info!("tenant_settings: {:?}", tenant_settings);
    let scheduled_events = find_scheduled_event_by_election_event_id(
        hasura_transaction,
        input.tenant_id.as_str(),
        input.election_event_id.as_str(),
    )
    .await
    .map_err(|err| {
        (Status::InternalServerError, format!("Failed to get scheduled events: {}", err))
    })?;

    let election_by_id = get_election_event_by_id(
        hasura_transaction,
        input.tenant_id.as_str(),
        input.election_event_id.as_str(),
    )
    .await
    .map_err(|err| {
        (Status::InternalServerError, format!("Failed to get election event: {}", err))
    })?;

    let election_event = election_by_id.clone(); 
    let auth_headers = keycloak::get_client_credentials().await
    .map_err(|err| {
        (Status::InternalServerError, format!("Failed to get election event: {}", err))
    })?;

    println!("auth headers: {:#?}", auth_headers);
    let hasura_response =
    hasura::tenant::get_tenant(auth_headers, input.tenant_id).await;

    info!("hasura_response: {:?}", hasura_response);

    let setting_schedules: Result<Vec<GetEventListOutput>, String> = tenant_settings
        .into_iter()
        .map(|setting| GetEventListOutput::try_from((setting, election_event.clone()))) 
        .collect();

    let scheduled_event: Result<Vec<GetEventListOutput>, String> = scheduled_events
        .into_iter()
        .map(|event| GetEventListOutput::try_from((event, election_event.clone()))) 
        .collect();

    info!("setting_schedules: {:?}", setting_schedules);
    info!("scheduled_event: {:?}", scheduled_event);

    let mut output: Vec<GetEventListOutput> = Vec::new();

    if let Ok(mut schedules) = setting_schedules {
        output.append(&mut schedules);
    }

    if let Ok(mut events) = scheduled_event {
        output.append(&mut events);
    }
    let total = output.len() as i32;
    let event_list_output = EventListOutput {
        items: output,
        total: total,
    };

    Ok(event_list_output)

}