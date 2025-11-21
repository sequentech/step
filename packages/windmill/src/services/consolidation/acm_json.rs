// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    eml_generator::{
        find_miru_annotation, MiruAreaAnnotations, MiruElectionAnnotations,
        MiruElectionEventAnnotations, MIRU_ELECTION_EVENT_ID, MIRU_ELECTION_EVENT_NAME,
        MIRU_PLUGIN_PREPEND,
    },
    eml_types::ACMJson,
};
use crate::services::consolidation::eml_types::ACMTrustee;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signatures::ecies_encrypt::{generate_ecies_key_pair, EciesKeyPair};
use sequent_core::{
    ballot::Annotations, types::date_time::DateFormat, util::date_time::generate_timestamp,
};
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str, types::date_time::TimeZone,
};
use std::env;
use tracing::instrument;

const ACM_JSON_FORMAT: &str = "%m/%d/%Y %I:%M:%S %p";
const DEFAULT_MIRU_DEVICE_ID: &str = "SQUNT420535311";
const DEFAULT_MIRU_SERIAL_NUMBER: &str = "SEQ-NT-52706782";
//const DEFAULT_MIRU_STATION_NAME: &str = "2094A,5346A,6588A,7474A,1489A";
const DEFAULT_MIRU_IP_ADDRESS: &str = "192.168.1.67";
const DEFAULT_MIRU_MAC_ADDRESS: &str = "3C:7E:5A:89:4D:2F";

pub fn get_miru_device_id() -> String {
    env::var("MIRU_DEVICE_ID").unwrap_or(DEFAULT_MIRU_DEVICE_ID.to_string())
}

pub fn get_miru_serial_number() -> String {
    env::var("MIRU_SERIAL_NUMBER").unwrap_or(DEFAULT_MIRU_SERIAL_NUMBER.to_string())
}

pub fn get_miru_ip_address() -> String {
    env::var("MIRU_IP_ADDRESS").unwrap_or(DEFAULT_MIRU_IP_ADDRESS.to_string())
}

pub fn get_miru_mac_address() -> String {
    env::var("_MIRU_MAC_ADDRESS").unwrap_or(DEFAULT_MIRU_MAC_ADDRESS.to_string())
}

#[instrument(err)]
pub async fn get_acm_key_pair(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<EciesKeyPair> {
    let secret_key = format!(
        "acm-key-pair-{}-{}",
        get_miru_device_id(),
        get_miru_serial_number()
    );

    if let Some(secret_str) = vault::read_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &secret_key,
    )
    .await?
    {
        deserialize_str(&secret_str).map_err(|err| anyhow!("{}", err))
    } else {
        let key_pair = generate_ecies_key_pair()?;
        let secret_str = serde_json::to_string(&key_pair)?;
        vault::save_secret(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            &secret_key,
            &secret_str,
        )
        .await?;
        Ok(key_pair)
    }
}

#[instrument(skip(election_event_annotations), err)]
pub fn generate_acm_json(
    sha256_hash: &str,
    encrypted_key_base64: &str,
    signature: &str,
    publickey: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &MiruElectionEventAnnotations,
    election_annotations: &MiruElectionAnnotations,
    area_annotations: &MiruAreaAnnotations,
    server_signatures: &Vec<ACMTrustee>,
) -> Result<ACMJson> {
    let er_datetime = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(ACM_JSON_FORMAT.to_string())),
        Some(date_time.clone()),
    );
    Ok(ACMJson {
        device_id: get_miru_device_id(),
        serial_number: get_miru_serial_number(),
        station_id: area_annotations.station_id.to_string(),
        station_name: area_annotations.station_name.clone(),
        event_id: election_event_annotations.event_id.clone(),
        event_name: election_event_annotations.event_name.clone(),
        sha256_hash: sha256_hash.into(),
        encrypted_key: encrypted_key_base64.into(),
        members: server_signatures.clone(),
        ip_address: get_miru_ip_address(),
        mac_address: get_miru_mac_address(),
        er_datetime: er_datetime.clone(),
        signature: signature.into(),
        publickey: publickey.into(),
        transfer_start: er_datetime.clone(),
    })
}
