// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{read_config::read_config, trustees::held_role_request},
};
use graphql_client::{GraphQLQuery, Response};
use reqwest::blocking::Client;
use sequent_core::types::permissions::Permissions;
use std::path::Path;
use std::{fs, io};
use url::Url;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/fetch_document.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct FetchDocument;

pub struct FetchDocumentOutput {
    pub url: String,
}

/// Like `fetch_document`, but for a document not tied to any election event
/// (e.g. a tenant config export) — `election_event_id` is optional in the
/// underlying query.
pub fn fetch_document_url(
    document_id: &str,
) -> Result<FetchDocumentOutput, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = fetch_document::Variables {
        election_event_id: None,
        document_id: document_id.to_string(),
    };

    let request_body = FetchDocument::build_query(variables);

    let response = held_role_request(
        &client,
        &config.endpoint_url,
        &config.auth_token,
        Permissions::DOCUMENT_DOWNLOAD,
    )
    .json(&request_body)
    .send()?;

    if response.status().is_success() {
        let response_body: Response<fetch_document::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(r) = data.fetch_document {
                Ok(FetchDocumentOutput { url: r.url.clone() })
            } else {
                Err(Box::from("No document URL found"))
            }
        } else if let Some(errors) = response_body.errors {
            let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            Err(Box::from(error_messages.join(", ")))
        } else {
            Err(Box::from("Unknown error occurred"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

/// Downloads a presigned URL as-is, with no hostname rewriting — unlike
/// `download_file`, which hardcodes rewriting the host to the devcontainer's
/// internal `minio` service name. Use this when step-cli isn't necessarily
/// running inside the devcontainer (e.g. against a remote deployment), where
/// the presigned URL's own host is already the one to connect to.
pub fn download_file_plain(
    presigned_url: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::default())
        .build()?;

    let output_dir = Path::new(output_path).parent().unwrap_or(Path::new("."));
    fs::create_dir_all(output_dir)?;

    let mut response = client.get(presigned_url).send()?;
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text()?;
        return Err(format!("HTTP Status: {}\nError Message: {}", status, error_text).into());
    }

    let mut file = fs::File::create(output_path)?;
    response.copy_to(&mut file)?;

    Ok(())
}

pub fn fetch_document(
    election_event_id: &str,
    document_id: &str,
) -> Result<FetchDocumentOutput, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = fetch_document::Variables {
        election_event_id: Some(election_event_id.to_string()),
        document_id: document_id.to_string(),
    };

    let request_body = FetchDocument::build_query(variables);

    let response = held_role_request(
        &client,
        &config.endpoint_url,
        &config.auth_token,
        Permissions::DOCUMENT_DOWNLOAD,
    )
    .json(&request_body)
    .send()?;

    if response.status().is_success() {
        let response_body: Response<fetch_document::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(r) = data.fetch_document {
                Ok(FetchDocumentOutput { url: r.url.clone() })
            } else {
                Err(Box::from("No document URL found"))
            }
        } else if let Some(errors) = response_body.errors {
            let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            Err(Box::from(error_messages.join(", ")))
        } else {
            Err(Box::from("Unknown error occurred"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

pub fn download_file(
    presigned_url: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the original URL (with 127.0.0.1)
    let original = Url::parse(presigned_url)?;

    // Only the devcontainer signs URLs for a host the CLI can't reach (127.0.0.1,
    // localhost, the minio proxy). A remote deployment signs real S3/CDN URLs:
    // rewriting those to `minio` breaks `export-election-event`, `step-cli load
    // prepare` and tally downloads against it.
    if !is_devcontainer_storage_host(&original) {
        return download_file_plain(presigned_url, output_path);
    }

    // Build the URL we will actually CONNECT to (minio:9000),
    // but keep the same path + query.
    let mut connect_url = original.clone();
    connect_url.set_host(Some("minio"))?;
    // keep the same port – if None, leave it
    if let Some(port) = original.port() {
        connect_url
            .set_port(Some(port))
            .map_err(|_| "failed setting port")?;
    }

    // Build the Host header exactly as it was originally signed
    let host_header = match original.port() {
        Some(port) => format!("{}:{}", original.host_str().unwrap(), port),
        None => original.host_str().unwrap().to_string(),
    };

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::default())
        .build()?;

    let output_dir = Path::new(output_path).parent().unwrap_or(Path::new("."));
    fs::create_dir_all(output_dir)?;

    let mut response = client.get(connect_url).header("Host", host_header).send()?;

    println!("response status: {}", response.status());
    if !response.status().is_success() {
        let error_text = response.text()?;
        return Err(format!("Failed to download file: {}", error_text).into());
    }

    let mut file = fs::File::create(output_path)?;
    response.copy_to(&mut file)?;

    Ok(())
}

/// Whether a presigned URL points at the devcontainer's storage, which step-cli
/// reaches as `minio` (with the signed Host header kept).
fn is_devcontainer_storage_host(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("127.0.0.1" | "localhost" | "minio" | "minio-proxy")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devcontainer_hosts_are_rewritten() {
        for url in [
            "http://127.0.0.1:9000/bucket/key?X-Amz-Signature=x",
            "http://localhost:9000/bucket/key",
            "http://minio:9000/bucket/key",
            "http://minio-proxy:9002/bucket/key",
        ] {
            assert!(is_devcontainer_storage_host(&Url::parse(url).unwrap()), "{url}");
        }
    }

    #[test]
    fn remote_storage_is_downloaded_as_signed() {
        for url in [
            "https://sequent-test-bucket-eu-west-1-581718213778.s3.eu-west-1.amazonaws.com/election-event-documents/doc?X-Amz-Signature=x",
            "https://storage.googleapis.com/bucket/key",
        ] {
            assert!(!is_devcontainer_storage_host(&Url::parse(url).unwrap()), "{url}");
        }
    }
}
