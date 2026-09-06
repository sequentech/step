// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use log::{debug, error, info, trace, warn};
// #[cfg(feature = "native")]
use rayon::prelude::*;
use std::collections::HashSet;
use tracing_attributes::instrument;

use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::{context::Ctx, elgamal::PrivateKey};

use crate::protocol::action::Action;
use crate::protocol::board::{LocalBoard, LocalBoardStorage};
use crate::protocol::predicate::Predicate;

use crate::util::{ProtocolContext, ProtocolError};
use b4::messages::artifact::Channel;
use b4::messages::artifact::Configuration;
use b4::messages::artifact::DkgPublicKey;
use b4::messages::artifact::Shares;
use b4::messages::artifact::{Ballots, DecryptionFactors, Mix, Plaintexts};
use b4::messages::message::Message;
use b4::messages::newtypes::*;
use b4::messages::statement::StatementType;
use b4::HttpB3Message;
use strand::util::StrandError;

use strand::symm::{self, EncryptionData};

const RETRIEVE_ALL_MESSAGES_PERIOD: i64 = 60 * 60;

/// A session-specific protocol trustee
///
/// Represents the instantiation of a trustee within a specific protocol
/// session. Runs the main loop for the trustee's participation in the session.
/// Contains a LocalBoard, a view of the bulletin board.
///
/// # Protocol Flow
///
/// 1) Receive remote messages from the bulletin board
/// 2) Update LocalBoard with Statements and Artifacts
/// 3) Derive Predicates from Statements on LocalBoard
/// 4) Invoke Datalog with input predicates
///    4.1) Pass output predicates from 4) to subsequent datalog Phases
/// 5) Run Actions resulting from 4)
/// 6) Return resulting Messages for subsequent posting on RemoteBoard
///
/// Does not post the messages itself.
///
/// # Security Model: Message ID Tracking
///
/// The trustee uses two distinct ID tracking systems for messages:
///
/// ## 1. Local Board ID (last_local_board_id) - SECURITY CRITICAL
///
/// - **Source**: SQLite AUTOINCREMENT PRIMARY KEY - controlled locally by the trustee's database
/// - **Purpose**: Tracks which messages have been loaded into the in-memory local board (Rust HashMaps)
/// - **Security Property**: Because this ID is assigned by the local SQLite database using AUTOINCREMENT,
///   the insertion order is immutable and locally controlled
/// - **Usage**: `WHERE id > ?1 ORDER BY id ASC` ensures messages are loaded into memory in the exact
///   order they were first stored locally
/// - **Critical Guarantee**: The bulletin board CANNOT manipulate this ordering - it's determined by
///   when messages first hit the local persistent store
///
/// ## 2. External ID (from get_last_external_id()) - OPTIMIZATION ONLY
///
/// - **Source**: The bulletin board's own ID system (stored in external_id column)
/// - **Purpose**: Optimization - tells the bulletin board "give me messages with ID > X" to avoid re-fetching
/// - **Security Property**: NONE - this is purely for efficiency, not security
/// - **Usage**: Returned by get_last_external_id() to request only new messages from the remote board
/// - **No Security Impact**: Even if the bulletin board lies about IDs or reorders them, the local
///   AUTOINCREMENT id ensures correct processing order
///
/// ## The Security Guarantee
///
/// The key insight is that the local database is the source of truth for message ordering:
///
/// 1. Fetch messages from bulletin board (using external_id for efficiency)
/// 2. Store them in local SQLite - each gets a new AUTOINCREMENT id on first insert
/// 3. `UNIQUE(sender_pk, statement_kind, batch, mix_number)` prevents duplicates
/// 4. Later cycles load messages from store into memory using `id > last_local_board_id ORDER BY id ASC`
///
/// This ensures append-only, locally-determined ordering regardless of bulletin board behavior.
///
/// The signing_key_sk is used to sign signatures
/// which are verified by the signing_key_pk
/// on the receiving end.
///
/// The encryption_key is used to symmetrically
/// encrypt elgamal private keys used for
/// trustees to send Shares messages privately.
/// This information is published on the bulletin board
/// in Channel objects.
///
/// The max_concurrent_actions value determines the maximum number
/// of actions that can be executed in parallel. Higher
/// values may increase core utilization, but also
/// peak memory usage.
pub struct Trustee<C: Ctx, S: LocalBoardStorage> {
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) board_name: String,
    pub(crate) signing_key: StrandSignatureSk,
    pub(crate) encryption_key: symm::SymmetricKey,
    // Public for external crates (e.g., braid-wasm) to access board state
    pub local_board: LocalBoard<C, S>,
    pub(crate) step_counter: i64,
    pub(crate) max_concurrent_actions: Option<usize>,
}

impl<C: Ctx, S: LocalBoardStorage> Trustee<C, S> {
    /// Constructs a trustee instance.
    ///
    /// A trustee instance exists in the context of a session, and therefore
    /// specific board.
    pub fn new(
        name: String,
        board_name: String,
        signing_key: StrandSignatureSk,
        encryption_key: symm::SymmetricKey,
        storage: S,
        max_concurrent_actions: Option<usize>,
    ) -> Trustee<C, S> {
        // let max_concurrent_actions = 10;

        info!(
            "Trustee {} created, max_concurrent_actions = {:?}",
            name, max_concurrent_actions
        );

        let local_board = LocalBoard::new(storage);

        Trustee {
            name,
            board_name,
            signing_key,
            encryption_key,
            local_board,
            step_counter: 0,
            max_concurrent_actions,
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Protocol step: update->derive predicates->infer&run
    ///////////////////////////////////////////////////////////////////////////

    /// Executes one step of the protocol main loop.
    ///
    /// Typically: update store => update board => derive predicates => infer actions
    /// run actions => return resulting messages.
    ///
    /// The protocol main loop is reactive: it will not advance until the necessary
    /// messages are present in the board.
    #[instrument(name = "Trustee::step", skip(remote_messages, self), level = "trace")]
    pub fn step(&mut self, remote_messages: &[HttpB3Message]) -> Result<StepResult, ProtocolError> {
        // When retrieving all messages, some of them will already exist in
        // the local store: this is not an error
        let ignore_existing = self.step_counter % RETRIEVE_ALL_MESSAGES_PERIOD == 0;
        // Store messages and retrieve them with IDs (persistent: local IDs, no-op: external IDs)
        let parsed_messages = self
            .local_board
            .store_and_return_messages(remote_messages, ignore_existing)
            .map_err(|e| ProtocolError::BoardError(format!("{}", e)))?;

        // Update the in-memory board with parsed messages. Returns count of added messages.
        let added_messages = self.update_local_board(parsed_messages)?;
        if added_messages > 0 {
            let max_messages = self.local_board.max_messages();
            let last_id = self.local_board.get_last_local_board_id();
            info!(
                "Loaded {} messages, last local board id {} (/{})!",
                added_messages, last_id, max_messages
            );
        }

        trace!("Update added {} messages", added_messages);
        let predicates = self.derive_predicates(false)?;
        trace!("Derived {} predicates", predicates.len());
        let (messages, actions) = self.infer_and_run_actions(&predicates, false)?;

        // Sanity check: ensure that all outgoing messages' cfg field matches that of the local board
        for m in messages.iter() {
            info!("Returning message {:?}", m);
            assert_eq!(
                m.statement.get_cfg_h(),
                self.local_board
                    .get_cfg_hash()
                    .expect("cfg hash should always be present once the trustee is posting")
            );
        }

        let last_id = self.local_board.get_last_local_board_id();
        let ret = StepResult::new(messages, actions, added_messages, last_id);

        Ok(ret)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Update
    ///////////////////////////////////////////////////////////////////////////

    /// Updates the message store only, not the local board.
    ///
    /// Used when the remote bulletin board returns a truncated response
    /// indicating that a further request must be made before inferring any
    /// new Actions.
    pub(crate) fn update_store(&self, messages: &[HttpB3Message]) -> Result<(), ProtocolError> {
        self.local_board
            .update_store(messages, false)
            .map_err(|e| ProtocolError::BoardError(format!("{}", e)))
    }

    /// Returns the largest external_id stored in the local message store
    ///
    /// OPTIMIZATION ONLY: This is used to request messages from the bulletin board
    /// with external_id > last_external_id, avoiding redundant fetches.
    ///
    /// SECURITY: This external ID has NO security implications. The bulletin board
    /// controls these IDs, but our local AUTOINCREMENT IDs control message ordering.
    /// Even if the bulletin board lies about or reorders external IDs, our local
    /// store ensures append-only, locally-determined ordering.
    ///
    /// Every RETRIEVE_ALL_MESSAGES_PERIOD a full refresh will be triggered,
    /// where all messages will be requested from the bulletin board.
    /// This is not a reset, local messages will persist and cannot
    /// be overriden. Rather it is a defensive mechanism against unknown errors
    /// that may cause an incomplete local view of the bulletin board
    /// and cause the protocol to get stuck.
    pub fn get_last_external_id(&mut self) -> Result<i64, ProtocolError> {
        self.step_counter = (self.step_counter + 1) % i64::MAX;
        // in the event that there are holes, a full update will eventually load missing
        // messages from the remote board
        let refresh = self.step_counter % RETRIEVE_ALL_MESSAGES_PERIOD == 0;
        let external_last_id = if refresh {
            info!(
                "* Full update from remote board (step = {})",
                self.step_counter
            );
            -1
        } else {
            self.local_board.get_last_external_id().unwrap_or(-1)
        };

        Ok(external_last_id)
    }

    /// Updates the LocalBoard, inserting new messages into the statements and
    /// artifact maps.
    ///
    /// Takes a vector of (message, local_id) pairs as input, returns count of added messages.
    ///
    /// SECURITY: The local_id values are locally-controlled (from our storage backend)
    /// and determine the order messages are processed, not the bulletin board's external IDs.
    /// LocalBoard automatically tracks last_local_board_id.
    #[instrument(name = "Trustee::update_local_board", skip_all, level = "trace")]
    pub fn update_local_board(
        &mut self,
        messages: Vec<(Message, i64)>,
    ) -> Result<i64, ProtocolError> {
        let configuration = self.local_board.get_configuration_raw();
        if let Some(configuration) = configuration {
            self.update(messages, configuration)
        } else {
            self.update_bootstrap(messages)
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // General (non-bootstrap) update
    //
    // Each message is verified and added to the local board.
    //
    // Takes a vector of (message, message_id) pairs as input plus configuration,
    // returns a pair of (updated messages count, last message id added)
    ///////////////////////////////////////////////////////////////////////////
    fn update(
        &mut self,
        messages: Vec<(Message, i64)>,
        configuration: Configuration<C>,
    ) -> Result<i64, ProtocolError> {
        let mut added = 0;

        // Sanity check: field cfg_hash must exist at this point
        let cfg_hash = self.local_board.get_cfg_hash();
        if cfg_hash.is_none() {
            return Err(ProtocolError::InternalError(
                "Local field cfg_hash not set".to_string(),
            ));
        }

        let cfg_hash = cfg_hash.expect("impossible");
        // Show the latest message received
        if !messages.is_empty() {
            let (last_message, id) = messages.last().expect("impossible");
            trace!(
                "Update: last message is {:?} ({})",
                last_message.statement.get_kind(),
                id
            );
        }

        for (message, id) in messages.into_iter() {
            let statement_info = message.statement.to_string();
            let verified = message.verify(&configuration).map_err(|_e| {
                ProtocolError::VerificationError(format!(
                    "Message failed verification: {:?}, cfg: {:?}",
                    statement_info, &configuration
                ))
            })?;

            if verified.statement.get_cfg_h() != cfg_hash {
                return Err(ProtocolError::MessageConfigurationMismatch(
                    "Message has mismatched configuration hash".to_string(),
                ));
            }

            let stmt = verified.statement.clone();
            self.local_board.add(verified, id)?;
            debug!("Added message type=[{}]", stmt);
            added += 1;
        }

        Ok(added)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Bootstrapping update
    //
    // There is no configuration. We retrieve message zero, check that it's the
    // configuration and add it to the local board.
    //
    // Takes a vector of (message, message_id) pairs as input, returns a pair
    // of (updated messages count, last message id added)
    ///////////////////////////////////////////////////////////////////////////
    fn update_bootstrap(
        &mut self,
        mut messages: Vec<(Message, i64)>,
    ) -> Result<i64, ProtocolError> {
        let mut added = 0;

        trace!("Configuration not present in board, getting first remote message");
        if messages.is_empty() {
            return Err(ProtocolError::BootstrapError(
                "Zero messages received, cannot retrieve configuration".to_string(),
            ));
        }
        let (zero, last_id) = messages.remove(0);

        if zero.statement.get_kind() != StatementType::Configuration {
            return Err(ProtocolError::BootstrapError(format!(
                "Invalid statement type for zeroth message {:?}",
                zero.statement.get_kind()
            )));
        }

        if zero.artifact.is_none() {
            return Err(ProtocolError::BootstrapError(
                "No artifact for configuration message".to_string(),
            ));
        }

        let artifact = zero.artifact.as_ref().expect("impossible");
        let configuration = Configuration::strand_deserialize(artifact)?;

        if !configuration.is_valid() {
            return Err(ProtocolError::InvalidConfiguration(format!(
                "Configuration::is_valid failed, {:?}",
                configuration
            )));
        }

        let verified = zero.verify(&configuration).map_err(|e| {
            ProtocolError::VerificationError(format!(
                "Configuration signature did not verify: {:?}",
                e
            ))
        })?;

        assert!(verified.signer_position == PROTOCOL_MANAGER_INDEX);
        trace!("Verified signature, Configuration signed by Protocol Manager");

        self.local_board.add(verified, last_id)?;
        added += 1;

        // Process the rest of the messages
        if !messages.is_empty() {
            return self.update(messages, configuration);
        }

        Ok(added)
    }

    ///////////////////////////////////////////////////////////////////////////
    // derive
    ///////////////////////////////////////////////////////////////////////////

    /// Derives predicates from the statements in the LocalBoard.
    ///
    /// Verifying mode: used to derive a configuration predicate specific
    /// to running the verifier. This predicate will identify this trustee
    /// as a strict observer, only deriving verification Actions.
    #[instrument(name = "Trustee::derive_predicates", skip(self), level = "trace")]
    fn derive_predicates(&self, verifying_mode: bool) -> Result<Vec<Predicate>, ProtocolError> {
        let mut predicates = vec![];

        let configuration = self.local_board.get_configuration_raw();
        let configuration =
            configuration.ok_or(ProtocolError::MissingArtifact(StatementType::Configuration))?;

        let configuration_p = if !verifying_mode {
            Predicate::get_bootstrap_predicate(&configuration, &self.get_pk()?)
        } else {
            Predicate::get_verifier_bootstrap_predicate(&configuration)
        };

        trace!("Adding bootstrap predicate {:?}", configuration_p);
        predicates.push(configuration_p?);

        let entries = self.local_board.get_statement_entries();

        let stmts: Vec<String> = entries.iter().map(|s| s.key.kind.to_string()).collect();
        trace!(
            "There are {} non bootstrap statements on local board, {:?}",
            entries.len(),
            stmts
        );

        // Generate predicates from board statements
        for entry in entries.iter() {
            debug!("Found statement entry {:?}", entry.value);
            let statement = &entry.value.1;
            let next =
                Predicate::from_statement(statement, entry.key.signer_position, &configuration)?;
            predicates.push(next);
        }

        trace!("Derived {} predicates", predicates.len());

        Ok(predicates)
    }

    ///////////////////////////////////////////////////////////////////////////
    // infer&run
    ///////////////////////////////////////////////////////////////////////////

    /// Runs the datalog engine and inferred Actions.
    ///
    /// Verifying mode: used to execute Actions in verifying mode.
    /// In this mode only those actions that are relevant to verification
    /// are run.
    fn infer_and_run_actions(
        &self,
        predicates: &Vec<Predicate>,
        verifying_mode: bool,
    ) -> Result<(Vec<Message>, HashSet<Action>), ProtocolError> {
        let _ = self
            .local_board
            .get_configuration_raw()
            .ok_or(ProtocolError::MissingArtifact(StatementType::Configuration))?;

        let actions = crate::protocol::datalog::run(predicates)?;
        trace!(
            "Datalog derived {} actions, {:?}",
            actions.len(),
            actions
                .iter()
                .map(|a| format!("{:?}", a))
                .collect::<Vec<String>>()
        );

        let ret_actions = actions.clone();

        if actions.is_empty() {
            trace!("-- Idle --");
        }

        // If there are more than self.max_concurrent_actions actions they will be skipped
        // until the next step.
        let actions: Vec<Action> = if let Some(max) = self.max_concurrent_actions {
            actions.into_iter().take(max).collect()
        } else {
            Vec::from_iter(actions)
        };

        // Cross-Action parallelism (which in effect is cross-batch parallelism)
        // #[cfg(feature = "native")]
        let results: Result<Vec<Vec<Message>>, ProtocolError> = actions
            .into_par_iter()
            .map(|a| {
                info!("Running action {:?}..", a);
                let m = if !verifying_mode {
                    a.run(self)
                } else {
                    a.run_for_verifier(self)
                };

                if m.is_err() {
                    error!("Action {:?} returned error {:?} (propagating)", a, m);
                    m.add_context(&format!("When executing Action {:?}", a))
                } else {
                    m
                }
            })
            .collect();

        /*
        #[cfg(not(feature = "native"))]
        let results: Result<Vec<Vec<Message>>, ProtocolError> = actions
            .into_iter()
            .map(|a| {
                info!("Running action {:?}..", a);
                let m = if !verifying_mode {
                    a.run(self)
                } else {
                    a.run_for_verifier(self)
                };

                if m.is_err() {
                    error!("Action {:?} returned error {:?} (propagating)", a, m);
                    m.add_context(&format!("When executing Action {:?}", a))
                } else {
                    info!("Completed action");
                    m
                }
            })
            .collect();*/

        // flatten all messages
        let result = results?.into_iter().flatten().collect();

        Ok((result, ret_actions))
    }

    ///////////////////////////////////////////////////////////////////////////
    // Trustee verifying mode
    ///////////////////////////////////////////////////////////////////////////

    /// Runs the trustee in verifying mode.
    ///
    /// This function is used as part of the braid verifier.
    pub(crate) fn verify(&mut self, messages: Vec<(Message, i64)>) -> Result<Vec<Message>> {
        self.update_local_board(messages)?;

        let predicates = self.derive_predicates(true)?;
        let (messages, _) = self.infer_and_run_actions(&predicates, true)?;

        Ok(messages)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Artifact accessors for Actions
    ///////////////////////////////////////////////////////////////////////////

    /// Gets the Configuration, with a hash check
    ///
    /// If the configuration does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_configuration(
        &self,
        hash: &ConfigurationHash,
    ) -> Result<&Configuration<C>, ProtocolError> {
        self.local_board
            .get_configuration(hash)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Configuration))
    }

    /// Gets a Channel, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_channel(
        &self,
        hash: &ChannelHash,
        signer_position: TrusteePosition,
    ) -> Result<Channel<C>, ProtocolError> {
        self.local_board.get_channel(hash, signer_position)
    }

    /// Gets a Share, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_shares(
        &self,
        hash: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError> {
        self.local_board.get_shares(hash, signer_position)
    }

    /// Gets the DkgPublicKey, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_dkg_public_key(
        &self,
        hash: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError> {
        self.local_board.get_dkg_public_key(hash, signer_position)
    }

    /// Gets Ballots, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_ballots(
        &self,
        hash: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Ballots<C>, ProtocolError> {
        self.local_board.get_ballots(hash, batch, signer_position)
    }

    /// Gets a Mix, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_mix(
        &self,
        hash: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Mix<C>, ProtocolError> {
        self.local_board.get_mix(hash, batch, signer_position)
    }

    /// Gets DecryptionFactors, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_decryption_factors(
        &self,
        hash: &DecryptionFactorsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<DecryptionFactors<C>, ProtocolError> {
        self.local_board
            .get_decryption_factors(hash, batch, signer_position)
    }

    /// Gets Plaintexts, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised.
    ///
    /// Used by Actions.
    pub(crate) fn get_plaintexts(
        &self,
        hash: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Plaintexts<C>, ProtocolError> {
        self.local_board
            .get_plaintexts(hash, batch, signer_position)
    }

    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn is_config_approved(&self, _config: &Configuration<C>) -> bool {
        // FIXME validate (called by sign cfg Action)
        true
    }

    /// Returns this trustee's signature public key.
    ///
    /// Trustee signing public keys are also used to
    /// identify trustees in protocol Configurations.
    pub fn get_pk(&self) -> Result<StrandSignaturePk, StrandError> {
        StrandSignaturePk::from_sk(&self.signing_key)
    }

    pub(crate) fn encrypt_share_sk(
        &self,
        sk: &PrivateKey<C>,
        _cfg: &Configuration<C>,
    ) -> Result<EncryptionData, ProtocolError> {
        let bytes: &[u8] = &sk.strand_serialize()?;
        let ed = symm::encrypt(self.encryption_key, bytes)?;

        Ok(ed)
    }

    pub(crate) fn decrypt_share_sk(
        &self,
        c: &Channel<C>,
        _cfg: &Configuration<C>,
    ) -> Result<PrivateKey<C>, ProtocolError> {
        let decrypted = symm::decrypt(&self.encryption_key, &c.encrypted_channel_sk)?;
        let ret = PrivateKey::<C>::strand_deserialize(&decrypted)?;

        Ok(ret)
    }

    /// Convenience function used by tests and dbg
    pub fn _get_dkg_public_key_nohash(&self) -> Option<DkgPublicKey<C>> {
        self.local_board.get_dkg_public_key_nohash(0)
    }

    /// Convenience function used by tests and dbg
    pub fn _get_plaintexts_nohash(
        &self,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        self.local_board
            .get_plaintexts_nohash(batch, signer_position)
    }

    ///////////////////////////////////////////////////////////////////////////
}

/// Trustees can sign Messages
impl<C: Ctx, S: LocalBoardStorage> b4::messages::message::Signer for Trustee<C, S> {
    fn get_signing_key(&self) -> &StrandSignatureSk {
        &self.signing_key
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl<C: Ctx, S: LocalBoardStorage> std::fmt::Debug for Trustee<C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trustee({})", self.name)
    }
}

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

/// The configuration for a Trustee.
///
/// Trustee configuration files are passed to the braid
/// binary on startup. Trustee configuration files
/// contain cryptographic secrets.
#[derive(Serialize, Deserialize, Clone)]
pub struct TrusteeConfig {
    // base64 encoding of a der encoded pkcs#8 v1
    pub signing_key_sk: String,
    // base64 encoding of a der encoded spki
    pub signing_key_pk: String,
    // base64 encoding of a sign::SymmetricKey
    pub encryption_key: String,
}
impl TrusteeConfig {
    /// Construct a TrusteeConfig from keys in serialized base64 form.
    pub fn new(signing_key_sk: &str, signing_key_pk: &str, symm_key: &str) -> Self {
        TrusteeConfig {
            signing_key_sk: signing_key_sk.to_string(),
            signing_key_pk: signing_key_pk.to_string(),
            encryption_key: symm_key.to_string(),
        }
    }

    /// Construct a TrusteeConfig from keys in object form.
    pub fn new_from_objects(
        signing_key: StrandSignatureSk,
        encryption_key: symm::SymmetricKey,
    ) -> Self {
        let sk_string = signing_key.to_der_b64_string().unwrap();
        let pk_string = StrandSignaturePk::from_sk(&signing_key)
            .unwrap()
            .to_der_b64_string()
            .unwrap();

        // Compatible with both aes and chacha20poly backends
        let ek_bytes = encryption_key.as_slice();
        let ek_string: String = general_purpose::STANDARD_NO_PAD.encode(ek_bytes);

        TrusteeConfig {
            signing_key_sk: sk_string,
            signing_key_pk: pk_string,
            encryption_key: ek_string,
        }
    }
}

/// The result of running one step of the protocol loop.
///
/// Contains the messages generated by the trustee as well as
/// statistics about the step execution.
#[derive(Debug)]
pub struct StepResult {
    pub messages: Vec<Message>,
    pub actions: HashSet<Action>,
    pub added_messages: i64,
    pub last_id: i64,
}
impl StepResult {
    fn new(
        messages: Vec<Message>,
        actions: HashSet<Action>,
        added_messages: i64,
        last_id: i64,
    ) -> Self {
        StepResult {
            messages,
            actions,
            added_messages,
            last_id,
        }
    }
}
