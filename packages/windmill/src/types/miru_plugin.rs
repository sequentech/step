// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
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

#[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct MiruCcsServer {
    pub name: String,
    pub tag: String,
    pub address: String,
    pub public_key_pem: String,
    pub send_logs: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruTransmissionPackageData {
    pub election_id: String,
    pub area_id: String,
    pub servers: Vec<MiruCcsServer>,
    pub documents: Vec<MiruDocument>,
    pub logs: Vec<Log>,
    pub threshold: i64,
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
