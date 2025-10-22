// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use core::result::Result::Ok;
use sequent_core::plugins::{get_plugin_shared_dir, Plugins};
use sequent_core::std_temp_path::create_temp_file;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tracing::instrument;

use crate::bindings::plugins_manager::documents_manager::documents::{
    run_shell_command_ecies_encrypt_string, run_shell_command_ecies_sign_data,
    run_shell_command_generate_ecies_key_pair,
};
use crate::bindings::plugins_manager::extra_services_manager::cli_service::{
    run_shell_command_check_certificate_cas, run_shell_command_get_p12_cert,
    run_shell_command_get_p12_fingerprint,
};
use uuid::Uuid;

pub const ECIES_TOOL_PATH: &str = "/usr/local/bin/ecies-tool.jar";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EciesKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

#[instrument(err)]
pub fn generate_ecies_key_pair() -> Result<EciesKeyPair, String> {
    let base_path = get_plugin_shared_dir(&Plugins::MIRU);

    let (temp_private_pem_file, temp_private_pem_file_name) =
        create_temp_file("private_key", ".pem", &base_path)?;
    let temp_private_pem_file_path = temp_private_pem_file.path.as_path();

    let (temp_public_pem_file, temp_public_pem_file_name) =
        create_temp_file("public_key", ".pem", &base_path)?;
    let temp_public_pem_file_path = temp_public_pem_file.path.as_path();

    run_shell_command_generate_ecies_key_pair(
        ECIES_TOOL_PATH,
        &temp_public_pem_file_name,
        &temp_private_pem_file_name,
    )?;

    let private_key_pem =
        fs::read_to_string(temp_private_pem_file_path).map_err(|e| e.to_string())?;
    let public_key_pem =
        fs::read_to_string(temp_public_pem_file_path).map_err(|e| e.to_string())?;

    println!("generate_ecies_key_pair(): public_key_pem: {public_key_pem:?}");

    Ok(EciesKeyPair {
        private_key_pem: private_key_pem,
        public_key_pem: public_key_pem,
    })
}

#[instrument(skip(password), err)]
pub fn ecies_encrypt_string(
    public_key_pem: &str,
    password: &str,
    base_path: &str,
) -> Result<String, String> {
    let (temp_pem_file, temp_pem_file_name) = create_temp_file("public_key", ".pem", base_path)?;
    let temp_pem_file_path = temp_pem_file.path();
    // Write the salt and encrypted data to the output file
    // Using brackets: let it drop out of scope so that all bytes are written
    {
        let mut output_file = File::create(temp_pem_file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        output_file
            .write_all(public_key_pem.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }
    // Encode the &[u8] to a Base64 string
    let result =
        run_shell_command_ecies_encrypt_string(ECIES_TOOL_PATH, &temp_pem_file_name, password)?;

    println!("ecies_encrypt_string: '{}'", result);

    Ok(result)
}

#[instrument(skip(data), err)]
pub fn ecies_sign_data(
    acm_key_pair: &EciesKeyPair,
    data: &str,
    base_path: &str,
) -> Result<String, String> {
    // Retrieve the PEM as a string
    println!("pem: {}", acm_key_pair.private_key_pem);

    let (temp_pem_file, temp_pem_file_name) = create_temp_file("private_key", ".pem", base_path)?;
    let temp_pem_file_path = temp_pem_file.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    // Using brackets: let it drop out of scope so that all bytes are written
    {
        let mut output_file = File::create(temp_pem_file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        output_file
            .write_all(acm_key_pair.private_key_pem.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }
    let (temp_data_file, temp_data_file_name) = create_temp_file("data", ".eml", base_path)?;
    let temp_data_file_path = temp_data_file.path();
    let temp_data_file_string = temp_data_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    {
        let mut output_file = File::create(temp_data_file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        output_file
            .write_all(data.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }

    let encrypted_base64 = run_shell_command_ecies_sign_data(
        ECIES_TOOL_PATH,
        &temp_pem_file_name,
        &temp_data_file_name,
    )?;

    println!("ecies_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}

pub fn get_p12_cert(
    p12_file_path: &str,
    password: &str,
    base_path: &str,
) -> Result<String, String> {
    let (_cert_temp_file, cert_temp_path) = create_temp_file("p12", "cert", base_path)?;

    run_shell_command_get_p12_cert(&p12_file_path, password, &cert_temp_path)?;

    Ok(cert_temp_path)
}

#[instrument(err, ret)]
pub fn get_p12_fingerprint(p12_cert_path: &str) -> Result<String, String> {
    let fingerprint = run_shell_command_get_p12_fingerprint(p12_cert_path)?;

    Ok(fingerprint)
}

#[instrument(skip_all, err)]
pub fn check_certificate_cas(
    p12_cert_path: &str,
    root_ca: &str,
    intermediate_cas: &str,
    dir_base_path: &str,
) -> Result<(), String> {
    // Create a temporary directory
    let unique_dir_name = format!("temp-{}", Uuid::new_v4());
    let temp_dir_path = PathBuf::from(&dir_base_path).join(&unique_dir_name);
    fs::create_dir_all(&temp_dir_path)
        .map_err(|e| format!("Failed to create temporary directory: {}", e))?;

    // write root ca
    let root_ca_file_path = temp_dir_path.join("root-ca.cer");
    fs::write(root_ca_file_path.clone(), root_ca)
        .map_err(|e| format!("Failed to write root CA file: {}", e))?;

    //getting file path without the plugin base dir to send to host.
    let root_ca_file_path_str = format!("{}/root-ca.cer", unique_dir_name);

    // write root ca
    let intermediate_ca_file_path = temp_dir_path.join("intermediate-ca.cer");
    fs::write(intermediate_ca_file_path.clone(), intermediate_cas)
        .map_err(|e| format!("Failed to write intermediate CA file: {}", e))?;

    //getting file path without base dir to send to host.
    let intermediate_ca_file_path_str = format!("{}/intermediate-ca.cer", unique_dir_name);

    let verify_result = run_shell_command_check_certificate_cas(
        &root_ca_file_path_str,
        &intermediate_ca_file_path_str,
        p12_cert_path,
    )?;

    if !verify_result.ends_with(": OK") {
        return Err(format!("{}", verify_result));
    }

    Ok(())
}
