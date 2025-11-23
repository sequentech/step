// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use core::convert::From;
use std::path::PathBuf;
use openssl::{pkcs12::Pkcs12};
use sequent_core::signatures::shell::run_shell_command;
use reqwest::multipart;
use sequent_core::plugins_wit::lib::extra_services_bindings::plugins_manager::extra_services_manager::{
    cli_service::{Host as CLIServiceHost},
    request_service::{Host as RequestServiceHost}
};
use crate::services::{
    consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc,
    plugins_manager::plugin::PluginServices,
};
use std::fs;
use std::io::Read;

impl RequestServiceHost for PluginServices {
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

impl CLIServiceHost for PluginServices {
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
    async fn run_shell_command_derive_public_key_from_p12(
        &mut self,
        java_jar_file: String,
        pk12_file_name: String,
        password: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;
        let pk12_file_path = PathBuf::from(base_path).join(&pk12_file_name);

        let command = format!(
            "java -jar {} public-key {} {}",
            java_jar_file,
            pk12_file_path.to_string_lossy().to_string(),
            password
        );

        let result = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        Ok(result)
    }

    async fn run_shell_command_get_p12_cert(
        &mut self,
        p12_file_path: String,
        password: String,
        cert_temp_path: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;

        let p12_file_path = PathBuf::from(base_path).join(&p12_file_path);
        let p12_file_path_str = p12_file_path.to_string_lossy().to_string();
        let cert_temp_path = PathBuf::from(base_path).join(&cert_temp_path);
        let cert_temp_path_str = cert_temp_path.to_string_lossy().to_string();

        let command = format!(
            "openssl pkcs12 -in {} -passin pass:{} -nokeys -out {}",
            p12_file_path_str, password, cert_temp_path_str
        );

        let result = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        Ok(result)
    }

    async fn run_shell_command_get_p12_fingerprint(
        &mut self,
        cert_temp_path: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;
        let cert_temp_path = PathBuf::from(base_path).join(&cert_temp_path);

        let command = format!(
            "openssl x509 -in {} -noout -fingerprint -sha256",
            cert_temp_path.to_string_lossy().to_string()
        );

        let result = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        Ok(result)
    }

    async fn run_shell_command_check_certificate_cas(
        &mut self,
        root_ca_file_path: String,
        intermediate_ca_file_path: String,
        p12_cert_path: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;
        let root_ca_path = PathBuf::from(base_path).join(&root_ca_file_path);
        let intermediate_ca_path = PathBuf::from(base_path).join(&intermediate_ca_file_path);
        let p12_cert_path = PathBuf::from(base_path).join(&p12_cert_path);

        let command = format!(
            "openssl verify -CAfile {} -untrusted {} {}",
            root_ca_path.to_string_lossy().to_string(),
            intermediate_ca_path.to_string_lossy().to_string(),
            p12_cert_path.to_string_lossy().to_string(),
        );

        let result = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        Ok(result)
    }

    async fn get_pk12_id(&mut self, p12_path: String, password: String) -> Result<i32, String> {
        let base_dir = &self.documents.path_dir;
        let full_p12_path = PathBuf::from(base_dir).join(p12_path);

        let mut file = fs::File::open(full_p12_path)
            .map_err(|e| format!("Failed to open .p12 file: {}", e))?;
        let mut p12_data = Vec::new();
        file.read_to_end(&mut p12_data)
            .map_err(|e| format!("Failed to read .p12 file: {}", e))?;

        // Parse the .p12 file
        let pkcs12 =
            Pkcs12::from_der(&p12_data).map_err(|e| format!("Failed to parse .p12 file: {}", e))?;
        let parsed = pkcs12
            .parse2(&password)
            .map_err(|e| format!("Failed to parse .p12 file: {}", e))?;
        let pkey = parsed.pkey.ok_or(format!("Can't find pkey"))?;
        let id_i32 = openssl::pkey::Id::as_raw(&pkey.id());
        Ok(id_i32)
    }

    async fn run_shell_command_rsa_sign_data(
        &mut self,
        java_jar_file: String,
        pk12_file_path: String,
        data_path: String,
        password: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;
        let pk12_path = PathBuf::from(base_path).join(&pk12_file_path);
        let data_file_path = PathBuf::from(base_path).join(&data_path);

        let command = format!(
            "java -jar {} sign-rsa {} {} {}",
            java_jar_file,
            pk12_path.to_string_lossy().to_string(),
            data_file_path.to_string_lossy().to_string(),
            password
        );

        let encrypted_base64 = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        println!("rsa_sign_data: '{}'", encrypted_base64);

        Ok(encrypted_base64)
    }

    async fn run_shell_command_ecdsa_sign_data(
        &mut self,
        java_jar_file: String,
        pk12_file_path: String,
        data_path: String,
        password: String,
    ) -> Result<String, String> {
        let base_path = &self.documents.path_dir;
        let pk12_path = PathBuf::from(base_path).join(&pk12_file_path);
        let data_file_path = PathBuf::from(base_path).join(&data_path);

        let command = format!(
            "java -jar {} sign-ec {} {} {}",
            java_jar_file,
            pk12_path.to_string_lossy().to_string(),
            data_file_path.to_string_lossy().to_string(),
            password
        );

        let encrypted_base64 = run_shell_command(&command)
            .map_err(|err| format!("Failed run shell command"))?
            .replace("\n", "");

        println!("ecdsa_sign_data: '{}'", encrypted_base64);

        Ok(encrypted_base64)
    }

    async fn create_server_signature(
        &mut self,
        java_jar_file: String,
        pk12_file_path: String,
        data_path: String,
        password: String,
    ) -> Result<String, String> {
        let pk12_id_raw = self
            .get_pk12_id(pk12_file_path.clone(), password.clone())
            .await?;
        let pk12_id = openssl::pkey::Id::from_raw(pk12_id_raw as i32);

        let signature = match pk12_id {
            openssl::pkey::Id::RSA => {
                self.run_shell_command_rsa_sign_data(
                    java_jar_file.clone(),
                    pk12_file_path.clone(),
                    data_path.clone(),
                    password.clone(),
                )
                .await?
            }
            openssl::pkey::Id::EC => {
                self.run_shell_command_ecdsa_sign_data(
                    java_jar_file,
                    pk12_file_path,
                    data_path,
                    password,
                )
                .await?
            }
            _ => {
                return Err(format!("Unexpected p12 key {:?}", pk12_id));
            }
        };
        Ok(signature)
    }
}
