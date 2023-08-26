// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use reqwest;

use crate::routes::scheduled_event;
use crate::connection;
use crate::hasura::event_execution;

pub async fn process_scheduled_event(
    auth_headers: connection::AuthHeaders,
    event: scheduled_event::ScheduledEvent,
) -> Result<(), reqwest::Error> {
    if event.event_processor.is_none() {
        return Ok(());
    }
    match event.event_processor.unwrap() {
        scheduled_event::EventProcessors::CreateReport => {
            let event_execution = event_execution::insert_event_execution(
                auth_headers,
                event.tenant_id,
                event.election_event_id,
                event.id,
                "started".to_string(),
                event.event_payload,
                None,
            ).await?;
        }
    }

    Ok(())
}
