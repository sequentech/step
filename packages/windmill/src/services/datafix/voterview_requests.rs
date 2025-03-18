// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::types::*;
use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use anyhow::{anyhow, Result};
use reqwest;
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::util::date_time::generate_timestamp;
use tracing::{info, instrument};

impl SoapRequest {
    fn get_set_not_voted_body(
        annotations: &DatafixAnnotations,
        voter_id: &str,
        timestamp: &str,
    ) -> String {
        let county_mun = &annotations.voterview_request.county_mun;
        let usr = &annotations.voterview_request.usr;
        let psw = &annotations.voterview_request.psw;
        format!(
            r#"
            <soap:Envelope xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
                <soap:Body>
                    <SetNotVoted xmlns="https://www.voterview.ca/MVVServices">
                        <CountyMun>{county_mun}</CountyMun>
                        <Username>{usr}</Username>
                        <Password>{psw}</Password>
                        <VoterID>{voter_id}</VoterID>
                        <DateTimeUnrecorded>{timestamp}</DateTimeUnrecorded>
                    </SetNotVoted>
                </soap:Body>
            </soap:Envelope>
            "#
        )
    }
    fn get_set_voted_body(
        annotations: &DatafixAnnotations,
        voter_id: &str,
        timestamp: &str,
    ) -> String {
        let county_mun = &annotations.voterview_request.county_mun;
        let usr = &annotations.voterview_request.usr;
        let psw = &annotations.voterview_request.psw;
        format!(
            r#"
            <soap:Envelope xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
                <soap:Body>
                    <SetVoted xmlns="https://www.voterview.ca/MVVServices">
                        <CountyMun>{county_mun}</CountyMun>
                        <Username>{usr}</Username>
                        <Password>{psw}</Password>
                        <VoterID>{voter_id}</VoterID>
                        <Channel>INTERNET</Channel>
                        <DateTimeVoted>{timestamp}</DateTimeVoted>
                    </SetVoted>
                </soap:Body>
            </soap:Envelope>
            "#
        )
    }

    pub fn get_body(
        &self,
        annotations: &DatafixAnnotations,
        voter_id: &str,
        timestamp: &str,
    ) -> String {
        match self {
            SoapRequest::SetVoted => Self::get_set_voted_body(annotations, voter_id, timestamp),
            SoapRequest::SetNotVoted => {
                Self::get_set_not_voted_body(annotations, voter_id, timestamp)
            }
        }
    }
}

#[instrument(skip(election_event), err)]
pub async fn send(
    req_type: SoapRequest,
    election_event: ElectionEventDatafix,
    username: &Option<String>,
) -> Result<()> {
    let timestamp = generate_timestamp(
        Some(TimeZone::UTC),
        Some(DateFormat::Custom("%Y-%m-%dT%H:%M:%S.%3fZ".to_string())),
        None,
    );

    let voter_id = match username.to_owned() {
        Some(id) => id,
        _ => {
            return Err(anyhow!(
                "Cannot send the request to datafix because the username is None"
            ));
        }
    };
    let annotations: DatafixAnnotations = election_event
        .get_annotations()
        .map_err(|err| anyhow!("Error getting election event annotations: {err}"))?;

    let soap_body = req_type.get_body(&annotations, &voter_id, &timestamp);
    let url = &annotations.voterview_request.url;
    info!("Soap body: {soap_body}");
    info!("URL: {url}");

    let http = reqwest::Client::new();
    let response = http
        .post(url)
        .header("Content-Type", "text/xml; charset=UTF-8")
        .header(
            "SOAPAction",
            format!("https://www.voterview.ca/MVVServices/{req_type}"),
        )
        .body(soap_body)
        .send()
        .await
        .map_err(|err| anyhow!("Failed to get SOAP response: {err}"))?;

    let status = response.status();
    let response_txt = response
        .text()
        .await
        .map_err(|err| anyhow!("Failed to get the full response text: {err}"))?;

    info!("Response: {response_txt}");
    if !status.is_success() {
        let faultcode: String =
            parse_tag("<faultcode>", "</faultcode>", &response_txt).unwrap_or_default();
        let faultstring: String =
            parse_tag("<faultstring>", "</faultstring>", &response_txt).unwrap_or_default();
        return Err(anyhow!(
            "Request to VoterView {req_type} failed with response status: {status}. Faultcode: {faultcode}, Faultstring: {faultstring}"
        ));
    }

    let success_element = parse_tag("<Success>", "</Success>", &response_txt).unwrap_or_default();
    match success_element.as_str() {
        "true" => {
            info!("Request to VoterView {req_type} succeeded");
            Ok(())
        }
        "false" => {
            let error_message =
                parse_tag("<ErrorMessage>", "</ErrorMessage>", &response_txt).unwrap_or_default();
            Err(anyhow!(
                "Request to VoterView {req_type} failed with ErrorMessage: {error_message}"
            ))
        }
        _ => Err(anyhow!("Failed to parse the response text: {response_txt}")),
    }
}

pub fn parse_tag(open_tag: &str, close_tag: &str, response_txt: &str) -> Option<String> {
    match response_txt.split(open_tag).collect::<Vec<&str>>() {
        after if after.len() > 1 => match after[1].split(close_tag).collect::<Vec<&str>>() {
            before if before.len() > 1 => Some(before[0].to_string()),
            _ => None,
        },
        _ => None,
    }
}
