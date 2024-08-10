// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    eml_generator::{
        find_miru_annotation, MIRU_ELECTION_EVENT_ID, MIRU_ELECTION_EVENT_NAME, MIRU_PLUGIN_PREPEND,
    },
    eml_types::ACMJson,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::types::date_time::TimeZone;
use sequent_core::{
    ballot::Annotations, types::date_time::DateFormat, util::date_time::generate_timestamp,
};

const ACM_JSON_FORMAT: &str = "%m/%d/%Y %I:%M:%S %p";

pub fn generate_acm_json(
    sha256_hash: &str,
    encrypted_key: &str,
    signature: &str,
    publickey: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &Annotations,
) -> Result<ACMJson> {
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
        device_id: "PHACM240000011".into(),
        serial_number: "CEM-AC-24000011".into(),
        station_id: "24020166".into(),
        station_name: "0651A,0652A,0670A,0673A,0674A".into(),
        event_id: election_event_id,
        event_name: election_event_name,
        sha256_hash: sha256_hash.into(),
        encrypted_key: encrypted_key.into(),
        members: vec![],
        ip_address: "192.168.1.197".into(),
        mac_address: "10:FC:B6:10:00:0B".into(),
        er_datetime: er_datetime.clone(),
        signature: signature.into(),
        publickey: publickey.into(),
        transfer_start: er_datetime.clone(),
    })
}
