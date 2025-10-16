// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    acm_json::generate_acm_json,
    eml_generator::{
        render_eml_file, MiruAreaAnnotations, MiruElectionAnnotations, MiruElectionEventAnnotations,
    },
    eml_types::ACMJson,
    xz_compress::xz_compress,
    zip::compress_folder_to_zip,
};
use crate::{
    bindings::plugins_manager::documents_manager::documents::{
        download_s3_file_to_string, encrypt_file, get_s3_public_asset_file_path, hash_sha256,
        render_template_text,
    },
    services::{
        eml_types::ACMTrustee,
        signatures::{ecies_encrypt_string, ecies_sign_data, EciesKeyPair},
    },
};
use chrono::{DateTime, Utc};
use sequent_core::types::date_time::TimeZone;
use sequent_core::{
    password::generate_random_string_with_charset,
    std_temp_path::{create_temp_file, read_temp_file, write_into_named_temp_file, TempFileGuard},
    types::{ceremonies::Log, velvet::ReportData},
};
use serde_json::{Map, Value};
use std::io::Write;
use std::{env, fs, path::Path};
use std::{fs::File, path::PathBuf};
use tracing::instrument;
use wit_bindgen_rt::async_support::futures::TryFutureExt;

pub const PUBLIC_ASSETS_EML_BASE_TEMPLATE: &'static str = "eml_base.hbs";

// returns (base_compressed_xml, eml, eml_hash)
#[instrument(skip_all, err)]
pub fn compress_hash_eml(eml: &str) -> Result<(Vec<u8>, String), String> {
    println!("Compressing and hashing EML...");
    let rendered_xml_hash = hash_sha256(eml.as_bytes())
        .map_err(|e| format!("Error hashing the rendered XML: {}", e))?
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect();
    println!("Rendered XML hash: {}", rendered_xml_hash);

    let compressed_xml = xz_compress(eml.as_bytes())
        .map_err(|e| format!("Error compressing the rendered XML: {}", e))?;
    Ok((compressed_xml, rendered_xml_hash))
}

#[instrument(skip(reports), err)]
pub fn generate_base_compressed_xml(
    tally_id: &str,
    transaction_id: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &MiruElectionEventAnnotations,
    election_annotations: &MiruElectionAnnotations,
    area_annotations: &MiruAreaAnnotations,
    reports: &Vec<ReportData>,
) -> Result<(Vec<u8>, String, String), String> {
    let eml_data = render_eml_file(
        tally_id,
        transaction_id,
        time_zone,
        date_time,
        &election_event_annotations,
        &election_annotations,
        area_annotations,
        &reports,
    )
    .map_err(|e| format!("Error render eml file: {}", e))?;
    let mut variables_map: Map<String, Value> = Map::new();
    variables_map.insert(
        "data".to_string(),
        serde_json::to_value(eml_data).map_err(|e| format!("Error serializing EML data: {}", e))?,
    );
    let template_path = PUBLIC_ASSETS_EML_BASE_TEMPLATE;
    let s3_template_url = get_s3_public_asset_file_path(&template_path)
        .map_err(|e| format!("Error fetching S3 template URL: {}", e))?;
    println!("S3 Template URL: {}", s3_template_url);
    let template_string = download_s3_file_to_string(&s3_template_url)
        .map_err(|e| format!("Error downloading S3 template file: {}", e))?;
    // render handlebars template
    let variables_map_str = serde_json::to_string(&variables_map)
        .map_err(|e| format!("Error serializing variables map: {}", e))?;
    let render_xml = render_template_text(&template_string, &variables_map_str).map_err(|err| {
        println!("[Guest Plugin] Error rendering template: {}", err);
        format!("{}", err)
    })?;

    let (compressed_xml, rendered_xml_hash) = compress_hash_eml(&render_xml).map_err(|e| {
        println!("[Guest Plugin] Error compressing and hashing EML: {}", e);
        format!("Error compressing and hashing EML: {}", e)
    })?;

    Ok((compressed_xml, render_xml, rendered_xml_hash))
}

#[instrument(skip(compressed_xml), err)]
fn generate_encrypted_compressed_xml(
    compressed_xml: Vec<u8>,
    public_key_pem: &str,
    dir_base_path: &str,
) -> Result<(TempFileGuard, String), String> {
    let charset: String = "0123456789abcdef".into();
    let random_pass = generate_random_string_with_charset(64, &charset);

    let (_temp_path, temp_path_file_name, temp_path_string, _file_size) =
        write_into_named_temp_file(&compressed_xml, "template", ".xz", dir_base_path)
            .map_err(|e| format!("Error writing into temp file: {e:?}"))?;

    let (exz_temp_file, exz_temp_file_name) =
        create_temp_file("er_xxxxxxxx", ".exz", dir_base_path)
            .map_err(|e| format!("Error creating temp file: {e:?}"))?;

    // Encrypt the temporary files using AES-256-CBC
    encrypt_file(&temp_path_file_name, &exz_temp_file_name, &random_pass);

    let encrypted_random_pass_base64 =
        ecies_encrypt_string(public_key_pem, &random_pass, dir_base_path)
            .map_err(|e| format!("Error encrypting the random pass: {e:?}"))?;
    Ok((exz_temp_file, encrypted_random_pass_base64))
}

#[instrument(skip_all, err)]
fn generate_er_final_zip(
    exz_temp_file_bytes: Vec<u8>,
    acm_json: ACMJson,
    area_station_id: &str,
    output_file_path: &Path,
    is_log: bool,
) -> Result<(), String> {
    let MIRU_STATION_ID = area_station_id.to_string();

    let mut temp_dir_path = PathBuf::new();
    fs::create_dir_all(&temp_dir_path).map_err(|e| e.to_string())?;

    let prefix = if is_log { "al_" } else { "er_" };

    let exz_xml_path = temp_dir_path.join(format!("{}{}.exz", prefix, MIRU_STATION_ID).as_str());
    {
        let mut exz_xml_file = File::create(&exz_xml_path)
            .map_err(|e| format!("Failed to create or open file: {:?}", exz_xml_path))?;
        exz_xml_file
            .write_all(&exz_temp_file_bytes)
            .map_err(|e| format!("Failed to write data to file: {:?}", exz_xml_path))?;
    }

    let acm_json_stringified = serde_json::to_string_pretty(&acm_json)
        .map_err(|e| format!("Failed convert acm_json to string: {}", e))?;

    let exz_json_path = temp_dir_path.join(format!("{}{}.json", prefix, MIRU_STATION_ID).as_str());
    {
        let mut exz_json_file = File::create(&exz_json_path)
            .map_err(|e| format!("Failed to create or open file: {:?}", exz_json_path))?;
        exz_json_file
            .write_all(acm_json_stringified.as_bytes())
            .map_err(|e| format!("Failed to write data to file: {:?}", exz_json_path))?;
    }

    compress_folder_to_zip(temp_dir_path.as_path(), output_file_path)
        .map_err(|e| format!("Error compress folder to zip: {}", e))?;

    Ok(())
}

#[instrument(skip(acm_key_pair), err)]
pub fn create_logs_package(
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &MiruElectionEventAnnotations,
    election_annotations: &MiruElectionAnnotations,
    acm_key_pair: &EciesKeyPair,
    ccs_public_key_pem_str: &str,
    area_annotations: &MiruAreaAnnotations,
    output_file_path: &Path,
    server_signatures: &Vec<ACMTrustee>,
    logs: &Vec<Log>,
    dir_base_path: &str,
) -> Result<(), String> {
    let logs_str =
        serde_json::to_string(logs).map_err(|e| format!("Can't stringify logs: {}", e))?;

    let (compressed_xml, rendered_xml_hash) = compress_hash_eml(&logs_str)?;

    let (mut exz_temp_file, encrypted_random_pass_base64): (TempFileGuard, String) =
        generate_encrypted_compressed_xml(compressed_xml, ccs_public_key_pem_str, dir_base_path)
            .map_err(|e| format!(" Error in generate_encrypted_compressed_xml: {}", e))?;

    let exz_temp_file_bytes =
        read_temp_file(&mut exz_temp_file).map_err(|e| format!("Error reading the exz: {}", e))?;
    let signed_eml_base64 = ecies_sign_data(acm_key_pair, &logs_str, dir_base_path)
        .map_err(|e| format!("Error signing the eml hash: {}", e))?;

    println!(
        "create_logs_package(): acm_key_pair.public_key_pem = {:?}",
        acm_key_pair.public_key_pem
    );
    let logs_servers: Vec<ACMTrustee> = server_signatures
        .clone()
        .into_iter()
        .map(|server| ACMTrustee {
            id: server.id.clone(),
            signature: None,
            publickey: None,
            name: server.name.clone(),
        })
        .collect();
    let acm_json = generate_acm_json(
        &rendered_xml_hash,
        &encrypted_random_pass_base64,
        &signed_eml_base64,
        &acm_key_pair.public_key_pem,
        time_zone,
        date_time,
        election_event_annotations,
        election_annotations,
        area_annotations,
        &logs_servers,
    )?;
    generate_er_final_zip(
        exz_temp_file_bytes,
        acm_json,
        &area_annotations.station_id,
        output_file_path,
        true,
    )?;

    Ok(())
}

#[instrument(skip(compressed_xml, acm_key_pair), err)]
pub fn create_transmission_package(
    eml_hash: &str,
    eml: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &MiruElectionEventAnnotations,
    compressed_xml: Vec<u8>,
    acm_key_pair: &EciesKeyPair,
    ccs_public_key_pem_str: &str,
    area_annotations: &MiruAreaAnnotations,
    output_file_path: &Path,
    server_signatures: &Vec<ACMTrustee>,
    election_annotations: &MiruElectionAnnotations,
    dir_base_path: &str,
) -> Result<(), String> {
    let (mut exz_temp_file, encrypted_random_pass_base64) =
        generate_encrypted_compressed_xml(compressed_xml, ccs_public_key_pem_str, dir_base_path)?;

    let exz_temp_file_bytes =
        read_temp_file(&exz_temp_file).map_err(|e| format!("Error reading the exz: {}", e))?;
    let signed_eml_base64 = ecies_sign_data(acm_key_pair, eml, dir_base_path)
        .map_err(|e| format!("Error signing the eml hash: {}", e))?;

    println!(
        "create_transmission_package(): acm_key_pair.public_key_pem = {:?}",
        acm_key_pair.public_key_pem
    );
    let acm_json = generate_acm_json(
        eml_hash,
        &encrypted_random_pass_base64,
        &signed_eml_base64,
        &acm_key_pair.public_key_pem,
        time_zone,
        date_time,
        election_event_annotations,
        election_annotations,
        area_annotations,
        server_signatures,
    )?;
    generate_er_final_zip(
        exz_temp_file_bytes,
        acm_json,
        &area_annotations.station_id,
        output_file_path,
        false,
    )?;

    Ok(())
}
