// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::{read_config::read_config, upload_file::GetUploadUrl};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::types::permissions::Permissions;

#[derive(Args)]
#[command(about = "Generate Preview Url", long_about = None)]
pub struct GeneratePreview {
    /// Path of Preview file - .json file
    #[arg(long)]
    file_path: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/generate_preview_url.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GeneratePreviewUrl;

impl GeneratePreview {
    pub fn run(&self) {
        match generate_preview(&self.file_path) {
            Ok(url) => {
                println!("Success! generated preview url: {}", url);
            }
            Err(err) => {
                eprintln!("Error! Failed to generated preview url: {}", err)
            }
        }
    }
}

pub fn generate_preview(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();
    let document_id = GetUploadUrl::upload(String::from(file_path), true)?;

    let variables = generate_preview_url::Variables {
        tenant_id: config.tenant_id.clone(),
        document_id,
    };

    let request_body = GeneratePreviewUrl::build_query(variables);
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .header("x-hasura-role", Permissions::GENERATE_PREVIEW.to_string())
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<generate_preview_url::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            if let Some(ref p) = data.generate_preview_url {
                Ok(p.preview_url.clone())
            } else {
                Err(Box::from("failed generating url"))
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
