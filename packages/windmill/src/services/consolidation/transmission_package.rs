// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    acm_json::generate_acm_json,
    aes_256_cbc_encrypt::encrypt_file_aes_256_cbc,
    ecies_encrypt::{ecies_encrypt_string, ecies_sign_data, generate_ecies_key_pair, EciesKeyPair},
    eml_generator::render_eml_file,
    eml_types::ACMJson,
    xz_compress::xz_compress,
    zip::compress_folder_to_zip,
};
use crate::services::{
    password::generate_random_password,
    s3::{download_s3_file_to_string, get_public_asset_file_path},
    temp_path::{generate_temp_file, write_into_named_temp_file},
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::ballot::Annotations;
use sequent_core::services::reports;
use sequent_core::types::date_time::TimeZone;
use serde_json::{Map, Value};
use std::env;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use tempfile::tempdir;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use velvet::pipes::generate_reports::ReportData;

#[instrument(err)]
pub fn read_temp_file(mut temp_file: NamedTempFile) -> Result<Vec<u8>> {
    // Rewind the file to the beginning to read its contents
    temp_file.rewind()?;

    // Read the file's contents into a Vec<u8>
    let mut file_bytes = Vec::new();
    temp_file.read_to_end(&mut file_bytes)?;
    Ok(file_bytes)
}

#[instrument(skip(report), err)]
pub async fn generate_base_compressed_xml(
    tally_id: &str,
    transaction_id: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &Annotations,
    election_annotations: &Annotations,
    report: &ReportData,
) -> Result<Vec<u8>> {
    let eml_data = render_eml_file(
        tally_id,
        transaction_id,
        time_zone,
        date_time,
        &election_event_annotations,
        &election_annotations,
        &report,
    )?;
    let mut variables_map: Map<String, Value> = Map::new();
    variables_map.insert("data".to_string(), serde_json::to_value(eml_data)?);
    let template_path = env::var("PUBLIC_ASSETS_EML_BASE_TEMPLATE")
        .with_context(|| "Missing env var PUBLIC_ASSETS_EML_BASE_TEMPLATE")?;
    let s3_template_url = get_public_asset_file_path(&template_path)
        .with_context(|| "Error fetching get_minio_url")?;
    let template_string = download_s3_file_to_string(&s3_template_url).await?;
    // render handlebars template
    let render_xml = reports::render_template_text(&template_string, variables_map)
        .map_err(|err| anyhow!("{}", err))?;
    let compressed_xml = xz_compress(render_xml.as_bytes())?;
    Ok(compressed_xml)
}

#[instrument(skip(compressed_xml, acm_key_pair), err)]
async fn generate_encrypted_compressed_xml(
    compressed_xml: Vec<u8>,
    public_key_pem: &str,
    acm_key_pair: &EciesKeyPair,
) -> Result<(NamedTempFile, String)> {
    let random_pass = generate_random_password(64);

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&compressed_xml, "template", ".xz")
            .with_context(|| "Error writing to file")?;
    let exz_temp_file = generate_temp_file("er_xxxxxxxx", ".exz")?;
    let exz_temp_file_string = exz_temp_file.path().to_string_lossy().to_string();
    encrypt_file_aes_256_cbc(&temp_path_string, &exz_temp_file_string, &random_pass)?;

    let encrypted_random_pass_base64 =
        ecies_encrypt_string(public_key_pem, acm_key_pair, random_pass.as_bytes())?;
    Ok((exz_temp_file, encrypted_random_pass_base64))
}

#[instrument(skip_all, err)]
fn generate_er_final_zip(exz_temp_file_bytes: Vec<u8>, acm_json: ACMJson) -> Result<NamedTempFile> {
    let MIRU_STATION_ID =
        std::env::var("MIRU_STATION_ID").map_err(|_| anyhow!("MIRU_STATION_ID env var missing"))?;
    // Create a temporary directory
    let temp_dir = tempdir().with_context(|| "Error generating temp directory")?;
    let temp_dir_path = temp_dir.path();

    let exz_xml_path = temp_dir_path.join(format!("er_{}.exz", MIRU_STATION_ID).as_str());
    let mut exz_xml_file = File::create(&exz_xml_path)
        .with_context(|| format!("Failed to create or open file: {:?}", exz_xml_path))?;
    exz_xml_file
        .write_all(&exz_temp_file_bytes)
        .with_context(|| format!("Failed to write data to file: {:?}", exz_xml_path))?;

    let acm_json_stringified = serde_json::to_string_pretty(&acm_json)?;
    let exz_json_path = temp_dir_path.join(format!("er_{}.json", MIRU_STATION_ID).as_str());
    let mut exz_json_file = File::create(&exz_json_path)
        .with_context(|| format!("Failed to create or open file: {:?}", exz_json_path))?;
    exz_json_file
        .write_all(acm_json_stringified.as_bytes())
        .with_context(|| format!("Failed to write data to file: {:?}", exz_xml_path))?;

    let dst_file = generate_temp_file(format!("er_{}", MIRU_STATION_ID).as_str(), ".zip")?;
    compress_folder_to_zip(temp_dir_path, dst_file.path())?;
    Ok(dst_file)
}

#[instrument(skip(compressed_xml, acm_key_pair), err)]
pub async fn create_transmission_package(
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &Annotations,
    compressed_xml: Vec<u8>,
    acm_key_pair: &EciesKeyPair,
    ccs_public_key_pem_str: &str,
) -> Result<NamedTempFile> {
    let (mut exz_temp_file, encrypted_random_pass_base64) =
        generate_encrypted_compressed_xml(compressed_xml, ccs_public_key_pem_str, acm_key_pair)
            .await?;

    let exz_temp_file_bytes = read_temp_file(exz_temp_file)?;
    let (exz_hash_base64, signed_exz_base64) =
        ecies_sign_data(acm_key_pair, &exz_temp_file_bytes)?;

    let acm_json = generate_acm_json(
        &exz_hash_base64,
        &encrypted_random_pass_base64,
        &signed_exz_base64,
        &acm_key_pair.public_key_pem,
        time_zone,
        date_time,
        election_event_annotations,
    )?;
    let zip_tmp_file = generate_er_final_zip(exz_temp_file_bytes, acm_json)?;

    Ok(zip_tmp_file)
}
