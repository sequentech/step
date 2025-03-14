// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::types::*;
use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use anyhow::{anyhow, Result};
use reqwest;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::reports::render_template_text;
use sequent_core::services::s3::{download_s3_file_to_string, get_public_asset_file_path};
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::util::date_time::generate_timestamp;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::{error, info, instrument, warn};

pub const PUBLIC_ASSETS_VOTERVIEW_SETVOTED_TEMPLATE: &str = "voterview_setvoted.hbs";
pub const PUBLIC_ASSETS_VOTERVIEW_SETNOTVOTED_TEMPLATE: &str = "voterview_setnotvoted.hbs";

impl SoapRequestData {
    pub fn new(county_mun: &str, usr: &str, psw: &str, voter_id: &str, timestamp: &str) -> Self {
        SoapRequestData {
            county_mun: county_mun.to_string(),
            usr: usr.to_string(),
            psw: psw.to_string(),
            voter_id: voter_id.to_string(),
            timestamp: timestamp.to_string(),
        }
    }
}

impl SoapRequest {
    pub async fn get_body(
        &self,
        annotations: &DatafixAnnotations,
        voter_id: &str,
        timestamp: &str,
    ) -> Result<String> {
        let data = SoapRequestData::new(
            &annotations.voterview_request.county_mun,
            &annotations.voterview_request.usr,
            &annotations.voterview_request.psw,
            voter_id,
            timestamp,
        );

        let variables_map: Map<String, Value> = deserialize_value(serde_json::to_value(data)?)
            .map_err(|e| anyhow!("Error deserializing data: {e:?}"))?;

        let template_path = match self {
            SoapRequest::SetVoted => PUBLIC_ASSETS_VOTERVIEW_SETVOTED_TEMPLATE,
            SoapRequest::SetNotVoted => PUBLIC_ASSETS_VOTERVIEW_SETNOTVOTED_TEMPLATE,
        };
        let s3_template_url = get_public_asset_file_path(template_path)
            .map_err(|e| anyhow!("Error fetching get_minio_url: {e:?}"))?;
        let template_string = download_s3_file_to_string(&s3_template_url).await?;
        // render handlebars template
        render_template_text(&template_string, variables_map).map_err(|err| anyhow!("{}", err))
    }
}

impl SoapRequestResponse {
    pub async fn new(
        response: reqwest::Response,
        req_type: SoapRequest,
    ) -> Result<SoapRequestResponse> {
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
            error!("Request to VoterView {req_type} failed with response status: {status}. Faultcode: {faultcode}, Faultstring: {faultstring}");
            return Ok(SoapRequestResponse::Faultstring(faultstring));
        }

        let success_element =
            parse_tag("<Success>", "</Success>", &response_txt).unwrap_or_default();
        match success_element.as_str() {
            "true" => {
                info!("Request to VoterView {req_type} succeeded");
                Ok(SoapRequestResponse::Ok)
            }
            "false" => {
                let error_message = parse_tag("<ErrorMessage>", "</ErrorMessage>", &response_txt)
                    .unwrap_or_default();
                if error_message.eq(&SoapRequestResponse::HasVotedErrorMsg.to_string()) {
                    Ok(SoapRequestResponse::HasVotedErrorMsg)
                } else {
                    warn!("VoterView responded with ErrorMessage: {error_message} to the {req_type} action.");
                    Ok(SoapRequestResponse::OtherErrorMsg(error_message))
                }
            }
            _ => Err(anyhow!("Failed to parse the response text: {response_txt}")),
        }
    }
}

#[instrument(skip(election_event), err)]
pub async fn send(
    req_type: SoapRequest,
    election_event: ElectionEventDatafix,
    username: &Option<String>,
) -> Result<SoapRequestResponse> {
    let timestamp = generate_timestamp(
        Some(TimeZone::UTC),
        Some(DateFormat::Custom("%Y-%m-%dT%H:%M:%S.%3fZ".to_string())),
        None,
    );
    // Datafix voter_id is the username!
    let voter_id = username.as_deref().ok_or(anyhow!("Username is None"))?;
    let annotations: DatafixAnnotations = election_event
        .get_annotations()
        .map_err(|err| anyhow!("Error getting election event annotations: {err}"))?;

    let soap_body = req_type
        .get_body(&annotations, voter_id, &timestamp)
        .await?;
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
    SoapRequestResponse::new(response, req_type).await
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
