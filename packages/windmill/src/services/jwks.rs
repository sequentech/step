// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::s3;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::env;
use tracing::{event, instrument, Level};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JWKKey {
    pub alg: String,
    pub kty: String,
    pub r#use: String,
    pub n: String,
    pub e: String,
    pub kid: String,
    pub x5t: String,
    pub x5c: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JwksOutput {
    pub keys: Vec<JWKKey>,
}

fn get_jwks_secret_path() -> String {
    "certs.json".to_string()
}

#[instrument]
pub async fn get_jwks() -> Result<Vec<JWKKey>> {
    let minio_private_uri =
        env::var("MINIO_PRIVATE_URI").expect(&format!("MINIO_PRIVATE_URI must be set"));
    let bucket = s3::get_public_bucket();

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

#[instrument]
pub async fn download_realm_jwks_from_keycloak(realm: &str) -> Result<Vec<JWKKey>> {
    let keycloak_url = env::var("KEYCLOAK_URL").expect(&format!("KEYCLOAK_URL must be set"));
    let hasura_endpoint = format!(
        "{}/realms/{}/protocol/openid-connect/certs",
        keycloak_url, realm
    );

    let client = reqwest::Client::new();
    let res = client.get(hasura_endpoint).send().await?;
    let response_body: JwksOutput = res.json().await?;
    Ok(response_body.keys)
}

#[instrument]
pub async fn upsert_realm_jwks(realm: &str) -> Result<()> {
    let realm_jwks = download_realm_jwks_from_keycloak(realm).await?;
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
    if 0 == new_jwks.len() {
        event!(Level::INFO, "Jwks for realm {} already present", realm);
        return Ok(());
    }
    existing_jwks.extend(new_jwks);

    let jwks_output = JwksOutput {
        keys: existing_jwks,
    };
    let jwks_output_str = to_string(&jwks_output)?;

    s3::upload_to_s3(
        &jwks_output_str.into_bytes(),
        get_jwks_secret_path(),
        "application/json".to_string(),
        s3::get_public_bucket(),
    )
    .await?;

    Ok(())
}
