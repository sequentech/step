// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Managing JWKS maintenance: S3 cache, Keycloak realm certs, and realm-scoped key removal.

use anyhow::{anyhow, Context, Result};
use sequent_core::services::s3;
use sequent_core::util::temp_path::generate_temp_file;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{BufWriter, Write};
use tempfile::NamedTempFile;
use tracing::{event, instrument, Level};

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Single JWK entry as used by Keycloak/OpenID Connect.
pub struct JWKKey {
    /// Signing algorithm (e.g. "RS256").
    pub alg: String,
    /// Key type (e.g. "RSA").
    pub kty: String,
    /// Intended use (typically "sig").
    pub r#use: String,
    /// Modulus (base64url).
    pub n: String,
    /// Exponent (base64url).
    pub e: String,
    /// Key id.
    pub kid: String,
    /// X.509 certificate SHA-1 thumbprint.
    pub x5t: String,
    /// X.509 certificate chain (base64 DER).
    pub x5c: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// JWKS payload as returned by Keycloak/OpenID endpoints.
pub struct JwksOutput {
    /// Key list.
    pub keys: Vec<JWKKey>,
}

/// S3 object path where the aggregated JWKS is stored.
pub fn get_jwks_secret_path() -> String {
    env::var("AWS_S3_JWKS_CERTS_PATH").unwrap_or("certs.json".to_string())
}

/// Read the cache-control policy used when uploading the JWKS object.
///
/// # Errors
///
/// Returns an error if `AWS_S3_JWKS_CACHE_POLICY` is not set.
pub fn get_cache_policy() -> Result<String> {
    let cache_policy = env::var("AWS_S3_JWKS_CACHE_POLICY")
        .map_err(|err| anyhow!("AWS_S3_JWKS_CACHE_POLICY Must be set: {}", { err }))?;
    Ok(cache_policy)
}

#[instrument(err)]
/// Get the JWKS from the S3 bucket.
///
/// # Errors
///
/// Returns an error if the JWKS object cannot be fetched or parsed.
pub async fn get_jwks() -> Result<Vec<JWKKey>> {
    let minio_private_uri =
        env::var("AWS_S3_PRIVATE_URI").map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = s3::get_public_bucket()?;

    let hasura_endpoint = format!(
        "{}/{}/{}",
        minio_private_uri,
        bucket,
        get_jwks_secret_path()
    );

    let client = reqwest::Client::new();
    let response = client.get(hasura_endpoint).send().await?;

    let unwrapped = if response.status() == reqwest::StatusCode::NOT_FOUND {
        event!(Level::INFO, "Jwks are empty");
        return Ok(vec![]);
    } else {
        response
    };
    let response_body: JwksOutput = unwrapped.json().await?;
    Ok(response_body.keys)
}

#[instrument(err)]
/// Download a realm's JWKS from Keycloak.
///
/// # Errors
///
/// Returns an error if Keycloak cannot be reached or the JWKS response cannot be parsed.
pub async fn download_realm_jwks_from_keycloak(realm: &str) -> Result<Vec<JWKKey>> {
    let keycloak_url =
        env::var("KEYCLOAK_URL").map_err(|err| anyhow!("KEYCLOAK_URL must be set"))?;
    let hasura_endpoint = format!(
        "{}/realms/{}/protocol/openid-connect/certs",
        keycloak_url, realm
    );

    let client = reqwest::Client::new();
    let res = client
        .get(hasura_endpoint)
        .send()
        .await
        .map_err(|err| anyhow!("Error downloading JWKS: {err:?}"))?;
    let response_body: JwksOutput = res
        .json()
        .await
        .map_err(|err| anyhow!("Error parsing JWKS: {err:?}"))?;
    Ok(response_body.keys)
}

#[instrument(err)]
/// Download a realm's JWKS and merge any new keys into the S3 aggregate.
///
/// # Errors
///
/// Returns an error if reading existing JWKS from S3 or writing the updated object fails.
pub async fn upsert_realm_jwks(realm: &str) -> Result<()> {
    let realm_jwks = download_realm_jwks_from_keycloak(realm)
        .await
        .unwrap_or(vec![]);
    let mut existing_jwks = get_jwks().await?;
    let existing_kids: Vec<String> = existing_jwks
        .iter()
        .map(|realm| realm.kid.clone())
        .collect();
    let new_jwks: Vec<JWKKey> = realm_jwks
        .clone()
        .into_iter()
        .filter(|key| !existing_kids.contains(&key.kid))
        .collect();
    if new_jwks.is_empty() {
        event!(Level::INFO, "Jwks for realm {} already present", realm);
        return Ok(());
    }
    existing_jwks.extend(new_jwks);

    let jwks_output = JwksOutput {
        keys: existing_jwks,
    };

    let file = generate_temp_file("jwks-", ".json").with_context(|| "Error creating temp file")?;
    let file2 = file
        .reopen()
        .with_context(|| "Couldn't reopen file for writing")?;
    let buf_writer = BufWriter::new(file2);
    serde_json::to_writer_pretty(buf_writer, &jwks_output)
        .with_context(|| "Failed writing into temp file")?;

    let temp_path = file.into_temp_path();

    s3::upload_file_to_s3(
        /* key */ get_jwks_secret_path(),
        /* is_public */ false,
        /* s3_bucket */ s3::get_public_bucket()?,
        /* media_type */ "application/json".to_string(),
        /* file_path */ temp_path.to_string_lossy().to_string(),
        /* cache_control_policy */ Some(get_cache_policy()?),
        /* download filed name */ None,
    )
    .await
    .with_context(|| "Error uploading file to s3")?;

    temp_path
        .close()
        .with_context(|| "Error closing temp file path")?;

    Ok(())
}

/// Remove all JWK entries for `realm` from the aggregated JWKS object in S3.
///
/// # Errors
///
/// Returns an error if Keycloak, S3, or temp file I/O fails.
#[instrument(err)]
pub async fn remove_realm_jwks(realm: &str) -> Result<()> {
    let realm_jwks = download_realm_jwks_from_keycloak(realm)
        .await
        .unwrap_or(vec![]);
    let existing_jwks = get_jwks().await?;

    let realm_kids: Vec<String> = realm_jwks.iter().map(|realm| realm.kid.clone()).collect();

    let new_jwks: Vec<JWKKey> = existing_jwks
        .clone()
        .into_iter()
        .filter(|key| !realm_kids.contains(&key.kid))
        .collect();

    let jwks_output = JwksOutput { keys: new_jwks };

    let file = generate_temp_file("jwks-", ".json").with_context(|| "Error creating temp file")?;
    let file2 = file
        .reopen()
        .with_context(|| "Couldn't reopen file for writing")?;
    let buf_writer = BufWriter::new(file2);
    serde_json::to_writer_pretty(buf_writer, &jwks_output)
        .with_context(|| "Failed writing into temp file")?;

    let temp_path = file.into_temp_path();

    s3::upload_file_to_s3(
        /* key */ get_jwks_secret_path(),
        /* is_public */ false,
        /* s3_bucket */ s3::get_public_bucket()?,
        /* media_type */ "application/json".to_string(),
        /* file_path */ temp_path.to_string_lossy().to_string(),
        /* cache_control_policy */ Some(get_cache_policy()?),
        /* download filed name */ None,
    )
    .await
    .with_context(|| "Error uploading file to s3")?;

    temp_path
        .close()
        .with_context(|| "Error closing temp file path")?;

    Ok(())
}
