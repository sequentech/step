// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    eml_generator::{
        find_miru_annotation, MIRU_ELECTION_EVENT_ID, MIRU_ELECTION_EVENT_NAME, MIRU_PLUGIN_PREPEND,
    },
    eml_types::ACMJson,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::types::date_time::TimeZone;
use sequent_core::{
    ballot::Annotations, types::date_time::DateFormat, util::date_time::generate_timestamp,
};
use tracing::instrument;

const ACM_JSON_FORMAT: &str = "%m/%d/%Y %I:%M:%S %p";
const MIRU_DEVICE_ID: &str = "SQUNT420535311";
const MIRU_SERIAL_NUMBER: &str = "SEQ-NT-52706782";
const MIRU_STATION_NAME: &str = "2094A,5346A,6588A,7474A,1489A";
const IP_ADDRESS: &str = "192.168.1.67";
const MAC_ADDRESS: &str = "3C:7E:5A:89:4D:2F";

#[instrument(skip(election_event_annotations), err)]
pub fn generate_acm_json(
    sha256_hash: &str,
    encrypted_key_base64: &str,
    signature: &str,
    publickey: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &Annotations,
    area_station_id: &str,
) -> Result<ACMJson> {
    let MIRU_STATION_ID = area_station_id.to_string();
    let er_datetime = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(ACM_JSON_FORMAT.to_string())),
        Some(date_time.clone()),
    );

    let election_event_id =
        find_miru_annotation(MIRU_ELECTION_EVENT_ID, election_event_annotations).with_context(
            || {
                format!(
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_ID
                )
            },
        )?;
    let election_event_name =
        find_miru_annotation(MIRU_ELECTION_EVENT_NAME, election_event_annotations).with_context(
            || {
                format!(
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_NAME
                )
            },
        )?;
    Ok(ACMJson {
        device_id: MIRU_DEVICE_ID.into(),
        serial_number: MIRU_SERIAL_NUMBER.into(),
        station_id: MIRU_STATION_ID.into(),
        station_name: MIRU_STATION_NAME.into(),
        event_id: election_event_id,
        event_name: election_event_name,
        sha256_hash: sha256_hash.into(),
        encrypted_key: encrypted_key_base64.into(),
        members: vec![],
        ip_address: IP_ADDRESS.into(),
        mac_address: MAC_ADDRESS.into(),
        er_datetime: er_datetime.clone(),
        signature: signature.into(),
        publickey: publickey.into(),
        transfer_start: er_datetime.clone(),
    })
}
