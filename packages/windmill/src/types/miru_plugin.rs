// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::types::ceremonies::Log;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruSignature {
    pub trustee_name: String,
    pub pub_key: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruServerDocument {
    pub name: String,
    pub document_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruDocument {
    pub document_id: String,
    pub transaction_id: String,
    pub servers_sent_to: Vec<MiruServerDocument>,
    pub created_at: String,
    pub signatures: Vec<MiruSignature>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruCcsServer {
    pub name: String,
    pub address: String,
    pub public_key_pem: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruTransmissionPackageData {
    pub election_id: String,
    pub area_id: String,
    pub servers: Vec<MiruCcsServer>,
    pub documents: Vec<MiruDocument>,
    pub logs: Vec<Log>,
}

pub type MiruTallySessionData = Vec<MiruTransmissionPackageData>;
