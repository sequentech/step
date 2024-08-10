// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_types::ACMJson;

pub fn generate_acm_json(
    sha256_hash: &str,
    encrypted_key: &str,
    signature: &str,
    publickey: &str,
) -> ACMJson {
    ACMJson {
        device_id: "PHACM240000011".into(),
        serial_number: "CEM-AC-24000011".into(),
        station_id: "24020166".into(),
        station_name: "0651A,0652A,0670A,0673A,0674A".into(),
        event_id: "10".into(),
        event_name: "2024 LAB Test".into(),
        sha256_hash: sha256_hash.into(),
        encrypted_key: encrypted_key.into(),
        members: vec![],
        ip_address: "192.168.1.197".into(),
        mac_address: "10:FC:B6:10:00:0B".into(),
        er_datetime: "07/17/2024 10:48:51 AM".into(),
        signature: signature.into(),
        publickey: publickey.into(),
        transfer_start: "07/17/2024 02:24:03 PM".into(),
    }
}
