// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::types::ceremonies::Log;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruDocument {
    pub document_id: String,
    pub servers_sent_to: Vec<String>,
    pub created_at: String,
    pub signatures: Vec<MiruSignature>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruSignature {
    pub pub_key: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruTransmissionPackage {
    pub election_id: String,
    pub area_id: String,
    pub servers: Vec<String>,
    pub documents: Vec<MiruDocument>,
    pub logs: Vec<Log>,
}
