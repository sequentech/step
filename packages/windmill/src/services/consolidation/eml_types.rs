// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ACMTrustee {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publickey: Option<String>,
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLOfficialStatusDetail {
    pub official_status: String,
    pub status_date: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLHeader {
    pub transaction_id: String,
    pub issue_date: String,
    pub official_status_detail: EMLOfficialStatusDetail,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLIdentifier {
    pub id_number: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLContest {
    pub identifier: EMLIdentifier,
    pub total_votes: EMLTotalVotes,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLStatusItem {
    pub setting: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLAffiliation {
    pub identifier: EMLIdentifier,
    pub party: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLCandidate {
    pub identifier: EMLIdentifier,
    pub status_details: Vec<EMLStatusItem>,
    pub affiliation: EMLAffiliation,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLSelection {
    pub candidates: Vec<EMLCandidate>,
    pub valid_votes: i64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLCountMetric {
    pub kind: String,
    pub id: String,
    pub datum: i64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLTotalVotes {
    pub count_metrics: Vec<EMLCountMetric>,
    pub selections: Vec<EMLSelection>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLElection {
    pub identifier: EMLIdentifier,
    pub contests: Vec<EMLContest>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLCount {
    pub identifier: EMLIdentifier,
    pub elections: Vec<EMLElection>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLFile {
    pub id: String,
    pub header: EMLHeader,
    pub counts: Vec<EMLCount>,
}
