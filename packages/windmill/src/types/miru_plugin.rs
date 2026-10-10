// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::types::ceremonies::Log;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use strum_macros::EnumString;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruSignature {
    pub sbei_miru_id: String,
    pub pub_key: String,
    pub signature: String,
    pub certificate_fingerprint: String,
}

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum MiruServerDocumentStatus {
    SUCCESS,
    ERROR,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruServerDocument {
    pub name: String,
    pub sent_at: String, // date using ISO8601/rfc3339
    pub status: MiruServerDocumentStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruDocumentIds {
    #[serde(default)]
    pub eml: String,
    pub xz: String,
    pub all_servers: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruDocument {
    pub document_ids: MiruDocumentIds,
    pub transaction_id: String,
    pub servers_sent_to: Vec<MiruServerDocument>,
    pub created_at: String,
    pub signatures: Vec<MiruSignature>,
}

#[derive(
    Display, EnumString, Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone, Copy,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CcsTlsVerificationPolicy {
    #[default]
    Verify,
    AcceptInvalidCertificates,
}

impl CcsTlsVerificationPolicy {
    pub fn accepts_invalid_certificates(&self) -> bool {
        match self {
            CcsTlsVerificationPolicy::Verify => false,
            CcsTlsVerificationPolicy::AcceptInvalidCertificates => true,
        }
    }
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct MiruCcsServer {
    pub name: String,
    pub tag: String,
    pub address: String,
    pub public_key_pem: String,
    pub send_logs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_verification_policy: Option<CcsTlsVerificationPolicy>,
}

impl MiruCcsServer {
    pub fn tls_verification_policy(&self) -> CcsTlsVerificationPolicy {
        self.tls_verification_policy.unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruTransmissionPackageData {
    pub election_id: String,
    pub area_id: String,
    pub servers: Vec<MiruCcsServer>,
    pub documents: Vec<MiruDocument>,
    pub logs: Vec<Log>,
    pub threshold: i64,
    /// The request whose signatures the package waits for, when the event's
    /// transmit-results rule needs them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_request: Option<MiruSigningRequest>,
}

/// The signing request of a transmission package.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MiruSigningRequest {
    pub id: String,
    pub code: String,
    pub required: i32,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct MiruSbeiUser {
    pub username: String,
    pub miru_id: String,
    pub miru_role: String,
    pub miru_name: String,
    pub miru_election_id: String,
    pub certificate_fingerprint: Option<String>,
}

pub type MiruTallySessionData = Vec<MiruTransmissionPackageData>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const CCS_SERVER_WITHOUT_POLICY: &str = r#"{
        "name": "ccs",
        "tag": "ccs-tag",
        "address": "https://ccs.example.com",
        "public_key_pem": "pem",
        "send_logs": true
    }"#;

    fn ccs_server_with_policy(policy: &str) -> String {
        format!(
            r#"{{
                "name": "ccs",
                "tag": "ccs-tag",
                "address": "https://ccs.example.com",
                "public_key_pem": "pem",
                "tls_verification_policy": "{policy}"
            }}"#
        )
    }

    #[test]
    fn ccs_server_without_policy_verifies_certificates() {
        let server: MiruCcsServer = serde_json::from_str(CCS_SERVER_WITHOUT_POLICY).unwrap();
        assert_eq!(server.tls_verification_policy, None);
        assert_eq!(
            server.tls_verification_policy(),
            CcsTlsVerificationPolicy::Verify
        );
        assert!(!server
            .tls_verification_policy()
            .accepts_invalid_certificates());
    }

    #[test]
    fn ccs_server_explicit_verify_policy_verifies_certificates() {
        let server: MiruCcsServer =
            serde_json::from_str(&ccs_server_with_policy("verify")).unwrap();
        assert!(!server
            .tls_verification_policy()
            .accepts_invalid_certificates());
    }

    #[test]
    fn ccs_server_accept_invalid_certificates_policy_is_honored() {
        let server: MiruCcsServer =
            serde_json::from_str(&ccs_server_with_policy("accept_invalid_certificates")).unwrap();
        assert_eq!(
            server.tls_verification_policy(),
            CcsTlsVerificationPolicy::AcceptInvalidCertificates
        );
        assert!(server
            .tls_verification_policy()
            .accepts_invalid_certificates());
    }

    #[test]
    fn ccs_server_invalid_policy_is_rejected() {
        assert!(
            serde_json::from_str::<MiruCcsServer>(&ccs_server_with_policy("insecure")).is_err()
        );
        assert!(CcsTlsVerificationPolicy::from_str("insecure").is_err());
    }

    #[test]
    fn ccs_tls_verification_policy_round_trips_through_strings() {
        for policy in [
            CcsTlsVerificationPolicy::Verify,
            CcsTlsVerificationPolicy::AcceptInvalidCertificates,
        ] {
            assert_eq!(
                CcsTlsVerificationPolicy::from_str(&policy.to_string()).unwrap(),
                policy
            );
        }
    }

    #[test]
    fn ccs_server_without_policy_serializes_without_the_field() {
        let server: MiruCcsServer = serde_json::from_str(CCS_SERVER_WITHOUT_POLICY).unwrap();
        let value = serde_json::to_value(&server).unwrap();
        assert!(value.get("tls_verification_policy").is_none());
    }
}
