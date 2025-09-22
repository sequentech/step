// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::config::ConfigData;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::util::mime::get_mime_types;
use std::collections::HashMap;
use std::fs::{metadata, File};
use std::io::Read;
use std::path::Path;

use super::read_config::read_config;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_upload_url.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]

pub struct GetUploadUrl;

impl GetUploadUrl {
    pub fn upload(file_path: String, is_local: bool) -> Result<String, Box<dyn std::error::Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::new();

        let path = Path::new(&file_path);
        let file_name = path
            .file_name()
            .ok_or("Invalid file path")?
            .to_str()
            .ok_or("Invalid file name")?;
        let file_size = metadata(&path)?.len() as i64;

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or("Unable to determine file extension")?;
        let mime_type = get_mime_types(extension)[0];

        let variables = get_upload_url::Variables {
            name: String::from(file_name),
            media_type: String::from(mime_type),
            size: file_size,
            is_public: false, // If local then the url changes
            is_local: Some(is_local),
            election_event_id: None,
        };

        let request_body = GetUploadUrl::build_query(variables);

        let response = client
            .post(&config.endpoint_url)
            .bearer_auth(config.auth_token)
            .json(&request_body)
            .send()?;

        if response.status().is_success() {
            let response_body: Response<get_upload_url::ResponseData> = response.json()?;
            if let Some(data) = response_body.data {
                if let Some(e) = data.get_upload_url {
                    let upload_url = e.url.clone();
                    let mut file = File::open(&file_path)?;
                    let mut file_contents = Vec::new();
                    file.read_to_end(&mut file_contents)?;
                    let upload_response = match mime_type {
                        "application/json" | "text/csv" => {
                            let file_contents_str = String::from_utf8(file_contents)?;
                            client
                                .put(&upload_url)
                                .header("Content-Type", mime_type)
                                .body(file_contents_str)
                                .send()?
                        }
                        _ => client
                            .put(&upload_url)
                            .header("Content-Type", mime_type)
                            .body(file_contents)
                            .send()?,
                    };

                    if upload_response.status().is_success() {
                        Ok(e.document_id.clone())
                    } else {
                        let status = upload_response.status();
                        let error_message = upload_response.text()?;
                        let error =
                            format!("HTTP Status: {}\nError Message: {}", status, error_message);
                        Err(Box::from(error))
                    }
                } else {
                    Err(Box::from("failed uploading document"))
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
}
