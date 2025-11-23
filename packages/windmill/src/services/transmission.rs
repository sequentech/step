// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    postgres::tally_session::{get_tally_session_by_id, get_tally_sessions_by_election_event_id},
    types::miru_plugin::{MiruServerDocumentStatus, MiruTallySessionData},
};
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{Area, TallySession};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::consolidation::{
    eml_generator::ValidateAnnotations, send_transmission_package_service::get_latest_miru_document,
};

#[instrument(err, skip_all)]
pub async fn get_transmission_data_from_tally_session_by_area(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    tally_session_id: Option<String>,
) -> Result<MiruTallySessionData> {
    let tally_sessions: Vec<TallySession> = if let Some(tally_session_id) = tally_session_id {
        let tally_session = get_tally_session_by_id(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &tally_session_id,
        )
        .await
        .map_err(|err| anyhow!("Error getting the tally session: {err:?}"))?;
        vec![tally_session.clone()]
    } else {
        get_tally_sessions_by_election_event_id(
            hasura_transaction,
            tenant_id,
            election_event_id,
            false,
        )
        .await
        .map_err(|err| anyhow!("Error getting the tally sessions: {err:?}"))?
    };

    let tally_session_data: MiruTallySessionData = {
        if let Some(tally_session) = tally_sessions
            .iter()
            .filter(|session| {
                session.election_event_id == election_event_id
                    && match session.area_ids {
                        Some(ref area_ids) => area_ids.contains(&area_id.to_string()),
                        None => false,
                    }
            })
            .max_by_key(|session| session.created_at)
        {
            info!("********* FOUND {area_id}");
            tally_session
                .get_annotations_or_empty_values()
                .map_err(|err| anyhow!("Error getting valid annotations: {err}"))?
        } else {
            info!(
                "Tally session not found for the given election event and area, \
                setting default transmission status for area: {:?}",
                area_id
            );
            vec![]
        }
    };
    Ok(tally_session_data)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerData {
    pub server_code: String,
    pub transmitted: String,
    pub server_name: String,
    pub received: String,
    pub date_transmitted: String,
    pub date_received: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransmissionData {
    pub servers: Vec<ServerData>,
    pub total_transmitted: i64,
    pub total_not_transmitted: i64,
    pub last_date_transmitted: String,
}

#[instrument(err, skip_all)]
pub async fn get_transmission_servers_data(
    tally_session_data: &MiruTallySessionData,
    area: &Area,
) -> Result<TransmissionData> {
    let annotations = area.get_annotations_or_empty_values()?;

    let mut total_transmitted: i64 = 0;
    let mut total_not_transmitted: i64 = 0;

    let tally_area = tally_session_data.iter().find(|t| t.area_id == area.id);

    let document = tally_area.and_then(|ta| get_latest_miru_document(&ta.documents));

    let servers_sent_to = document
        .map(|d| d.servers_sent_to.clone())
        .unwrap_or_else(|| vec![]);

    let servers: Vec<ServerData> = annotations
        .ccs_servers
        .into_iter()
        .map(|server| ServerData {
            server_code: server.tag,
            transmitted: if servers_sent_to
                .iter()
                .any(|server_sent| server_sent.name == server.name)
            {
                total_transmitted += 1;
                "Transmitted".to_string()
            } else {
                total_not_transmitted += 1;
                "Not Transmitted".to_string()
            },
            date_transmitted: tally_session_data
                .iter()
                .find_map(|data| {
                    let server_name = server.name.clone();
                    servers_sent_to.iter().find_map(|server_sent| {
                        if server_sent.name == server_name {
                            Some(server_sent.sent_at.clone())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| "".to_string()),
            received: if tally_area
                .clone()
                .map(|data| {
                    servers_sent_to.iter().any(|server_sent| {
                        server_sent.name == server.name
                            && server_sent.status == MiruServerDocumentStatus::SUCCESS
                    })
                })
                .unwrap_or(false)
            {
                "Received".to_string()
            } else {
                "Not Received".to_string()
            },
            date_received: tally_session_data
                .iter()
                .find_map(|data| {
                    let server_name = server.name.clone();
                    servers_sent_to.iter().find_map(|server_sent| {
                        if server_sent.name == server_name
                            && server_sent.status == MiruServerDocumentStatus::SUCCESS
                        {
                            Some(server_sent.sent_at.clone())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| "".to_string()),
            server_name: server.name,
        })
        .collect();

    let last_date_transmitted = servers
        .iter()
        .max_by_key(|server| &server.date_transmitted)
        .map(|server| server.date_transmitted.clone());

    Ok(TransmissionData {
        servers,
        total_transmitted,
        total_not_transmitted,
        last_date_transmitted: last_date_transmitted.unwrap_or("".to_string()),
    })
}
