// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Trait for local board storage abstraction (native/wasm)

use crate::util::ProtocolError;
use anyhow::Result;
use b4::messages::artifact::*;
use b4::messages::message::{Message, VerifiedMessage};
use b4::messages::newtypes::*;
use b4::messages::statement::{Statement, StatementType};
use b4::HttpB3Message;
use strand::context::Ctx;
use strand::hash::Hash;

// Placeholder types to match LocalBoard interface
pub struct StatementEntry {
    pub key: StatementEntryIdentifier,
    pub value: (Hash, Statement),
}

/// Key used to store statements in the statement map
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct StatementEntryIdentifier {
    pub kind: StatementType,
    pub signer_position: TrusteePosition,
    // the batch number
    pub batch: BatchNumber,
    // When storing mix signature statementents in the local board they
    // will not be unique with the above fields only.
    // (mixes themselves are, since only one mix is produced by each trustee, so the signer position
    // is sufficient; on the other hand each trustee signs _all other mixes_).
    // Without including this field in the hash key, the different signature statements
    // would be rejected as duplicates.
    pub mix_number: usize,
}

/// Convenience to return entries to the trustee for inference.
pub struct BoardEntry {
    pub key: StatementEntryIdentifier,
    pub value: (Hash, Statement),
}

/// Key used to store artifacts in the artifact map
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct ArtifactEntryIdentifier {
    pub statement_entry: StatementEntryIdentifier,
}

/// Abstraction for local board storage (native: SQLite/filesystem, wasm: IndexedDB)
pub trait LocalBoardStorage<C: Ctx> {
    /// Add a message to the board (statement + optional artifact)
    fn add(&mut self, message: VerifiedMessage, store_id: i64) -> Result<(), ProtocolError>;

    /// Get configuration hash
    fn get_cfg_hash(&self) -> Option<Hash>;

    /// Get configuration object
    fn get_configuration_raw(&self) -> Option<Configuration<C>>;

    /// Get all statement entries
    fn get_statement_entries(&self) -> Vec<BoardEntry>;

    /// Get configuration by hash
    fn get_configuration(&self, configuration_h: &ConfigurationHash) -> Option<&Configuration<C>>;

    /// Get channel artifact
    fn get_channel(
        &self,
        channel_h: &ChannelHash,
        signer_position: TrusteePosition,
    ) -> Result<Channel<C>, ProtocolError>;

    /// Get shares artifact
    fn get_shares(
        &self,
        shares_h: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError>;

    /// Get DKG public key artifact
    fn get_dkg_public_key(
        &self,
        pk_h: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError>;

    /// Get ballots artifact
    fn get_ballots(
        &self,
        b_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Ballots<C>, ProtocolError>;

    /// Get mix artifact
    fn get_mix(
        &self,
        m_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Mix<C>, ProtocolError>;

    /// Get decryption factors artifact
    fn get_decryption_factors(
        &self,
        d_h: &DecryptionFactorsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<DecryptionFactors<C>, ProtocolError>;

    /// Get plaintexts artifact
    fn get_plaintexts(
        &self,
        p_h: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Plaintexts<C>, ProtocolError>;

    /// Update store with remote messages
    fn update_store(&self, messages: &Vec<HttpB3Message>, ignore_existing: bool) -> Result<()>;

    /// Store and return new messages with local_id > last_local_board_id
    /// SECURITY: Returns messages with locally-controlled IDs from our store's AUTOINCREMENT
    fn store_and_return_messages(
        &mut self,
        messages: &Vec<HttpB3Message>,
        last_local_board_id: i64,
        ignore_existing: bool,
    ) -> Result<Vec<(Message, i64)>>;

    /// Get last external message id (OPTIMIZATION ONLY - no security implications)
    fn get_last_external_id(&mut self) -> Result<i64>;

    /// Max messages for protocol
    fn max_messages(&self) -> usize;

    /// Get DKG public key (testing)
    fn get_dkg_public_key_nohash(
        &self,
        signer_position: TrusteePosition,
    ) -> Option<DkgPublicKey<C>>;

    /// Get plaintexts (testing)
    fn get_plaintexts_nohash(
        &self,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>>;
}
