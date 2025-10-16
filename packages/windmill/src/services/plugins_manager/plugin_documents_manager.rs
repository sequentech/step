// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use core::convert::From;
use deadpool_postgres::Transaction;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str,
    services::{
        reports,
        s3::{download_s3_file_to_string, get_public_asset_file_path},
    },
    signatures::shell::run_shell_command,
    types::hasura::core::Document,
};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use strand::hash::hash_sha256;
use tempfile::NamedTempFile;

use sequent_core::plugins_wit::lib::documents_bindings::plugins_manager::documents_manager::documents::Host;
use crate::services::{
    ceremonies::velvet_tally::generate_initial_state, consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc, documents::{get_document_as_temp_file, get_document_as_temp_file_at_dir, upload_and_return_document}, folders::list_files, plugins_manager::plugin::PluginServices
};

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

impl Host for PluginServices {
    // Implement the method to create a document as a temporary file.
    async fn create_document_as_temp_file(
        &mut self,
        tenant_id: String,
        document_json: String,
    ) -> Result<String, String> {
        let document: Document = deserialize_str::<Document>(&document_json)
            .map_err(|e| format!("Failed to parse document JSON: {}", e))?;

        let named_temp_file =
            get_document_as_temp_file_at_dir(&tenant_id, &document, &self.documents.path_dir)
                .await
                .map_err(|e| format!("Failed to get document as temp file: {}", e))?;

        let file_name = named_temp_file
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        self.documents.open_temp_files.push(named_temp_file);

        Ok(file_name)
    }

    async fn get_tally_results(&mut self, tally_base_path: String) -> Result<String, String> {
        println!("Getting tally results from: {}", tally_base_path);

        let base_path = &self.documents.path_dir;
        let path = PathBuf::from(base_path).join(tally_base_path);

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

    async fn upload_and_return_document(
        &mut self,
        file_size: u64,
        media_type: String,
        tenant_id: String,
        election_event_id: Option<String>,
        name: String,
        document_id: Option<String>,
        is_public: bool,
    ) -> Result<String, String> {
        let mut manager = self.transactions.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;

        println!("Uploading document with name: {}", &name);

        let base_path: &PathBuf = &self.documents.path_dir;
        let path: PathBuf = PathBuf::from(base_path).join(&name);
        let file_path = path.as_path().to_string_lossy();

        println!("Uploading document from path: {}", &file_path);

        let document = upload_and_return_document(
            hasura_transaction,
            &file_path,
            file_size,
            &media_type,
            &tenant_id,
            election_event_id,
            &name,
            document_id,
            is_public,
        )
        .await
        .map_err(|e| format!("Failed to upload and return document: {}", e))?;

        let document_json = serde_json::to_string(&document)
            .map_err(|e| format!("Failed to serialize document: {}", e))?;

        Ok(document_json)
    }

    async fn run_shell_command_generate_ecies_key_pair(
        &mut self,
        java_jar_file: String,
        temp_public_pem_file_name: String,
        temp_private_pem_file_name: String,
    ) -> Result<(), String> {
        let base_path = &self.documents.path_dir;
        let public_pem_path = PathBuf::from(base_path).join(&temp_public_pem_file_name);
        let private_pem_path = PathBuf::from(base_path).join(&temp_private_pem_file_name);

        let command = format!(
            "java -jar {} create-keys {} {}",
            java_jar_file,
            public_pem_path.to_string_lossy().to_string(),
            private_pem_path.to_string_lossy().to_string(),
        );

        run_shell_command(&command);
        Ok(())
    }
    async fn encrypt_file(
        &mut self,
        input_file_name: String,
        output_file_name: String,
        password: String,
    ) -> Result<(), String> {
        let base_path = &self.documents.path_dir;
        let input_path = PathBuf::from(base_path)
            .join(&input_file_name)
            .to_string_lossy()
            .to_string();
        let output_path = PathBuf::from(base_path)
            .join(&output_file_name)
            .to_string_lossy()
            .to_string();

        encrypt_file_aes_256_cbc(&input_path, &output_path, &password)
            .map_err(|e| format!("Failed encrypt file: {}", e))?;

        Ok(())
    }

    async fn run_shell_command_ecies_encrypt_string(
        &mut self,
        java_jar_file: String,
        temp_pem_file_name: String,
        password: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;
        let pem_path = PathBuf::from(base_path).join(&temp_pem_file_name);

        let command = format!(
            "java -jar {} encrypt {} {}",
            java_jar_file,
            pem_path.to_string_lossy().to_string(),
            password
        );

        let result = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        Ok(result)
    }

    async fn run_shell_command_ecies_sign_data(
        &mut self,
        java_jar_file: String,
        temp_pem_file_name: String,
        temp_data_file_name: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;
        let pem_path = PathBuf::from(base_path).join(&temp_pem_file_name);
        let data_path = PathBuf::from(base_path).join(&temp_data_file_name);

        let command = format!(
            "java -jar {} sign {} {}",
            java_jar_file,
            pem_path.to_string_lossy().to_string(),
            data_path.to_string_lossy().to_string()
        );

        let result = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        Ok(result)
    }
}
