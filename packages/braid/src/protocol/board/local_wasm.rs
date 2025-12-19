// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM-compatible local board implementation
//!
//! # Current Status: In-Memory Only
//!
//! This implementation currently provides in-memory storage only. IndexedDB persistence
//! is prepared but not yet integrated due to async/sync impedance mismatch.
//!
//! # Future: IndexedDB Persistence (Requires Async Refactor)
//!
//! To implement secure persistent storage in WASM, we need:
//!
//! ## Security Requirements:
//! - **Append-only**: Messages assigned auto-incrementing local IDs, cannot be deleted
//! - **Tamper-resistant**: Uniqueness constraints prevent duplicate messages
//! - **Replay protection**: Track last_local_board_id persistently
//! - **Locally-controlled ordering**: Local IDs determine order, not bulletin board
//!
//! ## IndexedDB Schema (Prepared):
//! - **messages** object store with auto-increment key (local_id)
//! - **Indexes**:
//!   - `external_id` (unique) - bulletin board's ID for optimization
//!   - `message_key` (unique) - composite key: sender_pk + kind + batch + mix_number
//!
//! ## Implementation Blocker:
//! IndexedDB is inherently async, but the LocalBoardStorage trait requires sync methods.
//! Options:
//! 1. Make Trustee::step() async in WASM (requires braid-wasm refactor)
//! 2. Use wasm-bindgen-futures with a custom executor (complex, may block UI)
//! 3. Hybrid: cache in memory, persist async in background (eventual consistency issues)
//!
//! **Recommended**: Option 1 - make WASM trustee async throughout.
//! This is the cleanest approach but requires refactoring braid-wasm::Trustee.

use crate::protocol::board::local::{
    ArtifactEntryIdentifier, BoardEntry, StatementEntryIdentifier,
};
use crate::util::{ProtocolContext, ProtocolError};
use anyhow::{anyhow, Result};
use b4::messages::artifact::*;
use b4::messages::message::{Message, VerifiedMessage};
use b4::messages::newtypes::*;
use b4::messages::statement::{Statement, StatementType};
use b4::HttpB3Message;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::path::PathBuf;
use strand::context::Ctx;
use strand::hash::Hash;
use strand::serialization::{StrandDeserialize, StrandSerialize};

/// A WASM-compatible local board - currently in-memory only.
///
/// NOTE: Persistent IndexedDB storage is architecturally ready but requires
/// making the Trustee async in WASM. See module documentation for details.
pub struct LocalBoard<C: Ctx> {
    pub(crate) configuration: Option<Configuration<C>>,
    cfg_hash: Option<Hash>,
    // Public for external crates (e.g., braid-wasm) to access statement count
    pub statements: HashMap<StatementEntryIdentifier, (Hash, Statement)>,
    pub(crate) store: Option<PathBuf>,
    pub(crate) artifacts_memory: HashMap<ArtifactEntryIdentifier, (Hash, Vec<u8>)>,
}

impl<C: Ctx> LocalBoard<C> {
    /// Construct an empty LocalBoard with in-memory storage only
    pub(crate) fn new(_store: Option<PathBuf>, _blob_store: Option<PathBuf>) -> Self {
        tracing::info!(
            "WASM LocalBoard: in-memory only (IndexedDB persistence requires async refactor)"
        );

        LocalBoard {
            configuration: None,
            cfg_hash: None,
            statements: HashMap::new(),
            store: None, // Persistence disabled until async refactor
            artifacts_memory: HashMap::new(),
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add messages to LocalBoard
    ///////////////////////////////////////////////////////////////////////////

    /// Adds a message to the board.
    pub(crate) fn add(
        &mut self,
        message: VerifiedMessage,
        _store_id: i64,
    ) -> Result<(), ProtocolError> {
        if message.statement.get_kind() == StatementType::Configuration {
            self.add_bootstrap(message)
        } else {
            self.add_message(message)
        }
    }

    /// Bootstraps the board with a configuration message
    fn add_bootstrap(&mut self, message: VerifiedMessage) -> Result<(), ProtocolError> {
        let cfg_hash = message.statement.get_cfg_h();

        if self.configuration.is_none() {
            let artifact_bytes =
                &message
                    .artifact
                    .ok_or(ProtocolError::BootstrapError(format!(
                        "Missing artifact in configuration message"
                    )))?;

            let configuration = Configuration::<C>::strand_deserialize(artifact_bytes);

            if let Ok(configuration) = configuration {
                self.configuration = Some(configuration);
                self.cfg_hash = Some(cfg_hash);

                return Ok(());
            } else {
                error!(
                    "Failed deserializing configuration {:?}, ignored",
                    configuration
                );
                return Err(configuration
                    .add_context("Bootstrapping, deserializing configuration")
                    .err()
                    .expect("impossible"));
            }
        }

        let message_hash = self
            .cfg_hash
            .expect("unexpected: cfg_hash always exists when cfg exists");

        if message_hash == cfg_hash {
            warn!("Configuration received when identical present, ignored");
            Ok(())
        } else {
            Err(ProtocolError::BoardOverwriteAttempt(format!(
                "Configuration"
            )))
        }
    }

    /// Adds a non-bootstrap message to the board.
    fn add_message(&mut self, message: VerifiedMessage) -> Result<(), ProtocolError> {
        let bytes = message.statement.strand_serialize()?;
        let statement_hash = strand::hash::hash(&bytes)?;

        let statement_identifier =
            self.get_statement_entry_identifier(&message.statement, message.signer_position);
        let statement_entry = self.statements.get(&statement_identifier);

        if let Some((existing_hash, _)) = statement_entry {
            if statement_hash == existing_hash {
                debug!(
                    "Statement identifier already exists (identical): {:?}",
                    statement_identifier
                );
                Ok(())
            } else {
                Err(ProtocolError::BoardOverwriteAttempt(format!(
                    "Statement identifier already exists (overwrite): {:?}, message was {:?}",
                    statement_identifier, message
                )))
            }
        } else {
            debug!(
                "Statement identifier is new: {:?}",
                statement_identifier.kind
            );

            // The statement is new, we check the artifact
            if let Some(artifact) = message.artifact {
                let artifact_identifier = self.get_artifact_entry_identifier(&statement_identifier);
                let artifact_hash = strand::hash::hash_to_array(&artifact)?;

                let artifact_entry = self.artifacts_memory.get(&artifact_identifier);

                if let Some((existing_hash, _)) = artifact_entry {
                    if artifact_hash == *existing_hash {
                        warn!("Artifact identical, ignored");
                        Ok(())
                    } else {
                        Err(ProtocolError::BoardOverwriteAttempt(format!(
                            "Artifact {}",
                            statement_identifier.kind
                        )))
                    }
                } else {
                    debug!(
                        "Artifact identifier is new: {:?}",
                        artifact_identifier.statement_entry.kind
                    );

                    // Both statement and artifact are new, insert into memory
                    self.statements.insert(
                        statement_identifier,
                        (
                            crate::util::hash_from_vec(&statement_hash)?,
                            message.statement,
                        ),
                    );

                    self.artifacts_memory
                        .insert(artifact_identifier, (artifact_hash, artifact));

                    debug!("Artifact inserted into memory");

                    Ok(())
                }
            } else {
                // Only a statement was sent, insert
                self.statements.insert(
                    statement_identifier,
                    (
                        crate::util::hash_from_vec(&statement_hash)?,
                        message.statement,
                    ),
                );
                debug!("Pure statement inserted");
                Ok(())
            }
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Raw accessors for Trustee
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn get_cfg_hash(&self) -> Option<Hash> {
        self.cfg_hash
    }

    pub(crate) fn get_configuration_raw(&self) -> Option<Configuration<C>> {
        self.configuration.clone()
    }

    // Public for external crates (e.g., braid-wasm) to get board summary
    pub fn get_statement_entries(&self) -> Vec<BoardEntry> {
        self.statements
            .iter()
            .map(|(k, v)| BoardEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }

    ///////////////////////////////////////////////////////////////////////////
    // Artifact accessors for Actions (forwarded from Trustee)
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn get_configuration(
        &self,
        configuration_h: &ConfigurationHash,
    ) -> Option<&Configuration<C>> {
        if let Some(h) = self.cfg_hash {
            if let Some(cfg) = &self.configuration {
                if h == configuration_h.0 {
                    return Some(cfg);
                } else {
                    warn!("Configuration hash mismatch");
                }
            }
        }
        warn!("Was unable to retrieve configuration");
        None
    }

    pub(crate) fn get_channel(
        &self,
        channel_h: &ChannelHash,
        signer_position: TrusteePosition,
    ) -> Result<Channel<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::Channel, channel_h.0, signer_position)?;
        Ok(Channel::<C>::strand_deserialize(&bytes)?)
    }

    pub(crate) fn get_shares(
        &self,
        shares_h: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::Shares, shares_h.0, signer_position)?;
        Ok(Shares::strand_deserialize(&bytes)?)
    }

    pub(crate) fn get_dkg_public_key(
        &self,
        pk_h: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::PublicKey, pk_h.0, signer_position)?;
        Ok(DkgPublicKey::<C>::strand_deserialize(&bytes)?)
    }

    pub(crate) fn get_ballots(
        &self,
        b_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Ballots<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Ballots, b_h.0, signer_position, batch)?;
        Ok(Ballots::<C>::strand_deserialize(&bytes)?)
    }

    pub(crate) fn get_mix(
        &self,
        m_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Mix<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Mix, m_h.0, signer_position, batch)?;
        Ok(Mix::<C>::strand_deserialize(&bytes)?)
    }

    pub(crate) fn get_decryption_factors(
        &self,
        d_h: &DecryptionFactorsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<DecryptionFactors<C>, ProtocolError> {
        let bytes = self.get_artifact(
            StatementType::DecryptionFactors,
            d_h.0,
            signer_position,
            batch,
        )?;
        Ok(DecryptionFactors::<C>::strand_deserialize(&bytes)?)
    }

    pub(crate) fn get_plaintexts(
        &self,
        p_h: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Plaintexts<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Plaintexts, p_h.0, signer_position, batch)?;
        Ok(Plaintexts::<C>::strand_deserialize(&bytes)?)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Artifact retrieval (in-memory only)
    //////////////////////////////////////////////////////////////////////////

    fn get_dkg_artifact(
        &self,
        kind: StatementType,
        hash: Hash,
        signer_position: TrusteePosition,
    ) -> Result<&Vec<u8>, ProtocolError> {
        self.get_artifact(kind, hash, signer_position, 0)
    }

    fn get_artifact(
        &self,
        kind: StatementType,
        hash: Hash,
        signer_position: TrusteePosition,
        batch: BatchNumber,
    ) -> Result<&Vec<u8>, ProtocolError> {
        let aei = self.get_artifact_entry_identifier_ext(kind.clone(), signer_position, batch, 0);

        let entry = self
            .artifacts_memory
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(kind.clone()))?;

        if hash != entry.0 {
            return Err(ProtocolError::MismatchedArtifactHash(kind));
        }

        Ok(&entry.1)
    }

    ///////////////////////////////////////////////////////////////////////////
    // LocalBoard key construction
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn get_statement_entry_identifier(
        &self,
        statement: &Statement,
        signer_position: usize,
    ) -> StatementEntryIdentifier {
        let (kind, _, batch, mix_number, _) = statement.get_data();

        StatementEntryIdentifier {
            kind,
            signer_position,
            batch,
            mix_number,
        }
    }

    pub(crate) fn get_artifact_entry_identifier(
        &self,
        statement_entry: &StatementEntryIdentifier,
    ) -> ArtifactEntryIdentifier {
        self.get_artifact_entry_identifier_ext(
            statement_entry.kind.clone(),
            statement_entry.signer_position,
            statement_entry.batch,
            statement_entry.mix_number,
        )
    }

    pub(crate) fn get_artifact_entry_identifier_ext(
        &self,
        statement_type: StatementType,
        signer_position: usize,
        batch: BatchNumber,
        mix_number: usize,
    ) -> ArtifactEntryIdentifier {
        let sti = StatementEntryIdentifier {
            kind: statement_type,
            signer_position,
            batch,
            mix_number,
        };
        ArtifactEntryIdentifier {
            statement_entry: sti,
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Additional methods required by Trustee
    ///////////////////////////////////////////////////////////////////////////

    // Public for external crates (e.g., braid-wasm) to track protocol progress
    pub fn max_messages(&self) -> usize {
        let Some(cfg) = &self.configuration else {
            return 1;
        };

        let mut sei = StatementEntryIdentifier {
            kind: StatementType::Ballots,
            signer_position: PROTOCOL_MANAGER_INDEX,
            batch: 0,
            mix_number: 0,
        };

        loop {
            if self.statements.get(&sei).is_none() {
                break;
            }
            sei.batch = sei.batch + 1;
        }

        let n = cfg.trustees.len();
        let t = cfg.threshold;

        let dkg = 1 + (5 * n);

        let per_batch_tally = 1 + (2 * t) + (t * (t - 1)) + n;

        dkg + ((sei.batch as usize) * per_batch_tally)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Testing functions (used by tests and dbg)
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn get_dkg_public_key_nohash(
        &self,
        signer_position: TrusteePosition,
    ) -> Option<DkgPublicKey<C>> {
        let aei =
            self.get_artifact_entry_identifier_ext(StatementType::PublicKey, signer_position, 0, 0);
        let entry = self.artifacts_memory.get(&aei)?;

        DkgPublicKey::<C>::strand_deserialize(&entry.1).ok()
    }

    pub(crate) fn get_plaintexts_nohash(
        &self,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Plaintexts,
            signer_position,
            batch,
            0,
        );
        let entry = self.artifacts_memory.get(&aei)?;

        Plaintexts::<C>::strand_deserialize(&entry.1).ok()
    }

    ///////////////////////////////////////////////////////////////////////////
    ///////////////////////////////////////////////////////////////////////////
    // Store methods - Currently stubs, pending async refactor for IndexedDB
    ///////////////////////////////////////////////////////////////////////////

    /// Not yet implemented: IndexedDB persistence requires async refactor
    ///
    /// TODO: Once Trustee::step() is async in WASM, implement:
    /// - Open IndexedDB for this board
    /// - Store messages with auto-increment local_id
    /// - Query messages WHERE local_id > last_local_board_id ORDER BY local_id ASC
    /// - Return Vec<(Message, local_id)>
    pub(crate) fn store_and_return_messages(
        &mut self,
        _messages: &Vec<HttpB3Message>,
        _last_local_board_id: i64,
        _ignore_existing: bool,
    ) -> Result<Vec<(Message, i64)>> {
        // WASM has no persistence yet - messages processed directly via step()
        Ok(vec![])
    }

    /// Not yet implemented: IndexedDB persistence requires async refactor
    ///
    /// TODO: Once async, implement:
    /// - Deserialize messages to extract metadata
    /// - Store in IndexedDB with uniqueness constraints
    /// - Use ignore_existing to handle duplicates during full refresh
    pub(crate) fn update_store(
        &self,
        _messages: &Vec<HttpB3Message>,
        _ignore_existing: bool,
    ) -> Result<()> {
        Ok(())
    }

    /// Not yet implemented: IndexedDB persistence requires async refactor
    ///
    /// TODO: Once async, implement:
    /// - Query IndexedDB for MAX(external_id)
    /// - Return max or -1 if empty
    /// Note: This is OPTIMIZATION ONLY, has no security implications
    pub(crate) fn get_last_external_id(&mut self) -> Result<i64> {
        Ok(-1)
    }
}
