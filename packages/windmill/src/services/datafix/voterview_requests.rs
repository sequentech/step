// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use std::time::Duration;
use tracing::{error, info, instrument, warn};

pub const PUBLIC_ASSETS_VOTERVIEW_SETVOTED_TEMPLATE: &str = "voterview_setvoted.hbs";
pub const PUBLIC_ASSETS_VOTERVIEW_SETNOTVOTED_TEMPLATE: &str = "voterview_setnotvoted.hbs";

/// Whole-request timeout for the VoterView SOAP calls. Must stay well below
/// the 120s process_cast_vote PgLock expiry: a hung request outliving the
/// lock would let a re-enqueued task process the same vote concurrently.
pub const VOTERVIEW_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

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

    /// Electoral-log operation string for a failed request. VoterView's
    /// message, when there is one, goes after the ':' so it reaches the log
    /// message body while the log description keeps only the short outcome.
    pub fn failed_operation(&self, error_message: Option<String>) -> String {
        match error_message {
            Some(msg) => format!("{self} Failed: {msg}"),
            None => format!("{self} Failed"),
        }
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

        Self::from_parts(status, &response_txt, &req_type)
    }

    fn from_parts(
        status: reqwest::StatusCode,
        response_txt: &str,
        req_type: &SoapRequest,
    ) -> Result<SoapRequestResponse> {
        if !status.is_success() {
            let faultcode: String =
                parse_tag("<faultcode>", "</faultcode>", &response_txt).unwrap_or_default();
            let faultstring: String =
                parse_tag("<faultstring>", "</faultstring>", &response_txt).unwrap_or_default();
            error!("Request to VoterView {req_type} failed with response status: {status}. Faultcode: {faultcode}, Faultstring: {faultstring}");
            // An HTTP error may carry no SOAP fault at all (e.g. an HTML
            // gateway error page); fall back to the HTTP status so the
            // electoral log still records the reason.
            let faultstring = if faultstring.is_empty() {
                format!("HTTP {status}")
            } else {
                faultstring
            };
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

    /// VoterView's message for a failed response, if any.
    pub fn error_message(&self) -> Option<String> {
        match self {
            SoapRequestResponse::Ok => None,
            SoapRequestResponse::HasVotedErrorMsg => Some(self.to_string()),
            SoapRequestResponse::Faultstring(msg) | SoapRequestResponse::OtherErrorMsg(msg) => {
                (!msg.is_empty()).then(|| msg.clone())
            }
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
    let http = reqwest::Client::builder()
        .timeout(VOTERVIEW_REQUEST_TIMEOUT)
        .build()
        .map_err(|err| anyhow!("Failed to build the HTTP client: {err}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_per_response_variant() {
        assert_eq!(SoapRequestResponse::Ok.error_message(), None);
        assert_eq!(
            SoapRequestResponse::HasVotedErrorMsg.error_message(),
            Some("The voter has already voted.".to_string())
        );
        assert_eq!(
            SoapRequestResponse::OtherErrorMsg("The voter has not voted.".to_string())
                .error_message(),
            Some("The voter has not voted.".to_string())
        );
        assert_eq!(
            SoapRequestResponse::Faultstring("Server was unable to process request.".to_string())
                .error_message(),
            Some("Server was unable to process request.".to_string())
        );
        // Fault strings can be missing from the response and default to "".
        assert_eq!(
            SoapRequestResponse::Faultstring(String::new()).error_message(),
            None
        );
    }

    #[test]
    fn http_error_without_soap_fault_falls_back_to_status() {
        let response = SoapRequestResponse::from_parts(
            reqwest::StatusCode::GATEWAY_TIMEOUT,
            "<html><body>Gateway Timeout</body></html>",
            &SoapRequest::SetNotVoted,
        )
        .unwrap();
        assert_eq!(
            response.error_message(),
            Some("HTTP 504 Gateway Timeout".to_string())
        );
    }

    #[test]
    fn http_error_with_soap_fault_keeps_faultstring() {
        let response = SoapRequestResponse::from_parts(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "<soap:Fault><faultcode>soap:Server</faultcode>\
             <faultstring>Server was unable to process request.</faultstring></soap:Fault>",
            &SoapRequest::SetVoted,
        )
        .unwrap();
        assert_eq!(
            response.error_message(),
            Some("Server was unable to process request.".to_string())
        );
    }

    #[test]
    fn success_false_returns_voterview_error_message() {
        let response_txt = r#"<?xml version="1.0" encoding="utf-8"?><soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema"><soap:Body><SetNotVotedResponse xmlns="https://www.voterview.ca/MVVServices"><SetNotVotedResult><Success>false</Success><ErrorMessage>The voter has not voted.</ErrorMessage></SetNotVotedResult></SetNotVotedResponse></soap:Body></soap:Envelope>"#;
        let response = SoapRequestResponse::from_parts(
            reqwest::StatusCode::OK,
            response_txt,
            &SoapRequest::SetNotVoted,
        )
        .unwrap();
        assert_eq!(
            response.error_message(),
            Some("The voter has not voted.".to_string())
        );
        assert_eq!(
            SoapRequest::SetNotVoted.failed_operation(response.error_message()),
            "SetNotVoted Failed: The voter has not voted."
        );
    }

    #[test]
    fn failed_operation_includes_detail_after_colon() {
        assert_eq!(
            SoapRequest::SetNotVoted.failed_operation(Some("The voter has not voted.".to_string())),
            "SetNotVoted Failed: The voter has not voted."
        );
        assert_eq!(
            SoapRequest::SetVoted.failed_operation(None),
            "SetVoted Failed"
        );
    }
}
