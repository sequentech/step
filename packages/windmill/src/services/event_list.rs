use sequent_core::types::hasura::core::ElectionEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value as Jsonb;
use std::{collections::HashMap, convert::TryFrom};
use sequent_core::services::keycloak;
use anyhow::Result;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use crate::{hasura, postgres::{scheduled_event::{insert_new_scheduled_event, insert_scheduled_event}, tenant::Schedule}, services::election_event_dates::generate_manage_date_task_name, types::resources::OrderDirection};
use tracing::{instrument, info};
use crate::postgres::{election_event::get_election_event_by_id, scheduled_event::{find_scheduled_event_by_election_event_id, PostgresScheduledEvent}};
use strum_macros::EnumString;

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

impl TryFrom<(PostgresScheduledEvent, ElectionEvent)> for GetEventListOutput {
    type Error = String;

    fn try_from(event: (PostgresScheduledEvent, ElectionEvent)) -> Result<Self, Self::Error> {
        let (event_data, election) = event;
        Ok(GetEventListOutput {
            election: event_data.event_payload
                .as_ref()
                .and_then(|payload| payload.get("election_id"))
                .and_then(|id| id.as_str())
                .unwrap_or_default()
                .to_string(),
            schedule: event_data.cron_config.map_or(None, |cc| Some(cc.scheduled_date?.to_string())),
            task_id: event_data.task_id.clone(),
            tenant_id: event_data.tenant_id,
            election_event_id: event_data.election_event_id,
            event_type: event_data.event_processor.map_or(None, |ep| Some(ep.to_string())),
            id: Some(event_data.id),
        })
    }
}

#[instrument(skip(hasura_transaction), err(Debug))]
pub async fn get_all_scheduled_events_from_db(
    hasura_transaction: &Transaction<'_>,
    input: GetEventListInput,
) -> Result<EventListOutput, (Status, String)> {
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
            (Status::InternalServerError, format!("Failed to get client credentials: {}", err))
        })?;

    let hasura_response = hasura::tenant::get_tenant(auth_headers, input.tenant_id).await;
    info!("hasura_response: {:?}", hasura_response);

    let scheduled_event: Result<Vec<GetEventListOutput>, String> = scheduled_events
        .into_iter()
        .map(|event| GetEventListOutput::try_from((event, election_event.clone()))) 
        .collect();

    info!("scheduled_event: {:?}", scheduled_event);

    let mut output: Vec<GetEventListOutput> = Vec::new();
    if let Ok(mut events) = scheduled_event {
        output.append(&mut events);
    }

    if let Some(filters) = &input.filter {
        output.retain(|item| {
            filters.iter().all(|(field, value)| match field {
                OrderField::Election => item.election.contains(value),
                OrderField::EventType => item.event_type.as_ref().map_or(false, |et| et.contains(value)),
                OrderField::TenantId => item.tenant_id.as_ref().map_or(false, |id| id.contains(value)),
                OrderField::Schedule => item.schedule.as_ref().map_or(false, |s| s.contains(value)),
                OrderField::Id => item.id.as_ref().map_or(false, |id| id.contains(value)),
            })
        });
    }

    if let Some(order_by) = &input.order_by {
        output.sort_by(|a, b| {
            order_by.iter().fold(std::cmp::Ordering::Equal, |acc, (field, direction)| {
                if acc != std::cmp::Ordering::Equal {
                    return acc;
                }
                let ordering = match field {
                    OrderField::Election => a.election.cmp(&b.election),
                    OrderField::EventType => a.event_type.cmp(&b.event_type),
                    OrderField::TenantId => a.tenant_id.cmp(&b.tenant_id),
                    OrderField::Schedule => a.schedule.cmp(&b.schedule),
                    OrderField::Id => a.id.cmp(&b.id),
                };
                match direction {
                    OrderDirection::Asc => ordering,
                    OrderDirection::Desc => ordering.reverse(),
                }
            })
        });
    }

    let start = input.offset.unwrap_or(0) as usize;
    let end = (start + input.limit.unwrap_or(output.len() as i64) as usize).min(output.len());
    let paginated_output = if start < output.len() {
        output[start..end].to_vec()
    } else {
        Vec::new()
    };

    let total = output.len() as i32;
    let event_list_output = EventListOutput {
        items: paginated_output,
        total: total,
    };

    Ok(event_list_output)
}

#[instrument(skip(hasura_transaction), err(Debug))]
pub async fn create_event_in_db(
    hasura_transaction: &Transaction<'_>,
    event: PostgresScheduledEvent,
) -> Result<PostgresScheduledEvent, (Status, String)> {
    info!("Creating event2 {:?}", event);
    let new_event = PostgresScheduledEvent {
        event_payload: event.event_payload,
        cron_config: event.cron_config,
        id: event.id.clone(),
        tenant_id: event.tenant_id.clone(),
        election_event_id: event.election_event_id.clone(),
        event_processor: event.event_processor,
        created_at: Some(chrono::Utc::now()),
        stopped_at: event.stopped_at,
        labels: event.labels,
        annotations: event.annotations,
        task_id: Some(format!(
            "tenant_{}_event_{}",
            event.tenant_id.unwrap_or_default(),
            event.election_event_id.unwrap_or_default(),
        )),
    };

    let result = insert_new_scheduled_event(
        hasura_transaction,
        new_event.clone(),
    ).await;
    result.map(|_| new_event).map_err(|err| {
        (Status::InternalServerError, format!("Failed to create event: {}", err))
    })
}