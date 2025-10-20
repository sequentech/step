// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use core::convert::From;
use std::path::PathBuf;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str,
};
use reqwest::multipart;
use sequent_core::plugins_wit::lib::client_request_bindings::plugins_manager::client_request_manager::client_request::Host;
use crate::services::{plugins_manager::plugin::PluginServices};
pub struct PluginClientRequestManager;

impl Host for PluginServices {
    async fn send_zip(&mut self, zip_path: String, uri: String) -> Result<(), String> {
        let base_path = &self.documents.path_dir;
        let new_zip_path = PathBuf::from(base_path).join(zip_path);
        let zip_bytes = std::fs::read(new_zip_path).map_err(|err| format!("{:?}", err))?;

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|err| format!("{:?}", err))?;

        // Create a multipart form
        let form = multipart::Form::new().part(
            "zip",
            multipart::Part::bytes(zip_bytes)
                .file_name("file.zip")
                .mime_str("application/zip")
                .map_err(|err| format!("{:?}", err))?,
        );

        // Send the POST request
        let response = client
            .post(&uri)
            .multipart(form)
            .send()
            .await
            .map_err(|err| format!("{:?}", err))?;
        let response_str = format!("{:?}", response);
        println!(
            "Response code: {}. Response: '{}'",
            response.status(),
            response_str
        );
        let is_success = response.status().is_success();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed get response text: {}", e))?;

        // Check if the request was successful
        if !is_success {
            return Err(format!(
                "Failed to send package. Text: {}. Response: {}",
                text, response_str
            ));
        }
        Ok(())
    }
}
