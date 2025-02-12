// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::types::*;
use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use anyhow::{anyhow, Result};
use tracing::{error, info, instrument, warn};

impl SoapRequest {
    fn get_set_not_voted_body(annotations: &DatafixAnnotations, voter_id: &str) -> String {
        let county_mun = &annotations.voterview_request.county_mun;
        let usr = &annotations.voterview_request.usr;
        let psw = &annotations.voterview_request.psw;
        format!(
            r#"
            <soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
                <soap:Body>
                    <SetNotVoted xmlns="https://www.voterview.ca/MVVServices/">
                        <CountyMun>{county_mun}</CountyMun>
                        <Username>{usr}</Username>
                        <Password>{psw}</Password>
                        <VoterID>{voter_id}</VoterID>
                        <DateTimeUnrecorded>2024-01-21T16:33:04.247Z</DateTimeUnrecorded>
                    </SetNotVoted>
                </soap:Body>
            </soap:Envelope>
        "#
        )
    }
    fn get_set_voted_body(annotations: &DatafixAnnotations, voter_id: &str) -> String {
        let county_mun = &annotations.voterview_request.county_mun;
        let usr = &annotations.voterview_request.usr;
        let psw = &annotations.voterview_request.psw;
        format!(
            r#"
            <soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema"> 
                <soap:Body>
                    <SetVoted xmlns="https://www.voterview.ca/MVVServices/">
                        <CountyMun>{county_mun}</CountyMun>
                        <Username>{usr}</Username>
                        <Password>{psw}</Password>
                        <VoterID>{voter_id}</VoterID>
                        <Channel>Internet</Channel>
                        <DateTimeVoted>2024-01-21T16:33:04.247Z</DateTimeVoted>
                    </SetVoted>
                </soap:Body>
            </soap:Envelope>

        "#
        )
    }

    pub fn get_body(&self, annotations: &DatafixAnnotations, voter_id: &str) -> String {
        match self {
            SoapRequest::SetVoted => Self::get_set_voted_body(annotations, voter_id),
            SoapRequest::SetNotVoted => Self::get_set_not_voted_body(annotations, voter_id),
        }
    }
}

#[instrument(skip(election_event), err)]
pub async fn send(
    req_type: SoapRequest,
    election_event: ElectionEventDatafix,
    username: &Option<String>,
    // WIP_ timestap: ...
) -> Result<()> {
    let voter_id = match username.to_owned() {
        Some(id) => id,
        None => {
            return Err(anyhow!(
                "Cannot send the request to datafix because the username is None"
            ));
        }
    };
    let annotations: DatafixAnnotations = election_event
        .get_annotations()
        .map_err(|err| anyhow!("Error getting election event annotations: {err}"))?;
    info!("Election event annotations: {annotations:?}");

    let content = req_type.get_body(&annotations, &voter_id);
    info!("Soup content: {content}");

    // WIP: Send the request
    Ok(())
}
