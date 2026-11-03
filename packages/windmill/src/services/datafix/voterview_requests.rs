// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::types::*;
use crate::postgres::election_event::ElectionEventDatafix;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use anyhow::{anyhow, Result};
use reqwest::{redirect, Response, Url};
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::util::date_time::generate_timestamp;
use std::time::Duration;
use tracing::{info, instrument};

pub const VOTERVIEW_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_VOTERVIEW_RESPONSE_BYTES: usize = 1024 * 1024;

const HTTPS_SCHEME: &str = "https";
const HTTP_SCHEME: &str = "http";

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

/// Returns the configured VoterView URL once its scheme is one the event's
/// [`VoterviewTransportPolicy`] accepts.
fn voterview_url(voterview_request: &VoterviewRequest) -> Result<String> {
    let url = Url::parse(&voterview_request.url)
        .map_err(|err| anyhow!("Invalid VoterView URL: {err}"))?;
    let policy = voterview_request.transport_policy;
    let accepted = match policy {
        VoterviewTransportPolicy::HttpsOnly => url.scheme() == HTTPS_SCHEME,
        VoterviewTransportPolicy::AllowPlaintext => {
            matches!(url.scheme(), HTTPS_SCHEME | HTTP_SCHEME)
        }
    };
    if !accepted {
        return Err(anyhow!(
            "VoterView URL scheme {} is not accepted by transport policy {policy}",
            url.scheme()
        ));
    }
    Ok(voterview_request.url.clone())
}

fn voterview_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(VOTERVIEW_REQUEST_TIMEOUT)
        .redirect(redirect::Policy::none())
        .build()
        .map_err(|err| anyhow!("Failed to build the VoterView HTTP client: {err}"))
}

/// Decodes a response body with the charset its `Content-Type` declares, as
/// `Response::text()` does, falling back to UTF-8.
fn decode_body(content_type: Option<&str>, body: &[u8]) -> String {
    let encoding = content_type
        .and_then(|value| {
            value.split(';').skip(1).find_map(|parameter| {
                let (name, label) = parameter.split_once('=')?;
                name.trim()
                    .eq_ignore_ascii_case("charset")
                    .then(|| label.trim().trim_matches('"'))
            })
        })
        .and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    encoding.decode(body).0.into_owned()
}

/// Reads the response body, up to [`MAX_VOTERVIEW_RESPONSE_BYTES`].
async fn read_limited_text(mut response: Response) -> Result<String> {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let too_large = || anyhow!("VoterView response exceeds {MAX_VOTERVIEW_RESPONSE_BYTES} bytes");
    if response
        .content_length()
        .is_some_and(|length| length > MAX_VOTERVIEW_RESPONSE_BYTES as u64)
    {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| anyhow!("Failed to get the full response text: {err}"))?
    {
        if body.len() + chunk.len() > MAX_VOTERVIEW_RESPONSE_BYTES {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(decode_body(content_type.as_deref(), &body))
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
    let url = voterview_url(&annotations.voterview_request)?;
    info!("Soap body: {soap_body}");
    info!("URL: {url}");

    let http = voterview_client()?;
    let response = http
        .post(&url)
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
    let response_txt = read_limited_text(response).await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_declared_charset() {
        let (latin1, _, _) = encoding_rs::WINDOWS_1252.encode("<a>caf\u{e9}</a>");
        assert_eq!(
            decode_body(Some("text/xml; charset=\"ISO-8859-1\""), &latin1),
            "<a>caf\u{e9}</a>"
        );
        let utf16: Vec<u8> = "<a>caf\u{e9}</a>"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(
            decode_body(Some("text/xml; charset=utf-16le"), &utf16),
            "<a>caf\u{e9}</a>"
        );
        assert_eq!(
            decode_body(None, "<a>caf\u{e9}</a>".as_bytes()),
            "<a>caf\u{e9}</a>"
        );
        assert_eq!(
            decode_body(Some("text/xml; charset=unknown"), b"<a/>"),
            "<a/>"
        );
    }
    use serde_json::Value;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    const TEST_NAMESPACE_BODY: &str = "<Success>true</Success>";

    async fn read_http_request(stream: &mut TcpStream) -> std::io::Result<()> {
        let mut request = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let read = stream.read(&mut buffer).await?;
            if read == 0 {
                return Ok(());
            }
            request.extend_from_slice(&buffer[..read]);
            let Some(headers_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..headers_end]).to_lowercase();
            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= headers_end + 4 + content_length {
                return Ok(());
            }
        }
    }

    async fn serve_once(
        listener: TcpListener,
        response: Vec<u8>,
        received: Arc<AtomicBool>,
    ) -> std::io::Result<()> {
        let (mut stream, _) = listener.accept().await?;
        received.store(true, Ordering::SeqCst);
        read_http_request(&mut stream).await?;
        stream.write_all(&response).await?;
        stream.shutdown().await
    }

    async fn spawn_server(
        response: Vec<u8>,
    ) -> (
        String,
        Arc<AtomicBool>,
        tokio::task::JoinHandle<std::io::Result<()>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/mvv", listener.local_addr().unwrap());
        let received = Arc::new(AtomicBool::new(false));
        let server = tokio::spawn(serve_once(listener, response, received.clone()));
        (url, received, server)
    }

    fn http_response(status: &str, headers: &str, body: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/xml; charset=utf-8\r\n{headers}Connection: close\r\n\r\n{body}"
        )
        .into_bytes()
    }

    fn success_body() -> String {
        TEST_NAMESPACE_BODY.to_string()
    }

    async fn post_to(url: &str) -> Result<String> {
        let response = voterview_client()?
            .post(url)
            .body("<x/>")
            .send()
            .await
            .map_err(|err| anyhow!("send: {err}"))?;
        if response.status().is_redirection() {
            return Err(anyhow!("redirect not followed"));
        }
        read_limited_text(response).await
    }

    fn voterview_request(url: &str, transport_policy: Option<&str>) -> VoterviewRequest {
        let mut value = serde_json::json!({
            "url": url,
            "usr": "user",
            "psw": "password",
            "county_mun": "county",
        });
        if let Some(policy) = transport_policy {
            value["transport_policy"] = Value::String(policy.to_string());
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn voterview_url_accepts_https_by_default() {
        let url = "https://voterview.example/mvv";
        assert_eq!(voterview_url(&voterview_request(url, None)).unwrap(), url);
    }

    #[test]
    fn voterview_url_requires_https_by_default() {
        let request = voterview_request("http://voterview.example/mvv", None);
        assert!(voterview_url(&request).is_err());
    }

    #[test]
    fn voterview_url_accepts_http_when_plaintext_is_allowed() {
        let url = "http://voterview.example/mvv";
        let request = voterview_request(url, Some("allow-plaintext"));
        assert_eq!(voterview_url(&request).unwrap(), url);
    }

    #[test]
    fn voterview_url_rejects_other_schemes_and_invalid_urls() {
        for url in [
            "file:///etc/hosts",
            "ftp://voterview.example/mvv",
            "not a url",
        ] {
            for policy in [None, Some("https-only"), Some("allow-plaintext")] {
                assert!(
                    voterview_url(&voterview_request(url, policy)).is_err(),
                    "{url} accepted with {policy:?}"
                );
            }
        }
    }

    #[tokio::test]
    async fn reads_a_response_within_the_limit() {
        let body = success_body();
        let (url, _, server) = spawn_server(http_response(
            "200 OK",
            &format!("Content-Length: {}\r\n", body.len()),
            &body,
        ))
        .await;
        assert_eq!(post_to(&url).await.unwrap(), body);
        server.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn does_not_follow_redirects() {
        let body = success_body();
        let (target_url, target_received, target) = spawn_server(http_response(
            "200 OK",
            &format!("Content-Length: {}\r\n", body.len()),
            &body,
        ))
        .await;
        let (url, _, server) = spawn_server(http_response(
            "307 Temporary Redirect",
            &format!("Location: {target_url}\r\nContent-Length: 0\r\n"),
            "",
        ))
        .await;
        let result = post_to(&url).await;
        target.abort();
        let _ = server.await;
        assert!(result.is_err(), "expected an error, got {result:?}");
        assert!(!target_received.load(Ordering::SeqCst));
    }

    fn oversized_success_body() -> String {
        let body = success_body();
        let padding = MAX_VOTERVIEW_RESPONSE_BYTES + 1 - body.len();
        format!("{body}{}", " ".repeat(padding))
    }

    #[tokio::test]
    async fn rejects_a_declared_response_length_over_the_limit() {
        let body = oversized_success_body();
        let (url, _, server) = spawn_server(http_response(
            "200 OK",
            &format!("Content-Length: {}\r\n", body.len()),
            &body,
        ))
        .await;
        let result = post_to(&url).await;
        let _ = server.await;
        assert!(result.is_err(), "expected an error, got {result:?}");
    }

    #[tokio::test]
    async fn rejects_a_streamed_response_over_the_limit() {
        let body = oversized_success_body();
        let chunked = format!("{:x}\r\n{body}\r\n0\r\n\r\n", body.len());
        let (url, _, server) = spawn_server(http_response(
            "200 OK",
            "Transfer-Encoding: chunked\r\n",
            &chunked,
        ))
        .await;
        let result = post_to(&url).await;
        let _ = server.await;
        assert!(result.is_err(), "expected an error, got {result:?}");
    }
}
