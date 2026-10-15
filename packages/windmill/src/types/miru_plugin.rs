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
    /// The results CSV generated with the EML. Packages made before it have
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csv: Option<String>,
}

/// The uppercase hex SHA-256 of each stored file of a package document, as
/// the page shows it and the download log records it. The EML's is the one
/// the package's JSON carries.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MiruDocumentHashes {
    pub eml: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csv: Option<String>,
    pub all_servers: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruDocument {
    pub document_ids: MiruDocumentIds,
    pub transaction_id: String,
    pub servers_sent_to: Vec<MiruServerDocument>,
    pub created_at: String,
    pub signatures: Vec<MiruSignature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<MiruDocumentHashes>,
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct MiruCcsServer {
    pub name: String,
    pub tag: String,
    /// Where the package is sent. Empty for a destination that receives it
    /// by hand.
    #[serde(default)]
    pub address: String,
    pub public_key_pem: String,
    pub send_logs: Option<bool>,
}

/// How a destination receives its package.
#[derive(Display, Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum MiruDelivery {
    /// Sent over the network to the destination's address.
    Network,
    /// Downloaded from the Transmission page and handed over: the
    /// destination has no address.
    ManualReceipt,
}

impl MiruCcsServer {
    pub fn delivery(&self) -> MiruDelivery {
        if self.address.trim().is_empty() {
            MiruDelivery::ManualReceipt
        } else {
            MiruDelivery::Network
        }
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
    /// The OVCS public key (PEM) the package's own signature verifies with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ovcs_public_key: Option<String>,
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
