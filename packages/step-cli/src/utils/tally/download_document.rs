// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::read_config::read_config;
use graphql_client::{GraphQLQuery, Response};
use reqwest::blocking::Client;
use std::fs;
use std::path::Path;
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

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
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
