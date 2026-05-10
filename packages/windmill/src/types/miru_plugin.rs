// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! JSON payloads for Miru (SBEI) signing servers, documents, and transmission packages.

use sequent_core::types::ceremonies::Log;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use strum_macros::EnumString;

/// Cryptographic signature produced by a Miru instance for one outbound document.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruSignature {
    /// Miru user or installation identifier.
    pub sbei_miru_id: String,
    /// Public key material used to verify `signature`.
    pub pub_key: String,
    /// Signature bytes over the document hash or payload.
    pub signature: String,
    /// Fingerprint of the X.509 certificate bound to this signature.
    pub certificate_fingerprint: String,
}

/// Delivery outcome when a document was pushed to a Miru server.
#[allow(missing_docs)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum MiruServerDocumentStatus {
    SUCCESS,
    ERROR,
}

/// One Miru server's response for a document send attempt.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruServerDocument {
    /// Hostname or logical Miru node name.
    pub name: String,
    /// ISO8601/rfc3339 timestamp when the document was submitted.
    pub sent_at: String,
    /// Whether the submission succeeded on this server.
    pub status: MiruServerDocumentStatus,
}

/// Stable identifiers Miru assigns to each document format in a transmission.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruDocumentIds {
    #[serde(default)]
    /// Identifier for the EML election interchange payload.
    pub eml: String,
    /// Identifier for the compressed XZ bundle.
    pub xz: String,
    /// Identifier for the multi-format archive sent to all servers.
    pub all_servers: String,
}

/// One logical document (possibly multiple formats) and its per-server delivery trail.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruDocument {
    /// Miru-side IDs for each serialized representation.
    pub document_ids: MiruDocumentIds,
    /// End-to-end transaction id grouping related sends.
    pub transaction_id: String,
    /// Per-target-server send results for this document.
    pub servers_sent_to: Vec<MiruServerDocument>,
    /// Creation time of this record in ISO 8601 / RFC 3339 form.
    pub created_at: String,
    /// Signatures collected from Miru for this document set.
    pub signatures: Vec<MiruSignature>,
}

/// Miru CCS endpoint.
#[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct MiruCcsServer {
    /// Human-readable server label.
    pub name: String,
    /// Deployment or version tag for the node.
    pub tag: String,
    /// Network location of the Miru API.
    pub address: String,
    /// PEM-encoded public key advertised by the server.
    pub public_key_pem: String,
    /// When true, this node should receive ceremony log excerpts.
    pub send_logs: Option<bool>,
}

/// Everything needed to build or audit one Miru transmission package for an election area.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiruTransmissionPackageData {
    /// Election this package belongs to.
    pub election_id: String,
    /// Area within the election.
    pub area_id: String,
    /// Miru CCS servers for this package.
    pub servers: Vec<MiruCcsServer>,
    /// Documents and receipts exchanged with Miru for this dispatch.
    pub documents: Vec<MiruDocument>,
    /// Ceremony log lines included for external verification.
    pub logs: Vec<Log>,
    /// Cryptographic threshold parameter for the tally ceremony.
    pub threshold: i64,
}

/// Miru-side account.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct MiruSbeiUser {
    /// Username.
    pub username: String,
    /// Miru's internal user id string.
    pub miru_id: String,
    /// Role name as Miru models it (e.g. operator vs observer).
    pub miru_role: String,
    /// Display name shown in Miru UIs.
    pub miru_name: String,
    /// Election identifier Miru uses for scoping requests.
    pub miru_election_id: String,
    /// Certificate fingerprint when the user is bound to a client cert.
    pub certificate_fingerprint: Option<String>,
}

/// Ordered list of per-area transmission packages loaded for a tally session.
pub type MiruTallySessionData = Vec<MiruTransmissionPackageData>;
