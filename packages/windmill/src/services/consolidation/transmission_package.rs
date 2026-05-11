// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Compresses EML, encrypts payloads, and assembles `er_` / `al_` zip packages for CCS.

use super::{
    acm_json::generate_acm_json,
    aes_256_cbc_encrypt::encrypt_file_aes_256_cbc,
    eml_generator::{
        render_eml_file, MiruAreaAnnotations, MiruElectionAnnotations, MiruElectionEventAnnotations,
    },
    eml_types::ACMJson,
    xz_compress::xz_compress,
    zip::compress_folder_to_zip,
};
use crate::services::consolidation::eml_types::ACMTrustee;
use crate::services::password::generate_random_string_with_charset;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use sequent_core::services::reports;
use sequent_core::services::s3::{download_s3_file_to_string, get_public_asset_file_path};
use sequent_core::signatures::ecies_encrypt::{
    ecies_encrypt_string, ecies_sign_data, EciesKeyPair,
};
use sequent_core::types::date_time::TimeZone;
use sequent_core::util::temp_path::{
    generate_temp_file, read_temp_file, write_into_named_temp_file,
};
use sequent_core::{ballot::Annotations, types::ceremonies::Log};
use serde_json::{Map, Value};
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::{env, path::Path};
use strand::hash::hash_sha256;
use tempfile::tempdir;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use velvet::pipes::generate_reports::ReportData;

/// Handlebars template object key under public assets used to wrap rendered EML XML.
pub const PUBLIC_ASSETS_EML_BASE_TEMPLATE: &str = "eml_base.hbs";

/// Compresses `eml` and returns bytes plus uppercase hex SHA-256 of the original string.
///
/// # Errors
///
/// Hashing or xz compression errors.
#[instrument(skip_all, err)]
pub fn compress_hash_eml(eml: &str) -> Result<(Vec<u8>, String)> {
    let rendered_xml_hash = hash_sha256(eml.as_bytes())
        .with_context(|| "Error hashing the rendered XML")?
        .iter()
        .fold(String::new(), |mut acc, &byte| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{byte:02X}");
            acc
        });

    let compressed_xml =
        xz_compress(eml.as_bytes()).with_context(|| "Error compressing the rendered XML")?;
    Ok((compressed_xml, rendered_xml_hash))
}

/// Renders full EML via public-asset template, then xz-compresses and hashes it.
///
/// # Errors
///
/// Template fetch/render failures, JSON errors, or [`compress_hash_eml`] errors.
#[instrument(skip(reports), err)]
pub async fn generate_base_compressed_xml(
    tally_id: &str,
    transaction_id: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &MiruElectionEventAnnotations,
    election_annotations: &MiruElectionAnnotations,
    area_annotations: &MiruAreaAnnotations,
    reports: &[ReportData],
) -> Result<(Vec<u8>, String, String)> {
    let eml_data = render_eml_file(
        tally_id,
        transaction_id,
        time_zone,
        date_time,
        election_event_annotations,
        election_annotations,
        area_annotations,
        reports,
    )?;
    let mut variables_map: Map<String, Value> = Map::new();
    variables_map.insert("data".to_string(), serde_json::to_value(eml_data)?);
    let template_path = PUBLIC_ASSETS_EML_BASE_TEMPLATE;
    let s3_template_url = get_public_asset_file_path(template_path)
        .with_context(|| "Error fetching get_minio_url")?;
    let template_string = download_s3_file_to_string(&s3_template_url).await?;
    // render handlebars template
    let render_xml = reports::render_template_text(&template_string, variables_map)
        .map_err(|err| anyhow!("{err:?}"))?;
    let (compressed_xml, rendered_xml_hash) = compress_hash_eml(&render_xml)?;
    Ok((compressed_xml, render_xml, rendered_xml_hash))
}

/// Writes xz to temp, AES-encrypts to `.exz`, and returns ECIES-wrapped random passphrase (base64).
///
/// # Errors
///
/// Temp file, openssl encrypt, or ECIES encrypt errors.
#[instrument(skip(compressed_xml), err)]
async fn generate_encrypted_compressed_xml(
    compressed_xml: Vec<u8>,
    public_key_pem: &str,
) -> Result<(NamedTempFile, String)> {
    let charset: String = "0123456789abcdef".into();
    let random_pass = generate_random_string_with_charset(64, &charset);

    let (_temp_path, temp_path_string, _file_size) =
        write_into_named_temp_file(&compressed_xml, "template", ".xz")
            .map_err(|e| anyhow!("Error writing into temp file: {e:?}"))?;
    let exz_temp_file = generate_temp_file("er_xxxxxxxx", ".exz")
        .map_err(|e| anyhow!("Error creating temp file: {e:?}"))?;
    let exz_temp_file_string = exz_temp_file.path().to_string_lossy().to_string();
    encrypt_file_aes_256_cbc(&temp_path_string, &exz_temp_file_string, &random_pass)
        .map_err(|e| anyhow!("Error encrypting the ZIP file: {e:?}"))?;

    let encrypted_random_pass_base64 = ecies_encrypt_string(public_key_pem, &random_pass)
        .map_err(|e| anyhow!("Error encrypting the random pass: {e:?}"))?;
    Ok((exz_temp_file, encrypted_random_pass_base64))
}

/// Writes `.exz` + `.json` siblings for a station id and zips the temp folder to `output_file_path`.
///
/// # Errors
///
/// Filesystem, JSON serialization, or [`compress_folder_to_zip`] errors.
#[instrument(skip_all, err)]
fn generate_er_final_zip(
    exz_temp_file_bytes: Vec<u8>,
    acm_json: ACMJson,
    area_station_id: &str,
    output_file_path: &Path,
    is_log: bool,
) -> Result<()> {
    let MIRU_STATION_ID = area_station_id.to_string();
    let temp_dir = tempdir().with_context(|| "Error generating temp directory")?;
    let temp_dir_path = temp_dir.path();

    let prefix = if is_log { "al_" } else { "er_" };

    let exz_xml_path = temp_dir_path.join(format!("{prefix}{MIRU_STATION_ID}.exz").as_str());
    {
        let mut exz_xml_file = File::create(&exz_xml_path).with_context(|| {
            format!("Failed to create or open file: {}", exz_xml_path.display())
        })?;
        exz_xml_file
            .write_all(&exz_temp_file_bytes)
            .with_context(|| format!("Failed to write data to file: {}", exz_xml_path.display()))?;
    }

    let acm_json_stringified = serde_json::to_string_pretty(&acm_json)?;
    let exz_json_path = temp_dir_path.join(format!("{prefix}{MIRU_STATION_ID}.json"));
    {
        let mut exz_json_file = File::create(&exz_json_path).with_context(|| {
            format!("Failed to create or open file: {}", exz_json_path.display())
        })?;
        exz_json_file
            .write_all(acm_json_stringified.as_bytes())
            .with_context(|| {
                format!("Failed to write data to file: {}", exz_json_path.display())
            })?;
    }

    compress_folder_to_zip(temp_dir_path, output_file_path)?;
    Ok(())
}

/// Builds the audit-log zip (`al_*.zip`) for one CCS destination: logs JSON → xz → encrypt → ACM JSON → zip.
///
/// # Errors
///
/// Serialization, encryption, signing, or zip errors in the pipeline.
#[instrument(skip(acm_key_pair), err)]
pub async fn create_logs_package(
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
) -> Result<()> {
    let logs_str = serde_json::to_string(logs).context("Can't stringify logs")?;

    let (compressed_xml, rendered_xml_hash) = compress_hash_eml(&logs_str)?;

    let (mut exz_temp_file, encrypted_random_pass_base64) =
        generate_encrypted_compressed_xml(compressed_xml, ccs_public_key_pem_str).await?;

    let exz_temp_file_bytes =
        read_temp_file(&mut exz_temp_file).with_context(|| "Error reading the exz")?;
    let signed_eml_base64 =
        ecies_sign_data(acm_key_pair, &logs_str).with_context(|| "Error signing the eml hash")?;

    info!(
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

/// Builds the election-results zip (`er_*.zip`): encrypted xz tally payload plus ACM metadata and signatures.
///
/// # Errors
///
/// Same pipeline failures as [`create_logs_package`], but signing the raw EML body instead of logs JSON.
#[instrument(skip(compressed_xml, acm_key_pair), err)]
pub async fn create_transmission_package(
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
) -> Result<()> {
    let (mut exz_temp_file, encrypted_random_pass_base64) =
        generate_encrypted_compressed_xml(compressed_xml, ccs_public_key_pem_str).await?;

    let exz_temp_file_bytes =
        read_temp_file(&mut exz_temp_file).with_context(|| "Error reading the exz")?;
    let signed_eml_base64 =
        ecies_sign_data(acm_key_pair, eml).with_context(|| "Error signing the eml hash")?;

    info!(
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
