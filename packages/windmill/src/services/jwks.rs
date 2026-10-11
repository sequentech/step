// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use sequent_core::services::jwks::{fetch_jwks, get_jwks_secret_path, JWKKey, JwksOutput};
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::s3;
use sequent_core::util::temp_path::generate_temp_file;
use std::env;
use std::io::BufWriter;
use tracing::{event, instrument, warn, Level};

pub fn get_cache_policy() -> Result<String> {
    let cache_policy = env::var("AWS_S3_JWKS_CACHE_POLICY")
        .map_err(|err| anyhow!("AWS_S3_JWKS_CACHE_POLICY Must be set: {}", { err }))?;
    Ok(cache_policy)
}

#[instrument(err)]
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
pub async fn upsert_realm_jwks(realm: &str) -> Result<()> {
    let realm_jwks = download_realm_jwks_from_keycloak(realm)
        .await
        .unwrap_or(vec![]);
    let mut existing_jwks = fetch_jwks().await?.keys;
    let tagged = tag_realm_keys(&mut existing_jwks, realm, &realm_jwks);
    let existing_kids: Vec<String> = existing_jwks
        .iter()
        .map(|realm| realm.kid.clone())
        .collect();
    let new_jwks: Vec<JWKKey> = realm_jwks
        .into_iter()
        .filter(|key| !existing_kids.contains(&key.kid))
        .map(|key| JWKKey {
            realm: Some(realm.to_string()),
            ..key
        })
        .collect();
    if new_jwks.is_empty() && !tagged {
        event!(Level::INFO, "Jwks for realm {} already present", realm);
        return Ok(());
    }
    existing_jwks.extend(new_jwks);

    write_jwks(existing_jwks).await
}

#[instrument(err)]
pub async fn remove_realm_jwks(realm: &str) -> Result<()> {
    let realm_jwks = download_realm_jwks_from_keycloak(realm)
        .await
        .unwrap_or(vec![]);
    let existing_jwks = fetch_jwks().await?.keys;

    let realm_kids: Vec<String> = realm_jwks.iter().map(|realm| realm.kid.clone()).collect();

    let new_jwks: Vec<JWKKey> = existing_jwks
        .into_iter()
        .filter(|key| key.realm.as_deref() != Some(realm) && !realm_kids.contains(&key.kid))
        .collect();

    write_jwks(new_jwks).await
}

#[instrument(err, skip(keys))]
async fn write_jwks(keys: Vec<JWKKey>) -> Result<()> {
    let jwks_output = JwksOutput { keys };

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

/// Tags the keys without a realm that are among the realm's keys. Returns
/// whether any was tagged.
fn tag_realm_keys(keys: &mut [JWKKey], realm: &str, realm_keys: &[JWKKey]) -> bool {
    let mut tagged = false;
    for key in keys.iter_mut().filter(|key| key.realm.is_none()) {
        if realm_keys.iter().any(|realm_key| realm_key.kid == key.kid) {
            key.realm = Some(realm.to_string());
            tagged = true;
        }
    }
    tagged
}

/// Tags the keys that `certs.json` has without a realm, as written before
/// keys had one, with the Keycloak realm that has them. Harvest does not trust
/// keys without a realm. Keys of no realm are left as they are.
#[instrument(err)]
pub async fn tag_jwks_realms() -> Result<()> {
    if fetch_jwks()
        .await?
        .keys
        .iter()
        .all(|key| key.realm.is_some())
    {
        return Ok(());
    }
    let realms = KeycloakAdminClient::new()
        .await?
        .client
        .get(Some(true))
        .await
        .context("Error listing Keycloak realms")?;
    let mut realms_keys = vec![];
    for realm in realms.into_iter().filter_map(|realm| realm.realm) {
        match download_realm_jwks_from_keycloak(&realm).await {
            Ok(keys) => realms_keys.push((realm, keys)),
            Err(err) => warn!("Skipping realm {realm}: {err:?}"),
        }
    }
    // Read again, so that keys written meanwhile are kept.
    let mut keys = fetch_jwks().await?.keys;
    let mut tagged = false;
    for (realm, realm_keys) in &realms_keys {
        tagged |= tag_realm_keys(&mut keys, realm, realm_keys);
    }
    if !tagged {
        return Ok(());
    }
    write_jwks(keys).await
}
