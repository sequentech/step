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
use std::net::IpAddr;
use tracing::instrument;

const ACM_JSON_FORMAT: &str = "%m/%d/%Y %I:%M:%S %p";

pub const MIRU_DEVICE_ID_VAR: &str = "MIRU_DEVICE_ID";
pub const MIRU_SERIAL_NUMBER_VAR: &str = "MIRU_SERIAL_NUMBER";
pub const MIRU_IP_ADDRESS_VAR: &str = "MIRU_IP_ADDRESS";
pub const MIRU_MAC_ADDRESS_VAR: &str = "MIRU_MAC_ADDRESS";

/// The OVCS's identity as its transmission packages name it, from the
/// deployment's configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OvcsIdentity {
    pub device_id: String,
    pub serial_number: String,
    pub ip_address: String,
    pub mac_address: String,
}

fn is_mac_address(text: &str) -> bool {
    let groups: Vec<&str> = text.split([':', '-']).collect();
    groups.len() == 6
        && groups
            .iter()
            .all(|group| group.len() == 2 && group.chars().all(|c| c.is_ascii_hexdigit()))
}

impl OvcsIdentity {
    /// From the process environment.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    /// From `lookup`, which answers a configuration variable's value. Every
    /// field is required.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let value = |name: &str| -> Result<String> {
            lookup(name)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    anyhow!("{name} is not configured: the transmission package needs the OVCS's identity")
                })
        };
        let ip_address = value(MIRU_IP_ADDRESS_VAR)?;
        ip_address
            .parse::<IpAddr>()
            .map_err(|_| anyhow!("{MIRU_IP_ADDRESS_VAR} is not an IP address: {ip_address}"))?;
        let mac_address = value(MIRU_MAC_ADDRESS_VAR)?;
        if !is_mac_address(&mac_address) {
            return Err(anyhow!(
                "{MIRU_MAC_ADDRESS_VAR} is not a MAC address: {mac_address}"
            ));
        }
        Ok(OvcsIdentity {
            device_id: value(MIRU_DEVICE_ID_VAR)?,
            serial_number: value(MIRU_SERIAL_NUMBER_VAR)?,
            ip_address,
            mac_address,
        })
    }
}

#[instrument(err)]
pub async fn get_acm_key_pair(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<EciesKeyPair> {
    let identity = OvcsIdentity::from_env()?;
    let secret_key = format!(
        "acm-key-pair-{}-{}-{tenant_id}-{election_event_id}",
        identity.device_id, identity.serial_number
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
    let identity = OvcsIdentity::from_env()?;
    Ok(ACMJson {
        device_id: identity.device_id,
        serial_number: identity.serial_number,
        station_id: area_annotations.station_id.to_string(),
        station_name: area_annotations.station_name.clone(),
        event_id: election_event_annotations.event_id.clone(),
        event_name: election_event_annotations.event_name.clone(),
        sha256_hash: sha256_hash.into(),
        encrypted_key: encrypted_key_base64.into(),
        members: server_signatures.clone(),
        ip_address: identity.ip_address,
        mac_address: identity.mac_address,
        er_datetime: er_datetime.clone(),
        signature: signature.into(),
        publickey: publickey.into(),
        transfer_start: er_datetime.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn configured(overrides: &[(&str, &str)]) -> Result<OvcsIdentity> {
        let mut values: HashMap<String, String> = [
            (MIRU_DEVICE_ID_VAR, "OVCS-0001"),
            (MIRU_SERIAL_NUMBER_VAR, "SN-2028-0001"),
            (MIRU_IP_ADDRESS_VAR, "10.0.0.7"),
            (MIRU_MAC_ADDRESS_VAR, "3C:7E:5A:89:4D:2F"),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
        for (name, value) in overrides {
            values.insert(name.to_string(), value.to_string());
        }
        OvcsIdentity::from_lookup(|name| values.get(name).cloned())
    }

    #[test]
    fn the_identity_comes_from_the_configuration() {
        assert_eq!(
            configured(&[]).unwrap(),
            OvcsIdentity {
                device_id: "OVCS-0001".to_string(),
                serial_number: "SN-2028-0001".to_string(),
                ip_address: "10.0.0.7".to_string(),
                mac_address: "3C:7E:5A:89:4D:2F".to_string(),
            }
        );
        assert_eq!(
            configured(&[
                (MIRU_IP_ADDRESS_VAR, " fd00::7 "),
                (MIRU_MAC_ADDRESS_VAR, "3c-7e-5a-89-4d-2f")
            ])
            .unwrap()
            .ip_address,
            "fd00::7"
        );
    }

    #[test]
    fn the_mac_address_is_read_under_its_own_name() {
        let refused = OvcsIdentity::from_lookup(|name| match name {
            "_MIRU_MAC_ADDRESS" => Some("3C:7E:5A:89:4D:2F".to_string()),
            MIRU_MAC_ADDRESS_VAR => None,
            _ => Some("10.0.0.7".to_string()),
        })
        .unwrap_err();
        assert!(refused
            .to_string()
            .contains("MIRU_MAC_ADDRESS is not configured"));
    }

    #[test]
    fn a_missing_or_malformed_value_is_refused_by_name() {
        for name in [
            MIRU_DEVICE_ID_VAR,
            MIRU_SERIAL_NUMBER_VAR,
            MIRU_IP_ADDRESS_VAR,
            MIRU_MAC_ADDRESS_VAR,
        ] {
            let refused = configured(&[(name, "  ")]).unwrap_err();
            assert_eq!(
                refused.to_string(),
                format!(
                    "{name} is not configured: the transmission package needs the OVCS's identity"
                )
            );
        }
        assert!(configured(&[(MIRU_IP_ADDRESS_VAR, "192.168.1")])
            .unwrap_err()
            .to_string()
            .contains("is not an IP address"));
        for mac in ["3C:7E:5A:89:4D", "3C:7E:5A:89:4D:2G", "3C7E5A894D2F"] {
            assert!(configured(&[(MIRU_MAC_ADDRESS_VAR, mac)])
                .unwrap_err()
                .to_string()
                .contains("is not a MAC address"));
        }
    }
}
