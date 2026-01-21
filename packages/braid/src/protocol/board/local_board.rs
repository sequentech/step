// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Universal LocalBoard implementation
//!
//! A LocalBoard is a trustee's view of a bulletin board, where by bulletin board
//! we refer to one particular board, not the entire bulletin board system.
//! As such a LocalBoard is specific to a protocol execution (session_id), referenced
//! in the configuration.
//!
//! This implementation is universal across platforms - the storage backend
//! (SQLite, IndexedDB, in-memory) is abstracted via the LocalBoardStorage trait.

use anyhow::Result;
use log::{debug, error, warn};
use std::collections::HashMap;
use strand::context::Ctx;
use strand::hash::Hash;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use b4::messages::artifact::*;
use b4::messages::message::VerifiedMessage;
use b4::messages::newtypes::*;
use b4::messages::statement::{Statement, StatementType};
use b4::HttpB3Message;

use crate::protocol::board::local_storage::LocalBoardStorage;
use crate::util::{ProtocolContext, ProtocolError};

///////////////////////////////////////////////////////////////////////////
// LocalBoard data structures
///////////////////////////////////////////////////////////////////////////

/// Key used to store statements in the statement map
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct StatementEntryIdentifier {
    pub kind: StatementType,
    pub signer_position: TrusteePosition,
    // the batch number
    pub batch: BatchNumber,
    // When storing mix signature statements in the local board they
    // will not be unique with the above fields only.
    // (mixes themselves are, since only one mix is produced by each trustee, so the signer position
    // is sufficient; on the other hand each trustee signs _all other mixes_).
    // Without including this field in the hash key, the different signature statements
    // would be rejected as duplicates.
    pub mix_number: usize,
}

/// Convenience to return entries to the trustee for inference.
#[derive(Clone)]
pub struct BoardEntry {
    pub key: StatementEntryIdentifier,
    pub value: (Hash, Statement),
}

/// Key used to store artifacts in the artifact map
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct ArtifactEntryIdentifier {
    pub statement_entry: StatementEntryIdentifier,
}

///////////////////////////////////////////////////////////////////////////
// LocalBoard implementation
///////////////////////////////////////////////////////////////////////////

/// A LocalBoard is a trustee's view of a bulletin board
///
/// Generic over:
/// - `C`: Cryptographic context (e.g., RistrettoCtx)
/// - `S`: Storage backend implementing LocalBoardStorage trait
pub struct LocalBoard<C: Ctx, S: LocalBoardStorage> {
    pub(crate) configuration: Option<Configuration<C>>,
    cfg_hash: Option<Hash>,

    // All keys contain a statement type and a sender. For multi instance predicates
    // (eg multiple decryption/mixing), they also have a batch (usize)
    //
    // We put the hash in the value so that we can detect overwrite attempt,
    // the statement hash is checked on retrieval (it's not in the key)
    pub statements: HashMap<StatementEntryIdentifier, (Hash, Statement)>,

    // Artifacts entries point to their source statement.
    // We put the hash in the value so that we can distinguish
    // between an artifact already present found and an overwrite attempt. It also
    // ensures checking that Action access to artifacts is for the matching hash
    // (coming from predicate data): the Action must provide the expected hash to
    // retrieve the artifact.
    //
    // This access to artifacts is done through specific type safe methods
    // that construct the keys to the underlying key value store, the hash is
    // checked on retrieval (it's not in the key)
    // FIXME we have lost the option of storing artifacts in the storage backend,
    // previously there was a separate field that stored row ids for artifacts in sqlite.
    pub(crate) artifacts_memory: HashMap<ArtifactEntryIdentifier, (Hash, Vec<u8>)>,

    // Storage backend (SQLite, IndexedDB, or no-op)
    // Public to allow external crates (e.g., braid-wasm) to access storage diagnostics
    pub storage: S,

    /// Tracks the last locally-controlled store ID loaded into the in-memory board.
    /// This is the local database's AUTOINCREMENT ID (or equivalent), NOT the
    /// bulletin board's external ID. Updated automatically by add().
    last_local_board_id: i64,
}

impl<C: Ctx, S: LocalBoardStorage> LocalBoard<C, S> {
    /// Construct an empty LocalBoard with the specified storage backend
    pub(crate) fn new(storage: S) -> LocalBoard<C, S> {
        tracing::info!("LocalBoard created");

        LocalBoard {
            configuration: None,
            cfg_hash: None,
            statements: HashMap::new(),
            artifacts_memory: HashMap::new(),
            storage,
            last_local_board_id: -1,
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add messages to LocalBoard
    ///////////////////////////////////////////////////////////////////////////

    /// Adds a message to the board.
    ///
    /// The _store_id parameter was historically a remnant, but is now used to track
    /// last_local_board_id (the locally-controlled storage ID: SQLite AUTOINCREMENT
    /// or IndexedDB position). This allows LocalBoard to automatically track which
    /// messages have been loaded into memory.
    pub(crate) fn add(
        &mut self,
        message: VerifiedMessage,
        local_id: i64,
    ) -> Result<(), ProtocolError> {
        let result = if message.statement.get_kind() == StatementType::Configuration {
            self.add_bootstrap(message)
        } else {
            self.add_message(message)
        };

        // Update tracking: this message with local_id has been loaded into memory
        if result.is_ok() && local_id > self.last_local_board_id {
            self.last_local_board_id = local_id;
        }

        result
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add bootstrap configuration
    //
    // The bootstrap configuration is not stored as a parameter/artifact, but directly
    // in the board struct fields.
    ///////////////////////////////////////////////////////////////////////////

    /// Bootstraps the board with a configuration message
    ///
    /// If the board has already been initialized the incoming
    /// message will be ignored if it's identical to the existing
    /// configuration. Otherwise an error will be raised.
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

    ///////////////////////////////////////////////////////////////////////////
    // All other statements
    //
    // Other statements, including _signed_ configuration
    ///////////////////////////////////////////////////////////////////////////

    /// Adds a non-bootstrap (not the configuration) message to the board.
    ///
    /// All messages that are not the configuration are added this way,
    /// including configuration signatures. Messages can be stand alone
    /// statements, or statements plus a binary artifact.
    ///
    /// If a statement that already existed in the board is received it
    /// will be ignored if it is identical. Otherwise an error will be raised.
    /// If an artifact that already existed in the board is received the
    /// artifact and the statement will be ignored.
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

    /// Returns the configuration hash.
    ///
    /// Used by the trustee for sanity checks.
    pub(crate) fn get_cfg_hash(&self) -> Option<Hash> {
        self.cfg_hash
    }

    /// Returns the configuration.
    ///
    /// Used by the trustee for sanity checks as well
    /// as for deriving the configuration predicate for
    /// datalog.
    pub(crate) fn get_configuration_raw(&self) -> Option<Configuration<C>> {
        self.configuration.clone()
    }

    /// Returns all the statement entries.
    ///
    /// Used by the trustee to derive all the datalog predicates.
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

    /// Gets the Configuration, with a hash check
    ///
    /// If the configuration does not exist, or the supplied hash does not match
    /// returns None. The trustee version of this function raises an error instead.
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

    /// Gets a Channel, with a hash check.
    pub(crate) fn get_channel(
        &self,
        channel_h: &ChannelHash,
        signer_position: TrusteePosition,
    ) -> Result<Channel<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::Channel, channel_h.0, signer_position)?;
        Ok(Channel::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets a Share, with a hash check.
    pub(crate) fn get_shares(
        &self,
        shares_h: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::Shares, shares_h.0, signer_position)?;
        Ok(Shares::strand_deserialize(&bytes)?)
    }

    /// Gets the DkgPublicKey, with a hash check.
    pub(crate) fn get_dkg_public_key(
        &self,
        pk_h: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::PublicKey, pk_h.0, signer_position)?;
        Ok(DkgPublicKey::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets Ballots, with a hash check.
    pub(crate) fn get_ballots(
        &self,
        b_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Ballots<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Ballots, b_h.0, signer_position, batch)?;
        Ok(Ballots::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets a Mix, with a hash check.
    pub(crate) fn get_mix(
        &self,
        m_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Mix<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Mix, m_h.0, signer_position, batch)?;
        Ok(Mix::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets DecryptionFactors, with a hash check.
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

    /// Gets Plaintexts, with a hash check.
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
    // Artifact retrieval (always from memory)
    //////////////////////////////////////////////////////////////////////////

    /// Returns a dkg artifact bytes from memory, with hash check.
    ///
    /// Dkg artifacts have their batch and mixnumber set to 0.
    fn get_dkg_artifact(
        &self,
        kind: StatementType,
        hash: Hash,
        signer_position: TrusteePosition,
    ) -> Result<&Vec<u8>, ProtocolError> {
        self.get_artifact(kind, hash, signer_position, 0)
    }

    /// Returns an artifact bytes from memory, with hash check.
    ///
    /// All artifacts have their mix number set to 0. Only mix signature
    /// statements are keyed (in the hashmap) with a mix number != 0.
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

    /// Constructs statement entry keys.
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

    /// Constructs artifact entry keys from a statement entry key.
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

    /// Constructs artifact entry keys.
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
    // Storage interaction methods
    ///////////////////////////////////////////////////////////////////////////

    /// Updates the message store with the supplied remote messages
    pub(crate) fn update_store(
        &self,
        messages: &[HttpB3Message],
        ignore_existing: bool,
    ) -> Result<()> {
        self.storage.store_messages(messages, ignore_existing)
    }

    /// Returns the last locally-controlled store ID loaded into this board.
    ///
    /// This is NOT the bulletin board's external ID - it's our local database ID
    /// (SQLite AUTOINCREMENT or IndexedDB position).
    pub(crate) fn get_last_local_board_id(&self) -> i64 {
        self.last_local_board_id
    }

    /// Updates the message store and returns messages not yet in the board.
    ///
    /// Called as part of the normal step update sequence
    /// 1) Retrieve remote messages
    /// 2) Store them in the message store (assigning locally-controlled IDs)
    /// 3) Return messages with local_id > last_local_board_id for loading into memory
    ///
    /// SECURITY: Uses locally-controlled AUTOINCREMENT IDs (not bulletin board IDs)
    /// to ensure append-only, tamper-proof message ordering.
    pub(crate) fn store_and_return_messages(
        &mut self,
        messages: &[HttpB3Message],
        ignore_existing: bool,
    ) -> Result<Vec<(b4::messages::message::Message, i64)>> {
        self.storage.store_messages(messages, ignore_existing)?;
        self.storage.retrieve_messages(self.last_local_board_id)
    }

    /// Returns the largest external_id stored in the message store.
    ///
    /// OPTIMIZATION ONLY: Has NO security implications.
    pub(crate) fn get_last_external_id(&self) -> Result<i64> {
        self.storage.get_last_external_id()
    }

    /// The maximum number of messages this protocol will generate.
    pub(crate) fn max_messages(&self) -> usize {
        let Some(cfg) = &self.configuration else {
            return 0;
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
}
