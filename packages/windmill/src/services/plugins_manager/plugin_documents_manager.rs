// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use core::convert::From;
use sequent_core::types::hasura::core::Document;
use std::path::{Path, PathBuf};
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
        //TODO: deserialize_str
        let document: Document = serde_json::from_str(&document_json)
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
}
