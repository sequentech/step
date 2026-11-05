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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn area() -> Area {
        serde_json::from_value(json!({
            "id": "area",
            "tenant_id": "tenant",
            "election_event_id": "event",
            "annotations": {
                "miru:area-ccs-servers": json!([
                    {"name": "CCS Manila (SBOC)", "tag": "ccs-manila", "address": "https://ccs.invalid", "public_key_pem": "", "send_logs": false},
                    {"name": "Central Server", "tag": "central", "address": "https://central.invalid", "public_key_pem": "", "send_logs": false},
                    {"name": "Backup Server", "tag": "backup", "address": "", "public_key_pem": "", "send_logs": false},
                ]).to_string(),
                "miru:area-station-id": "90010001",
                "miru:area-station-name": "SPAIN",
                "miru:area-threshold": "2",
                "miru:area-trustee-users": "[]",
                "miru:area-country": "Spain",
                "miru:registered-voters": "1250",
            },
        }))
        .unwrap()
    }

    fn package(sent: serde_json::Value) -> MiruTallySessionData {
        serde_json::from_value(json!([{
            "election_id": "election",
            "area_id": "area",
            "servers": [],
            "logs": [],
            "threshold": 2,
            "documents": [{
                "document_ids": {"eml": "eml", "xz": "xz", "all_servers": "all"},
                "transaction_id": "1",
                "servers_sent_to": sent,
                "created_at": "2028-05-09T10:00:00Z",
                "signatures": [],
            }],
        }]))
        .unwrap()
    }

    #[tokio::test]
    async fn each_destination_says_whether_it_was_sent_and_received_the_er_and_when() {
        let data = get_transmission_servers_data(
            &package(json!([
                {"name": "CCS Manila (SBOC)", "sent_at": "2028-05-09T11:05:00Z", "status": "SUCCESS"},
                {"name": "Central Server", "sent_at": "2028-05-09T11:06:00Z", "status": "ERROR"},
            ])),
            &area(),
        )
        .await
        .unwrap();
        let statuses: Vec<_> = data
            .servers
            .iter()
            .map(|server| {
                (
                    server.server_name.as_str(),
                    server.transmitted.as_str(),
                    server.date_transmitted.as_str(),
                    server.received.as_str(),
                    server.date_received.as_str(),
                )
            })
            .collect();
        assert_eq!(
            statuses,
            [
                (
                    "CCS Manila (SBOC)",
                    "Transmitted",
                    "2028-05-09T11:05:00Z",
                    "Received",
                    "2028-05-09T11:05:00Z"
                ),
                (
                    "Central Server",
                    "Transmitted",
                    "2028-05-09T11:06:00Z",
                    "Not Received",
                    ""
                ),
                ("Backup Server", "Not Transmitted", "", "Not Received", ""),
            ]
        );
        assert_eq!((data.total_transmitted, data.total_not_transmitted), (2, 1));
        assert_eq!(data.last_date_transmitted, "2028-05-09T11:06:00Z");
    }

    #[tokio::test]
    async fn an_area_without_a_package_has_sent_nothing() {
        let data = get_transmission_servers_data(&vec![], &area())
            .await
            .unwrap();
        assert!(data.servers.iter().all(
            |server| server.transmitted == "Not Transmitted" && server.date_received.is_empty()
        ));
        assert_eq!((data.total_transmitted, data.total_not_transmitted), (0, 3));
    }
}
