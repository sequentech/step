// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::datafix_types::{
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

const SOAP_11_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const SOAP_12_NAMESPACE: &str = "http://www.w3.org/2003/05/soap-envelope";
const ALREADY_VOTED_MESSAGE: &str = "the voter has already voted";
const ALREADY_NOT_VOTED_MESSAGE: &str = "the voter has not voted";

/// A fully rendered, validated and hashed SOAP request that has not yet been
/// dispatched. Producing one is fallible and side-effect-free, so the worker can
/// [`prepare`] it *before* claiming a vote and leave the row recoverable if the
/// template or config is wrong; [`send_prepared`] then consumes it.
pub struct PreparedSoapRequest {
    request: SoapRequest,
    client: reqwest::Client,
    url: String,
    body: String,
    template_sha256: String,
    operation_namespace: String,
}

impl PreparedSoapRequest {
    /// SHA-256 of the template that produced this request, for the audit trail.
    #[instrument(skip_all)]
    pub fn template_sha256(&self) -> &str {
        &self.template_sha256
    }
}

impl SoapRequest {
    #[instrument]
    fn template_name(self) -> &'static str {
        match self {
            Self::SetVoted => PUBLIC_ASSETS_VOTERVIEW_SETVOTED_TEMPLATE,
            Self::SetNotVoted => PUBLIC_ASSETS_VOTERVIEW_SETNOTVOTED_TEMPLATE,
        }
    }

    #[instrument]
    fn operation_name(self) -> &'static str {
        match self {
            Self::SetVoted => "SetVoted",
            Self::SetNotVoted => "SetNotVoted",
        }
    }

    #[instrument]
    fn result_name(self) -> &'static str {
        match self {
            Self::SetVoted => "SetVotedResult",
            Self::SetNotVoted => "SetNotVotedResult",
        }
    }
}

/// The rendered request body, plus everything about the render that
/// [`PreparedSoapRequest`] needs to carry forward: the template's hash for the
/// audit trail, and the operation namespace extracted from the rendered body
/// (see [`operation_namespace`]).
struct RenderedSoapRequest {
    body: String,
    template_sha256: String,
    operation_namespace: String,
}

/// Renders the request's Handlebars template with the annotation values. The
/// rendered output is checked for the invariants a correct template can never
/// violate (see [`validate_rendered_xml`]), and its operation namespace is
/// extracted (see [`operation_namespace`]); the rest of its structure is left to
/// the template.
#[instrument(skip(annotations), err)]
async fn render_request(
    request: SoapRequest,
    annotations: &DatafixAnnotations,
    voter_id: &str,
    timestamp: &str,
) -> Result<RenderedSoapRequest> {
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
    validate_rendered_xml(&body, &data)?;
    let operation_namespace = operation_namespace(&body, request)?;

    Ok(RenderedSoapRequest {
        body,
        template_sha256,
        operation_namespace,
    })
}

/// Validates only the invariants a correct template can never violate: the
/// rendered body is well-formed XML, and every injected value survived rendering
/// as escaped text (not markup) rather than being dropped or turned into an
/// element. This is a near-zero-flexibility-cost check by design: it deliberately
/// asserts nothing about element names, the operation, or the namespace, because
/// those are owned by the hot-swappable S3 template — a VoterView-side change
/// stays a template edit, not a code change. Well-formedness catches template
/// typos; the value check catches a mistyped Handlebars variable (renders empty)
/// and an escaping bug that would let voter-supplied data inject XML.
#[instrument(skip_all, err)]
fn validate_rendered_xml(body: &str, expected: &SoapRequestData<'_>) -> Result<()> {
    let document = Document::parse(body).context("Rendered VoterView template is not valid XML")?;
    let texts: Vec<&str> = document
        .descendants()
        .filter_map(|node| node.text())
        .map(str::trim)
        .collect();
    let injected = [
        expected.county_mun,
        expected.usr,
        expected.psw,
        expected.voter_id,
        expected.timestamp,
    ];
    for value in injected {
        if value.is_empty() || !texts.contains(&value) {
            return Err(anyhow!(
                "Injected value missing from rendered XML; check template variables and escaping"
            ));
        }
    }
    Ok(())
}

/// Returns the SOAP `Body`, rejecting anything that is not a SOAP 1.1/1.2
/// `Envelope`, so a stray HTML error page never reaches the response parser.
#[instrument(skip_all, err)]
fn soap_body<'a, 'input>(document: &'a Document<'input>) -> Result<Node<'a, 'input>> {
    let envelope = document.root_element();
    let namespace = envelope.tag_name().namespace();
    if envelope.tag_name().name() != "Envelope"
        || (namespace != Some(SOAP_11_NAMESPACE) && namespace != Some(SOAP_12_NAMESPACE))
    {
        return Err(anyhow!("Unexpected SOAP Envelope namespace"));
    }
    exactly_one_child(envelope, "Body", namespace)
}

/// Returns the single child element with `name` in `namespace`. `None` matches by
/// tag name only, for callers that don't yet know the namespace (e.g. extracting
/// it from a rendered template). Both "missing" and "more than one" are errors:
/// an ambiguous `Success` element must never be read as a definitive outcome.
#[instrument(skip(parent), err)]
fn exactly_one_child<'a, 'input>(
    parent: Node<'a, 'input>,
    name: &str,
    namespace: Option<&str>,
) -> Result<Node<'a, 'input>> {
    let mut elements = parent.children().filter(|node| {
        node.is_element()
            && node.tag_name().name() == name
            && (namespace.is_none() || node.tag_name().namespace() == namespace)
    });
    let element = elements.next().ok_or_else(|| match namespace {
        Some(namespace) => anyhow!("Missing XML element {name} in namespace {namespace}"),
        None => anyhow!("Missing XML element {name}"),
    })?;
    if elements.next().is_some() {
        return Err(anyhow!("Multiple XML elements named {name}"));
    }
    Ok(element)
}

/// Trimmed text of the single `name` child; errors when the element is missing,
/// duplicated, or empty.
#[instrument(skip(parent), err)]
fn child_text(parent: Node<'_, '_>, name: &str, namespace: &str) -> Result<String> {
    let element = exactly_one_child(parent, name, Some(namespace))?;
    element
        .text()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Missing or empty XML element {name}"))
}

/// Reads the target namespace of the request's operation root element (e.g.
/// `SetVoted`) off the rendered request body. The hot-swappable S3 template owns
/// this namespace; extracting it here — rather than hardcoding it in Rust — is
/// what keeps the outbound `SOAPAction` header and the expected response
/// namespace in sync with whatever template is actually live, instead of two
/// independent hardcoded values drifting apart.
#[instrument(skip_all, err)]
fn operation_namespace(body: &str, request: SoapRequest) -> Result<String> {
    let document = Document::parse(body).context("Rendered VoterView template is not valid XML")?;
    let body_element = soap_body(&document)?;
    let operation = exactly_one_child(body_element, request.operation_name(), None)?;
    operation
        .tag_name()
        .namespace()
        .map(str::to_string)
        .ok_or_else(|| {
            anyhow!(
                "Rendered VoterView template's {} element has no namespace",
                request.operation_name()
            )
        })
}

/// Collapses whitespace, strips trailing punctuation and lowercases, so a known
/// message ("the voter has already voted") is matched despite formatting drift.
#[instrument]
fn normalize_message(message: &str) -> String {
    message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', '!', '?'])
        .to_lowercase()
}

/// Classifies a VoterView response into a [`SoapRequestResponse`]. A non-success
/// HTTP status becomes a `Fault`; a `Success=false` with a recognized message
/// becomes the idempotent `AlreadyVoted`/`AlreadyNotVoted`, otherwise `Rejected`.
/// Any unparseable body or unexpected `Success` value is an error, never a
/// silent success.
#[instrument(skip(response_text), err)]
fn parse_response(
    status: StatusCode,
    response_text: &str,
    request: SoapRequest,
    namespace: &str,
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
    let response = exactly_one_child(soap_body, &response_name, Some(namespace))?;
    let result = exactly_one_child(response, request.result_name(), Some(namespace))?;
    let success = child_text(result, "Success", namespace)?;
    match success.to_lowercase().as_str() {
        "true" => Ok(SoapRequestResponse::Ok),
        "false" => {
            let message = child_text(result, "ErrorMessage", namespace)?;
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

/// Reads the response body and hands it to [`parse_response`] with its status.
#[instrument(skip(response), err)]
async fn read_response(
    response: Response,
    request: SoapRequest,
    namespace: &str,
) -> Result<SoapRequestResponse> {
    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("Failed to read the VoterView response")?;
    parse_response(status, &response_text, request, namespace)
}

/// Renders, validates and hashes the request without contacting VoterView.
/// Every failure mode here (missing username, invalid annotations, template or
/// XML error) happens before a vote is claimed, so the caller can retry later.
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
    let RenderedSoapRequest {
        body,
        template_sha256,
        operation_namespace,
    } = render_request(request, &annotations, voter_id, &timestamp).await?;

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
        operation_namespace,
    })
}

/// Failure of a dispatched VoterView request, split by whether the request may
/// have reached the server. `NotDispatched` is a connection-level failure that
/// provably never left this host, so the caller can safely retry later;
/// `Ambiguous` may have been received and processed, so a non-idempotent
/// request must not be blindly retried.
#[derive(Debug)]
pub enum SoapSendError {
    NotDispatched(anyhow::Error),
    Ambiguous(anyhow::Error),
}

impl std::fmt::Display for SoapSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotDispatched(err) => write!(f, "not dispatched: {err:#}"),
            Self::Ambiguous(err) => write!(f, "ambiguous outcome: {err:#}"),
        }
    }
}

impl std::error::Error for SoapSendError {}

/// Dispatches an already-prepared request and classifies the response, carrying
/// the template hash into the [`SoapRequestResult`] for the audit trail.
#[instrument(skip(prepared), fields(request = %prepared.request), err)]
pub async fn send_prepared(
    prepared: PreparedSoapRequest,
) -> Result<SoapRequestResult, SoapSendError> {
    let PreparedSoapRequest {
        request,
        client,
        url,
        body,
        template_sha256,
        operation_namespace,
    } = prepared;
    let response = client
        .post(url)
        .header("Content-Type", "text/xml; charset=UTF-8")
        .header(
            "SOAPAction",
            format!("{operation_namespace}/{}", request.operation_name()),
        )
        .body(body)
        .send()
        .await
        .map_err(|err| {
            let not_dispatched = err.is_connect();
            let wrapped = anyhow::Error::new(err).context(format!(
                "VoterView request failed (template_sha256={template_sha256})"
            ));
            if not_dispatched {
                SoapSendError::NotDispatched(wrapped)
            } else {
                SoapSendError::Ambiguous(wrapped)
            }
        })?;
    let response = read_response(response, request, &operation_namespace)
        .await
        .map_err(|err| {
            SoapSendError::Ambiguous(err.context(format!(
                "Invalid VoterView response (template_sha256={template_sha256})"
            )))
        })?;

    Ok(SoapRequestResult {
        response,
        template_sha256,
    })
}

/// Convenience [`prepare`] + [`send_prepared`] for callers that do not need to
/// separate preparation from dispatch across a vote claim.
#[instrument(skip(election_event, username), fields(request = %request), err)]
pub async fn send(
    request: SoapRequest,
    election_event: ElectionEventDatafix,
    username: &Option<String>,
) -> Result<SoapRequestResult> {
    let prepared = prepare(request, election_event, username).await?;
    send_prepared(prepared).await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A connection refused by the local host proves the request never left,
    /// so the send error must classify as `NotDispatched` (safe to retry).
    #[tokio::test]
    async fn connection_refused_is_classified_as_not_dispatched() {
        let prepared = PreparedSoapRequest {
            request: SoapRequest::SetVoted,
            client: reqwest::Client::new(),
            url: "http://127.0.0.1:9/mvv".to_string(),
            body: "<x/>".to_string(),
            template_sha256: "test".to_string(),
            operation_namespace: "test".to_string(),
        };
        match send_prepared(prepared).await {
            Err(SoapSendError::NotDispatched(_)) => {}
            other => panic!("expected NotDispatched, got {other:?}"),
        }
    }

    const TEST_NAMESPACE: &str = "urn:test:voterview";

    fn response(result: &str, prefix: &str, namespace: &str) -> String {
        format!(
            r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"
                xmlns:v="{namespace}">
                <soap:Body><v:{prefix}Response><v:{prefix}Result>{result}</v:{prefix}Result>
                </v:{prefix}Response></soap:Body></soap:Envelope>"#
        )
    }

    #[test]
    fn parses_namespaced_success() {
        let xml = response("<v:Success> true </v:Success>", "SetVoted", TEST_NAMESPACE);
        assert_eq!(
            parse_response(StatusCode::OK, &xml, SoapRequest::SetVoted, TEST_NAMESPACE).unwrap(),
            SoapRequestResponse::Ok
        );
    }

    #[test]
    fn classifies_known_responses_after_normalizing_whitespace() {
        let already_voted = response(
            "<v:Success>false</v:Success><v:ErrorMessage> The voter  has already voted. </v:ErrorMessage>",
            "SetVoted",
            TEST_NAMESPACE,
        );
        assert_eq!(
            parse_response(
                StatusCode::OK,
                &already_voted,
                SoapRequest::SetVoted,
                TEST_NAMESPACE
            )
            .unwrap(),
            SoapRequestResponse::AlreadyVoted
        );

        let already_not_voted = response(
            "<v:Success>false</v:Success><v:ErrorMessage>The voter has not voted.</v:ErrorMessage>",
            "SetNotVoted",
            TEST_NAMESPACE,
        );
        assert_eq!(
            parse_response(
                StatusCode::OK,
                &already_not_voted,
                SoapRequest::SetNotVoted,
                TEST_NAMESPACE
            )
            .unwrap(),
            SoapRequestResponse::AlreadyNotVoted
        );
    }

    #[test]
    fn rejects_malformed_or_ambiguous_success_elements() {
        assert!(parse_response(
            StatusCode::OK,
            "not xml",
            SoapRequest::SetVoted,
            TEST_NAMESPACE
        )
        .is_err());
        let duplicate = response(
            "<v:Success>true</v:Success><v:Success>true</v:Success>",
            "SetVoted",
            TEST_NAMESPACE,
        );
        assert!(parse_response(
            StatusCode::OK,
            &duplicate,
            SoapRequest::SetVoted,
            TEST_NAMESPACE
        )
        .is_err());
    }

    #[test]
    fn rejects_success_from_an_unexpected_namespace() {
        let xml = response("<v:Success>true</v:Success>", "SetVoted", TEST_NAMESPACE);
        assert!(parse_response(
            StatusCode::OK,
            &xml,
            SoapRequest::SetVoted,
            "urn:different:namespace"
        )
        .is_err());
    }

    #[test]
    fn extracts_the_operation_element_namespace_from_the_rendered_body() {
        let body = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
            <soap:Body><SetVoted xmlns="urn:whatever">
            <CountyMun>county</CountyMun></SetVoted></soap:Body></soap:Envelope>"#;
        assert_eq!(
            operation_namespace(body, SoapRequest::SetVoted).unwrap(),
            "urn:whatever"
        );
    }

    #[test]
    fn rejects_a_rendered_body_missing_the_operation_element() {
        let body = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
            <soap:Body><SetNotVoted xmlns="urn:whatever">
            <CountyMun>county</CountyMun></SetNotVoted></soap:Body></soap:Envelope>"#;
        assert!(operation_namespace(body, SoapRequest::SetVoted).is_err());
    }

    #[test]
    fn rejects_a_rendered_operation_element_with_no_namespace() {
        let body = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
            <soap:Body><SetVoted>
            <CountyMun>county</CountyMun></SetVoted></soap:Body></soap:Envelope>"#;
        assert!(operation_namespace(body, SoapRequest::SetVoted).is_err());
    }

    #[test]
    fn rejects_a_rendered_body_with_a_duplicated_operation_element() {
        let body = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
            <soap:Body><SetVoted xmlns="urn:whatever"/><SetVoted xmlns="urn:whatever"/>
            </soap:Body></soap:Envelope>"#;
        assert!(operation_namespace(body, SoapRequest::SetVoted).is_err());
    }

    #[test]
    fn rejects_malformed_xml_when_extracting_operation_namespace() {
        assert!(operation_namespace("not xml", SoapRequest::SetVoted).is_err());
    }

    fn sample_data() -> SoapRequestData<'static> {
        SoapRequestData {
            county_mun: "county",
            usr: "user",
            psw: "password",
            voter_id: "voter",
            timestamp: "timestamp",
        }
    }

    #[test]
    fn accepts_a_restructured_template_when_injected_values_survive() {
        // Renamed elements, a different channel and an extra field are all
        // template-owned, so a hot-swapped template must still validate as long
        // as the injected values are present.
        let xml = r#"<Envelope><Body><DoIt xmlns="urn:voterview:v2">
            <Muni>county</Muni><Login>user</Login><Secret>password</Secret>
            <Voter>voter</Voter><Channel>PHONE</Channel><Extra>new</Extra>
            <When>timestamp</When></DoIt></Body></Envelope>"#;
        assert!(validate_rendered_xml(xml, &sample_data()).is_ok());
    }

    #[test]
    fn rejects_a_template_that_drops_an_injected_value() {
        // The voter id never made it into the output (e.g. a mistyped variable).
        let xml = r#"<Envelope><Body><DoIt>
            <Muni>county</Muni><Login>user</Login><Secret>password</Secret>
            <When>timestamp</When></DoIt></Body></Envelope>"#;
        assert!(validate_rendered_xml(xml, &sample_data()).is_err());
    }

    #[test]
    fn rejects_an_injected_value_rendered_as_markup_instead_of_text() {
        // A non-escaping template emitted a value containing markup raw: it
        // parses, but the voter id is now an element, not the text we injected.
        let data = SoapRequestData {
            voter_id: "<x/>",
            ..sample_data()
        };
        let xml = r#"<Envelope><Body><DoIt>
            <Muni>county</Muni><Login>user</Login><Secret>password</Secret>
            <Voter><x/></Voter><When>timestamp</When></DoIt></Body></Envelope>"#;
        assert!(validate_rendered_xml(xml, &data).is_err());
    }

    #[test]
    fn rejects_malformed_rendered_xml() {
        assert!(validate_rendered_xml("<Body><unclosed>", &sample_data()).is_err());
    }

    #[test]
    fn parses_namespaced_soap_fault() {
        let xml = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
            <s:Body><s:Fault><s:Reason><s:Text>Service unavailable</s:Text></s:Reason>
            </s:Fault></s:Body></s:Envelope>"#;
        assert_eq!(
            parse_response(
                StatusCode::SERVICE_UNAVAILABLE,
                xml,
                SoapRequest::SetVoted,
                TEST_NAMESPACE
            )
            .unwrap(),
            SoapRequestResponse::Fault("Service unavailable".to_string())
        );
    }
}
