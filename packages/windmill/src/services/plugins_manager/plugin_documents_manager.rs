// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use core::convert::From;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str,
    services::{
        reports,
        s3::{download_s3_file_to_string, get_public_asset_file_path},
    },
    types::hasura::core::Document,
};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use strand::hash::hash_sha256;
use tempfile::NamedTempFile;

use sequent_core::plugins_wit::lib::documents_bindings::plugins_manager::documents_manager::documents::Host;
use crate::services::{ceremonies::velvet_tally::generate_initial_state, documents::get_document_as_temp_file_at_dir, folders::list_files};

// A struct to hold the host's state, including a map to manage the temporary files.
pub struct PluginDocumentsManager {
    open_temp_files: Vec<NamedTempFile>,
    path_dir: PathBuf,
}

impl PluginDocumentsManager {
    pub fn new(path_dir: PathBuf) -> Self {
        Self {
            open_temp_files: Vec::new(),
            path_dir,
        }
    }
}

impl Host for PluginDocumentsManager {
    // Implement the method to create a document as a temporary file.
    async fn create_document_as_temp_file(
        &mut self,
        tenant_id: String,
        document_json: String,
    ) -> Result<String, String> {
        let document: Document = deserialize_str::<Document>(&document_json)
            .map_err(|e| format!("Failed to parse document JSON: {}", e))?;

        let named_temp_file =
            get_document_as_temp_file_at_dir(&tenant_id, &document, &self.path_dir)
                .await
                .map_err(|e| format!("Failed to get document as temp file: {}", e))?;
        println!(
            "Temporary file created at: {}",
            named_temp_file.path().display()
        );

        let file_name = named_temp_file
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        self.open_temp_files.push(named_temp_file);

        Ok(file_name)
    }

    async fn print_data(&mut self, data: String) {
        println!("Data to print: {}", data);
    }

    async fn get_tally_results(&mut self, tally_base_path: String) -> Result<String, String> {
        println!("Getting tally results from: {}", tally_base_path);

        let file = &self.path_dir;
        let path = PathBuf::from(file).join(tally_base_path);

        list_files(path.as_path()).map_err(|e| format!("Error listing files: {:?}", e))?;

        let state = generate_initial_state(&path, "decode-ballots")
            .map_err(|e| format!("Error in generate_initial_state: {:?}", e))?;

        let results = state
            .get_results(true)
            .map_err(|e| format!("Error in get results from velvet state: {:?}", e))?;

        let tally_res = serde_json::to_string(&results)
            .map_err(|e| format!("Failed to serialize results event: {}", e))?;
        // println!("Host Tally results: {}", tally_res);

        Ok(tally_res)
    }

    async fn get_s3_public_asset_file_path(&mut self, filename: String) -> Result<String, String> {
        let path = get_public_asset_file_path(&filename)
            .map_err(|e| format!("Error getting S3 public asset file path: {}", e))?;
        Ok(path)
    }

    async fn download_s3_file_to_string(&mut self, file_url: String) -> Result<String, String> {
        let file_str = download_s3_file_to_string(&file_url)
            .await
            .map_err(|e| format!("Error downloading S3 file: {}", e))?;
        Ok(file_str)
    }

    async fn render_template_text(
        &mut self,
        template_string: String,
        variables_map: String,
    ) -> Result<String, String> {
        let variables_map: Map<String, Value> =
            deserialize_str::<Map<String, Value>>(&variables_map)
                .map_err(|e| format!("Error parsing variables map: {}", e))?;

        let rendered_text = reports::render_template_text(&template_string, variables_map)
            .map_err(|e| format!("Error rendering template text: {}", e))?;
        Ok(rendered_text)
    }

    async fn hash_sha256(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        hash_sha256(&bytes).map_err(|e| format!("Error hashing bytes: {}", e))
    }
}
