// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ACMTrustee {
    pub id: String,
    pub signature: String,
    pub publickey: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ACMJson {
    pub device_id: String,
    pub serial_number: String,
    pub station_id: String,
    pub station_name: String,
    pub event_id: String,
    pub event_name: String,
    pub sha256_hash: String,
    pub encrypted_key: String,
    pub members: Vec<ACMTrustee>,
    pub ip_address: String,
    pub mac_address: String,
    pub er_datetime: String,
    pub signature: String,
    pub publickey: String,
    pub transfer_start: String,
}
