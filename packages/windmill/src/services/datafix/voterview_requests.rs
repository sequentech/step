// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::types::{
    DatafixAnnotations, SoapRequest, SoapRequestData, SoapRequestResponse, SoapRequestResult,
};
use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use anyhow::{anyhow, Context, Result};
use reqwest::{Response, StatusCode};
use roxmltree::{Document, Node};
use sequent_core::services::reports::render_template_text;
use sequent_core::services::s3::{download_s3_file_to_string, get_public_asset_file_path};
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::util::date_time::generate_timestamp;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::instrument;

pub const PUBLIC_ASSETS_VOTERVIEW_SETVOTED_TEMPLATE: &str = "voterview_setvoted.hbs";
pub const PUBLIC_ASSETS_VOTERVIEW_SETNOTVOTED_TEMPLATE: &str = "voterview_setnotvoted.hbs";
pub const VOTERVIEW_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const VOTERVIEW_NAMESPACE: &str = "https://www.voterview.ca/MVVServices";
const SOAP_11_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const SOAP_12_NAMESPACE: &str = "http://www.w3.org/2003/05/soap-envelope";
const ALREADY_VOTED_MESSAGE: &str = "the voter has already voted";
const ALREADY_NOT_VOTED_MESSAGE: &str = "the voter has not voted";

pub struct PreparedSoapRequest {
    request: SoapRequest,
    client: reqwest::Client,
    url: String,
    body: String,
    template_sha256: String,
}

impl PreparedSoapRequest {
    pub fn template_sha256(&self) -> &str {
        &self.template_sha256
    }
}

impl SoapRequest {
    fn template_name(self) -> &'static str {
        match self {
            Self::SetVoted => PUBLIC_ASSETS_VOTERVIEW_SETVOTED_TEMPLATE,
            Self::SetNotVoted => PUBLIC_ASSETS_VOTERVIEW_SETNOTVOTED_TEMPLATE,
        }
    }

    fn operation_name(self) -> &'static str {
        match self {
            Self::SetVoted => "SetVoted",
            Self::SetNotVoted => "SetNotVoted",
        }
    }

    fn result_name(self) -> &'static str {
        match self {
            Self::SetVoted => "SetVotedResult",
            Self::SetNotVoted => "SetNotVotedResult",
        }
    }

    fn date_element_name(self) -> &'static str {
        match self {
            Self::SetVoted => "DateTimeVoted",
            Self::SetNotVoted => "DateTimeUnrecorded",
        }
    }
}

async fn render_request(
    request: SoapRequest,
    annotations: &DatafixAnnotations,
    voter_id: &str,
    timestamp: &str,
) -> Result<(String, String)> {
    let template_path = get_public_asset_file_path(request.template_name())
        .context("Error resolving the VoterView template path")?;
    let template = download_s3_file_to_string(&template_path)
        .await
        .context("Error downloading the VoterView template")?;
    let template_sha256 = hex::encode(Sha256::digest(template.as_bytes()));

    let data = SoapRequestData {
        county_mun: &annotations.voterview_request.county_mun,
        usr: &annotations.voterview_request.usr,
        psw: &annotations.voterview_request.psw,
        voter_id,
        timestamp,
    };
    let variables: Map<String, Value> = serde_json::to_value(&data)?
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("VoterView template data must be an object"))?;
    let body = render_template_text(&template, variables)
        .map_err(|err| anyhow!("Error rendering the VoterView template: {err}"))?;
    validate_request_xml(request, &body, &data)?;

    Ok((body, template_sha256))
}

fn validate_request_xml(
    request: SoapRequest,
    body: &str,
    expected: &SoapRequestData<'_>,
) -> Result<()> {
    let document = Document::parse(body).context("Rendered VoterView template is not valid XML")?;
    let soap_body = soap_body(&document)?;
    let operation = exactly_one_child(soap_body, request.operation_name(), VOTERVIEW_NAMESPACE)?;

    require_child_value(operation, "CountyMun", expected.county_mun)?;
    require_child_value(operation, "Username", expected.usr)?;
    require_child_value(operation, "Password", expected.psw)?;
    require_child_value(operation, "VoterID", expected.voter_id)?;
    require_child_value(operation, request.date_element_name(), expected.timestamp)?;
    if request == SoapRequest::SetVoted {
        require_child_value(operation, "Channel", "INTERNET")?;
    }
    Ok(())
}

fn soap_body<'a, 'input>(document: &'a Document<'input>) -> Result<Node<'a, 'input>> {
    let envelope = document.root_element();
    let namespace = envelope.tag_name().namespace();
    if envelope.tag_name().name() != "Envelope"
        || (namespace != Some(SOAP_11_NAMESPACE) && namespace != Some(SOAP_12_NAMESPACE))
    {
        return Err(anyhow!("Unexpected SOAP Envelope namespace"));
    }
    exactly_one_child(envelope, "Body", namespace.unwrap_or_default())
}

fn exactly_one_child<'a, 'input>(
    parent: Node<'a, 'input>,
    name: &str,
    namespace: &str,
) -> Result<Node<'a, 'input>> {
    let mut elements = parent.children().filter(|node| {
        node.is_element()
            && node.tag_name().name() == name
            && node.tag_name().namespace() == Some(namespace)
    });
    let element = elements
        .next()
        .ok_or_else(|| anyhow!("Missing XML element {name} in namespace {namespace}"))?;
    if elements.next().is_some() {
        return Err(anyhow!("Multiple XML elements named {name}"));
    }
    Ok(element)
}

fn require_child_value(parent: Node<'_, '_>, name: &str, expected: &str) -> Result<()> {
    let actual = child_text(parent, name, VOTERVIEW_NAMESPACE)?;
    if actual != expected {
        return Err(anyhow!(
            "Rendered XML element {name} does not match its input"
        ));
    }
    Ok(())
}

fn child_text(parent: Node<'_, '_>, name: &str, namespace: &str) -> Result<String> {
    let element = exactly_one_child(parent, name, namespace)?;
    element
        .text()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Missing or empty XML element {name}"))
}

fn normalize_message(message: &str) -> String {
    message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', '!', '?'])
        .to_lowercase()
}

fn parse_response(
    status: StatusCode,
    response_text: &str,
    request: SoapRequest,
) -> Result<SoapRequestResponse> {
    let document = Document::parse(response_text).context("VoterView returned malformed XML")?;
    let soap_body = soap_body(&document)?;

    if !status.is_success() {
        let fault = soap_body
            .descendants()
            .filter(|node| {
                node.is_element()
                    && matches!(node.tag_name().name(), "faultstring" | "Text" | "Reason")
            })
            .find_map(|node| node.text().map(str::trim).filter(|text| !text.is_empty()))
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Ok(SoapRequestResponse::Fault(fault));
    }

    let response_name = format!("{}Response", request.operation_name());
    let response = exactly_one_child(soap_body, &response_name, VOTERVIEW_NAMESPACE)?;
    let result = exactly_one_child(response, request.result_name(), VOTERVIEW_NAMESPACE)?;
    let success = child_text(result, "Success", VOTERVIEW_NAMESPACE)?;
    match success.to_lowercase().as_str() {
        "true" => Ok(SoapRequestResponse::Ok),
        "false" => {
            let message = child_text(result, "ErrorMessage", VOTERVIEW_NAMESPACE)?;
            match (request, normalize_message(&message).as_str()) {
                (SoapRequest::SetVoted, ALREADY_VOTED_MESSAGE) => {
                    Ok(SoapRequestResponse::AlreadyVoted)
                }
                (SoapRequest::SetNotVoted, ALREADY_NOT_VOTED_MESSAGE) => {
                    Ok(SoapRequestResponse::AlreadyNotVoted)
                }
                _ => Ok(SoapRequestResponse::Rejected(message)),
            }
        }
        value => Err(anyhow!("Unexpected VoterView Success value: {value}")),
    }
}

async fn read_response(response: Response, request: SoapRequest) -> Result<SoapRequestResponse> {
    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("Failed to read the VoterView response")?;
    parse_response(status, &response_text, request)
}

#[instrument(skip(election_event, username), fields(request = %request), err)]
pub async fn prepare(
    request: SoapRequest,
    election_event: ElectionEventDatafix,
    username: &Option<String>,
) -> Result<PreparedSoapRequest> {
    let timestamp = generate_timestamp(
        Some(TimeZone::UTC),
        Some(DateFormat::Custom("%Y-%m-%dT%H:%M:%S.%3fZ".to_string())),
        None,
    );
    let voter_id = username
        .as_deref()
        .ok_or_else(|| anyhow!("Username is None"))?;
    let annotations = election_event
        .get_annotations()
        .context("Invalid Datafix election event annotations")?;
    let (body, template_sha256) =
        render_request(request, &annotations, voter_id, &timestamp).await?;

    let client = reqwest::Client::builder()
        .timeout(VOTERVIEW_REQUEST_TIMEOUT)
        .build()
        .context("Failed to build the VoterView HTTP client")?;

    Ok(PreparedSoapRequest {
        request,
        client,
        url: annotations.voterview_request.url,
        body,
        template_sha256,
    })
}

#[instrument(skip(prepared), fields(request = %prepared.request), err)]
pub async fn send_prepared(prepared: PreparedSoapRequest) -> Result<SoapRequestResult> {
    let PreparedSoapRequest {
        request,
        client,
        url,
        body,
        template_sha256,
    } = prepared;
    let response = client
        .post(url)
        .header("Content-Type", "text/xml; charset=UTF-8")
        .header(
            "SOAPAction",
            format!("https://www.voterview.ca/MVVServices/{request}"),
        )
        .body(body)
        .send()
        .await
        .with_context(|| format!("VoterView request failed (template_sha256={template_sha256})"))?;
    let response = read_response(response, request).await.with_context(|| {
        format!("Invalid VoterView response (template_sha256={template_sha256})")
    })?;

    Ok(SoapRequestResult {
        response,
        template_sha256,
    })
}

#[instrument(skip(election_event, username), fields(request = %request), err)]
pub async fn send(
    request: SoapRequest,
    election_event: ElectionEventDatafix,
    username: &Option<String>,
) -> Result<SoapRequestResult> {
    let prepared = prepare(request, election_event, username).await?;
    send_prepared(prepared).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(result: &str, prefix: &str) -> String {
        format!(
            r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"
                xmlns:v="https://www.voterview.ca/MVVServices">
                <soap:Body><v:{prefix}Response><v:{prefix}Result>{result}</v:{prefix}Result>
                </v:{prefix}Response></soap:Body></soap:Envelope>"#
        )
    }

    #[test]
    fn parses_namespaced_success() {
        let xml = response("<v:Success> true </v:Success>", "SetVoted");
        assert_eq!(
            parse_response(StatusCode::OK, &xml, SoapRequest::SetVoted).unwrap(),
            SoapRequestResponse::Ok
        );
    }

    #[test]
    fn classifies_known_responses_after_normalizing_whitespace() {
        let already_voted = response(
            "<v:Success>false</v:Success><v:ErrorMessage> The voter  has already voted. </v:ErrorMessage>",
            "SetVoted",
        );
        assert_eq!(
            parse_response(StatusCode::OK, &already_voted, SoapRequest::SetVoted).unwrap(),
            SoapRequestResponse::AlreadyVoted
        );

        let already_not_voted = response(
            "<v:Success>false</v:Success><v:ErrorMessage>The voter has not voted.</v:ErrorMessage>",
            "SetNotVoted",
        );
        assert_eq!(
            parse_response(StatusCode::OK, &already_not_voted, SoapRequest::SetNotVoted).unwrap(),
            SoapRequestResponse::AlreadyNotVoted
        );
    }

    #[test]
    fn rejects_malformed_or_ambiguous_success_elements() {
        assert!(parse_response(StatusCode::OK, "not xml", SoapRequest::SetVoted).is_err());
        let duplicate = response(
            "<v:Success>true</v:Success><v:Success>true</v:Success>",
            "SetVoted",
        );
        assert!(parse_response(StatusCode::OK, &duplicate, SoapRequest::SetVoted).is_err());
    }

    #[test]
    fn rejects_success_from_an_unexpected_namespace() {
        let xml = response("<v:Success>true</v:Success>", "SetVoted")
            .replace(VOTERVIEW_NAMESPACE, "https://unexpected.example.test");
        assert!(parse_response(StatusCode::OK, &xml, SoapRequest::SetVoted).is_err());
    }

    #[test]
    fn rejects_a_template_that_changes_protocol_values() {
        let data = SoapRequestData {
            county_mun: "county",
            usr: "user",
            psw: "password",
            voter_id: "voter",
            timestamp: "timestamp",
        };
        let xml = format!(
            r#"<soap:Envelope xmlns:soap="{SOAP_11_NAMESPACE}">
                <soap:Body><SetVoted xmlns="{VOTERVIEW_NAMESPACE}">
                <CountyMun>county</CountyMun><Username>user</Username>
                <Password>password</Password><VoterID>different-voter</VoterID>
                <Channel>PHONE</Channel><DateTimeVoted>timestamp</DateTimeVoted>
                </SetVoted></soap:Body></soap:Envelope>"#
        );
        assert!(validate_request_xml(SoapRequest::SetVoted, &xml, &data).is_err());
    }

    #[test]
    fn parses_namespaced_soap_fault() {
        let xml = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
            <s:Body><s:Fault><s:Reason><s:Text>Service unavailable</s:Text></s:Reason>
            </s:Fault></s:Body></s:Envelope>"#;
        assert_eq!(
            parse_response(StatusCode::SERVICE_UNAVAILABLE, xml, SoapRequest::SetVoted).unwrap(),
            SoapRequestResponse::Fault("Service unavailable".to_string())
        );
    }
}
