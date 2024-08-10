// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    aes_256_cbc_encrypt::encrypt_file_aes_256_cbc,
    ecies_encrypt::{ecies_encrypt_string, ecies_sign_data, generate_ecies_key_pair},
    eml_generator::render_eml_file,
    xz_compress::xz_compress,
};
use crate::services::{
    password::generate_random_password,
    s3::{download_s3_file_to_string, get_public_asset_file_path},
    temp_path::{generate_temp_file, write_into_named_temp_file},
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use sequent_core::services::reports;
use sequent_core::types::date_time::TimeZone;
use sequent_core::{ballot::Annotations, encrypt::hash_ballot};
use serde_json::{Map, Value};
use std::env;
use std::io::{self, Read, Seek};
use strand::hash::hash_sha256;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use velvet::pipes::generate_reports::ReportData;

const EXAMPLE_PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEViVmM6/r024Bt71ZYT17OhPJHrIx
HqzGxXsLJBrJDxQGIZTXBCpJ49tpj/+Xp1nkf6NYNMjmV8I7vy5F3ShnCQ==
-----END PUBLIC KEY-----
";

pub fn read_temp_file(mut temp_file: NamedTempFile) -> Result<Vec<u8>> {
    // Rewind the file to the beginning to read its contents
    temp_file.rewind()?;

    // Read the file's contents into a Vec<u8>
    let mut file_bytes = Vec::new();
    temp_file.read_to_end(&mut file_bytes)?;
    Ok(file_bytes)
}

#[instrument(skip(report), err)]
async fn generate_compressed_xml(
    tally_id: i64,
    transaction_id: i64,
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
    let template_path = env::var("PUBLIC_ASSETS_EML_BASE_TEMPLATE")?;
    let s3_template_url = get_public_asset_file_path(&template_path)
        .with_context(|| "Error fetching get_minio_url")?;
    let template_string = download_s3_file_to_string(&s3_template_url).await?;
    // render handlebars template
    let render_xml = reports::render_template_text(&template_string, variables_map)
        .map_err(|err| anyhow!("{}", err))?;
    let compressed_xml = xz_compress(render_xml.as_bytes())?;
    Ok(compressed_xml)
}

#[instrument(skip(report), err)]
pub async fn create_transmission_package(
    tally_id: i64,
    transaction_id: i64,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &Annotations,
    election_annotations: &Annotations,
    report: &ReportData,
) -> Result<()> {
    let compressed_xml = generate_compressed_xml(
        tally_id,
        transaction_id,
        time_zone,
        date_time,
        election_event_annotations,
        election_annotations,
        report,
    )
    .await?;

    let random_pass = generate_random_password(64);

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&compressed_xml, "template", ".xml")
            .with_context(|| "Error writing to file")?;
    let mut exz_temp_file = generate_temp_file("er_xxxxxxxx", ".exz")?;
    let exz_temp_file_string = exz_temp_file.path().to_string_lossy().to_string();
    encrypt_file_aes_256_cbc(&temp_path_string, &exz_temp_file_string, &random_pass)?;

    let encrypted_random_pass =
        ecies_encrypt_string(EXAMPLE_PUBLIC_KEY_PEM, random_pass.as_bytes())?;

    let exz_temp_file_bytes = read_temp_file(exz_temp_file)?;
    let exz_hash_bytes = hash_sha256(exz_temp_file_bytes.as_slice())?;
    let exz_hash_base64 = STANDARD.encode(exz_hash_bytes);

    let (private_key_pem_str, public_key_pem_str) = generate_ecies_key_pair()?;
    let (exz_hash_base64, signed_exz_base64) =
        ecies_sign_data(&public_key_pem_str, &exz_hash_base64)?;

    Ok(())
}
