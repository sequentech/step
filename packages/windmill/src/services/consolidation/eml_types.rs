// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Serializable shapes for ACM sidecar JSON and the EML tally XML payload (Philippines COMELEC-style fields).

use serde::{Deserialize, Serialize};

/// Trustee line in ACM `members`: id, display name, and optional signature material.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ACMTrustee {
    /// Stable trustee / SBEI identifier in MIRU.
    pub id: String,
    /// Base64 or PEM signature over the payload, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Public key material for verification, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publickey: Option<String>,
    /// Human-readable trustee name.
    pub name: String,
}

/// ACM JSON metadata shipped beside encrypted election-results or audit-log zips.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ACMJson {
    /// Device identifier from station configuration.
    pub device_id: String,
    /// Hardware serial number string.
    pub serial_number: String,
    /// Precinct / station id.
    pub station_id: String,
    /// Station display name.
    pub station_name: String,
    /// Election event id (COMELEC event scope).
    pub event_id: String,
    /// Election event title.
    pub event_name: String,
    /// Uppercase hex SHA-256 of the cleartext payload (e.g. EML or logs JSON).
    pub sha256_hash: String,
    /// ECIES-wrapped symmetric key for the `.exz` blob (base64).
    pub encrypted_key: String,
    /// Co-signers / trustees for this package.
    pub members: Vec<ACMTrustee>,
    /// Reported IP of the sending station.
    pub ip_address: String,
    /// Reported MAC of the sending station.
    pub mac_address: String,
    /// Timestamp string for the election-results moment (local policy format).
    pub er_datetime: String,
    /// Station signature over the ACM JSON (or related payload).
    pub signature: String,
    /// Station public key PEM associated with `signature`.
    pub publickey: String,
    /// Transfer window start timestamp string.
    pub transfer_start: String,
}

/// EML header sub-node: official vs provisional and when that status was recorded.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLOfficialStatusDetail {
    /// Status label (e.g. official).
    pub official_status: String,
    /// Date-only or timestamp string for the status.
    pub status_date: String,
}

/// EML document header: ids, issue time, and official status.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLHeader {
    /// ACM / transaction id tying the EML to a submission.
    pub transaction_id: String,
    /// Issue datetime string in configured format.
    pub issue_date: String,
    /// Official vs provisional block.
    pub official_status_detail: EMLOfficialStatusDetail,
}

/// Generic id + name pair used throughout EML (election, contest, candidate, …).
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLIdentifier {
    /// External id number.
    pub id_number: String,
    /// Display name.
    pub name: String,
}

/// One contest block: id and aggregated vote metrics.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLContest {
    /// Contest id and title.
    pub identifier: EMLIdentifier,
    /// Per-contest totals and per-candidate breakdown.
    pub total_votes: EMLTotalVotes,
}

/// Single status flag on a candidate row (e.g. setting code).
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLStatusItem {
    /// Opaque setting key or label from source data.
    pub setting: String,
}

/// Party / organization line for a candidate.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLAffiliation {
    /// Party identifier block.
    pub identifier: EMLIdentifier,
    /// Party short name or acronym.
    pub party: String,
}

/// Candidate row under a selection with status and affiliation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLCandidate {
    /// Candidate id block.
    pub identifier: EMLIdentifier,
    /// Additional status fields from source annotations.
    pub status_details: Vec<EMLStatusItem>,
    /// Party affiliation subtree.
    pub affiliation: EMLAffiliation,
}

/// Ballot selection: one or more candidates and valid vote count for that row.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLSelection {
    /// Candidates listed for this selection (often one).
    pub candidates: Vec<EMLCandidate>,
    /// Valid votes attributed to this selection.
    pub valid_votes: i64,
}

/// Named integer metric in the contest total (over/under votes, registered voters, …).
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLCountMetric {
    /// Human-readable metric title.
    pub kind: String,
    /// Short code (e.g. `OV`, `RV`).
    pub id: String,
    /// Metric value.
    pub datum: i64,
}

/// Contest-level totals: roll-up metrics plus per-candidate selections.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLTotalVotes {
    /// Aggregate count metrics for the contest.
    pub count_metrics: Vec<EMLCountMetric>,
    /// Per-candidate (or per-selection) valid vote lines.
    pub selections: Vec<EMLSelection>,
}

/// Election subtree: identifier and its contests.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLElection {
    /// Election id block.
    pub identifier: EMLIdentifier,
    /// Contests within this election.
    pub contests: Vec<EMLContest>,
}

/// Region / aggregation node containing one or more elections (e.g. event-level count).
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLCount {
    /// Region or reporting unit identifier.
    pub identifier: EMLIdentifier,
    /// Elections reported under this count node.
    pub elections: Vec<EMLElection>,
}

/// Root EML document: id, header, and hierarchical counts.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct EMLFile {
    /// Document id (often tally session id).
    pub id: String,
    /// Standard EML header fields.
    pub header: EMLHeader,
    /// Top-level count regions.
    pub counts: Vec<EMLCount>,
}
