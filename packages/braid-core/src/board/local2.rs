// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use b3::grpc::GrpcB3Message;
use log::{debug, error, warn};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use strand::context::Ctx;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use b3::messages::artifact::*;
use b3::messages::message::{Message, VerifiedMessage};
use b3::messages::statement::{Statement, StatementType};

use crate::util::{ProtocolContext, ProtocolError};
use b3::messages::newtypes::*;
use strand::hash::Hash;

///////////////////////////////////////////////////////////////////////////
// LocalBoard
///////////////////////////////////////////////////////////////////////////

/// A LocalBoard is a trustee's view of a bulletin board, where by bulletin board
/// we refer to one particular board, not the entire bulletin board system.
/// As such a LocalBoard is specific to a protocol execution (session_id), referenced
/// in the configuration
///
/// Messages are composed of statements and optionally artifacts
///
pub(crate) struct LocalBoard<C: Ctx> {
    pub(crate) configuration: Option<Configuration<C>>,
    cfg_hash: Option<Hash>,

    // All keys contain a statement type and a sender. For multi instance predicates
    // (eg multiple decryption/mixing), they also have a batch (usize)
    //
    // We put the hash in the value so that we can detect overwrite attempt,
    // the statement hash is checked on retrieval (it's not in the key)
    pub(crate) statements: HashMap<StatementEntryIdentifier, (Hash, Statement)>,

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
    //
    // In addition to the bulletin board information being held locally
    // in memory, all messages are also persisted in a sqlite store,
    // such that messages can never be modified or removed, only added,
    // even if the trustee fails and is restarted. The reason the store
    // is an option is to allow running the trustee for testing purposes
    // or to run the verifier where the store is not necessary.
    pub(crate) store: Option<PathBuf>,
    // Depending on whether a store is available, artifact bytes can
    // either be held in the store or in memory. If they are held in
    // the store the i64 value points to the corresponding row.
    pub(crate) artifacts: HashMap<ArtifactEntryIdentifier, (Hash, i64)>,
    pub(crate) artifacts_memory: HashMap<ArtifactEntryIdentifier, (Hash, Vec<u8>)>,
    // For efficiency reasons, it is also possible to store the artifact
    // bytes themselves in the filesystem. In that case the sqlite field
    // for the artifact's bytes will be zero bytes.
    pub(crate) blob_store: Option<PathBuf>,
}

impl<C: Ctx> LocalBoard<C> {
    /// Construct an empty LocalBoard
    pub(crate) fn new(store: Option<PathBuf>, blob_store: Option<PathBuf>) -> LocalBoard<C> {
        tracing::info!("LocalBoard store is: {:?}", store);
        tracing::info!("LocalBoard: blob_store is {:?}", blob_store);

        LocalBoard {
            configuration: None,
            cfg_hash: None,
            statements: HashMap::new(),
            artifacts: HashMap::new(),
            store,
            blob_store,
            artifacts_memory: HashMap::new(),
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add messages to LocalBoard
    ///////////////////////////////////////////////////////////////////////////

    /// Adds a message to the board.
    ///
    /// If the message comes from a store, the store_id must point to the row in the
    /// sqlite store where the message originates. If there is no store this value is
    /// ignored.
    pub(crate) fn add(
        &mut self,
        message: VerifiedMessage,
        store_id: i64,
    ) -> Result<(), ProtocolError> {
        if message.statement.get_kind() == StatementType::Configuration {
            self.add_bootstrap(message)
        } else {
            self.add_message(message, store_id)
        }
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
    /// will be ignored if it is identical. Otherwise an error will be raised.
    fn add_message(
        &mut self,
        message: VerifiedMessage,
        store_id: i64,
    ) -> Result<(), ProtocolError> {
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

                let artifact_entry = self.artifacts.get(&artifact_identifier);

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

                    // Both statement and artifact are new, insert
                    self.statements.insert(
                        statement_identifier,
                        (
                            crate::util::hash_from_vec(&statement_hash)?,
                            message.statement,
                        ),
                    );

                    if self.store.is_some() {
                        self.artifacts
                            .insert(artifact_identifier, (artifact_hash, store_id));
                    } else {
                        self.artifacts_memory
                            .insert(artifact_identifier, (artifact_hash, artifact));
                    }

                    debug!("Artifact inserted");

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
    /// Used by the trustee to deriva all the datalog predicates.
    pub(crate) fn get_statement_entries(&self) -> Vec<BoardEntry> {
        let ret: Vec<BoardEntry> = self
            .statements
            .iter()
            .map(|(k, v)| BoardEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        ret
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
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store or from memory and will be deserialized into
    /// the expected struct.
    pub(crate) fn get_channel(
        &self,
        channel_h: &ChannelHash,
        signer_position: TrusteePosition,
    ) -> Result<Channel<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::Channel, channel_h.0, signer_position)?;
        let bytes = bytes.get_ref();
        Ok(Channel::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets a Share, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store or from memory and will be deserialized into
    /// the expected struct.
    pub(crate) fn get_shares(
        &self,
        shares_h: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::Shares, shares_h.0, signer_position)?;
        let bytes = bytes.get_ref();
        Ok(Shares::strand_deserialize(&bytes)?)
    }

    /// Gets the DkgPublicKey, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store or from memory and will be deserialized into
    /// the expected struct.
    pub(crate) fn get_dkg_public_key(
        &self,
        pk_h: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError> {
        let bytes = self.get_dkg_artifact(StatementType::PublicKey, pk_h.0, signer_position)?;
        let bytes = bytes.get_ref();
        Ok(DkgPublicKey::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets Ballots, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store or from memory and will be deserialized into
    /// the expected struct.
    pub(crate) fn get_ballots(
        &self,
        b_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Ballots<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Ballots, b_h.0, signer_position, batch)?;
        let bytes = bytes.get_ref();
        Ok(Ballots::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets a Mix, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store or from memory and will be deserialized into
    /// the expected struct.
    pub(crate) fn get_mix(
        &self,
        m_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Mix<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Mix, m_h.0, signer_position, batch)?;
        let bytes = bytes.get_ref();
        Ok(Mix::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets DecryptionFactors, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store or from memory and will be deserialized into
    /// the expected struct.
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
        let bytes = bytes.get_ref();
        Ok(DecryptionFactors::<C>::strand_deserialize(&bytes)?)
    }

    /// Gets Plaintexts, with a hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store or from memory and will be deserialized into
    /// the expected struct.
    pub(crate) fn get_plaintexts(
        &self,
        p_h: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<Plaintexts<C>, ProtocolError> {
        let bytes = self.get_artifact(StatementType::Plaintexts, p_h.0, signer_position, batch)?;
        let bytes = bytes.get_ref();
        Ok(Plaintexts::<C>::strand_deserialize(&bytes)?)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Artifact retrieval commonality
    //////////////////////////////////////////////////////////////////////////

    /// Returns a dkg artifact bytes from the store, with hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store) or from memory.
    ///
    /// Dkg artifacts have their batch and mixnumber set to 0.
    fn get_dkg_artifact(
        &self,
        kind: StatementType,
        hash: Hash,
        signer_position: TrusteePosition,
    ) -> Result<ArtifactRef<Vec<u8>>, ProtocolError> {
        self.get_artifact(kind, hash, signer_position, 0)
    }

    /// Returns an artifact bytes from the store, with hash check.
    ///
    /// If the artifact does not exist, or the supplied hash does not match
    /// an error is raised. The artifact bytes will be retrieved from the
    /// store (or blob store) or from memory.
    ///
    /// All artifacts have their mix number set to 0. Only mix signature
    /// statements are keyed (in the hashmap) with a mix number != 0.
    fn get_artifact(
        &self,
        kind: StatementType,
        hash: Hash,
        signer_position: TrusteePosition,
        batch: BatchNumber,
    ) -> Result<ArtifactRef<Vec<u8>>, ProtocolError> {
        // Mix number is always zero for all artifacts, only a signed mix _statement_ has a mixnumber
        let aei = self.get_artifact_entry_identifier_ext(kind.clone(), signer_position, batch, 0);

        let bytes = if self.store.is_some() {
            let entry = self
                .artifacts
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(kind.clone()))?;

            if hash != entry.0 {
                return Err(ProtocolError::MismatchedArtifactHash(kind));
            } else {
                let bytes = self.get_artifact_from_store(entry.1);

                let Ok(bytes) = bytes else {
                    error!("Error retrieving artifact: {}", bytes.err().unwrap());
                    return Err(ProtocolError::MissingArtifact(kind));
                };

                ArtifactRef::Owned(bytes)
            }
        } else {
            let entry = self
                .artifacts_memory
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(kind.clone()))?;

            if hash != entry.0 {
                return Err(ProtocolError::MismatchedArtifactHash(kind));
            } else {
                ArtifactRef::Ref(&entry.1)
            }
        };

        Ok(bytes)
    }

    ///////////////////////////////////////////////////////////////////////////
    // LocalBoard key construction
    ///////////////////////////////////////////////////////////////////////////

    /// Constructs statement entry keys.
    ///
    /// Statement entry keys data structures contain the
    /// identifying information that makes them unique,
    /// there can not be more than one per board.
    /// This together with a persistent store makes
    /// the board append only: a LocalBoard will not
    /// accept a subsequent duplicate message. The order
    /// is established locally by the trustee.
    pub(crate) fn get_statement_entry_identifier(
        &self,
        statement: &Statement,
        signer_position: usize,
    ) -> StatementEntryIdentifier {
        let (kind, _, batch, mix_number, _) = statement.get_data();

        StatementEntryIdentifier {
            kind,
            signer_position,
            batch: batch,
            mix_number: mix_number,
        }
    }
    /// Constructs artifact entry keys from a statement entry key.
    ///
    /// Artifact entry keys are entirely
    /// identified by their originating statements.
    /// Like statements, they are also unique.
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
    ///
    /// Artifact entry keys are entirely
    /// identified by their originating statements.
    /// Like statements, they are also unique.
    pub(crate) fn get_artifact_entry_identifier_ext(
        &self,
        statement_type: StatementType,
        signer_position: usize,
        batch: BatchNumber,
        mix_number: MixNumber,
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
    // Message store
    ///////////////////////////////////////////////////////////////////////////

    /// Updates the message store with the supplied remote messages
    ///
    /// This method can be called independently of step, to only update the store
    /// (when a truncated message is received from the bulletin board)
    ///
    /// Messages are deserialized to recover metadata, and then stored.
    /// If a blob store exists, bytes will be stored in the filesystem, and
    /// the message store will only contain the metadata.
    pub(crate) fn update_store(
        &self,
        messages: &Vec<GrpcB3Message>,
        ignore_existing: bool,
    ) -> Result<()> {
        let now = Instant::now();

        if let Some(blob_store) = &self.blob_store {
            if !blob_store.exists() {
                fs::create_dir_all(&blob_store)?;
            }
        }

        let connection = self.get_store()?;

        // FIXME verify message signatures before inserting in local store
        let mut statement = if ignore_existing {
            connection.prepare(
                "INSERT OR IGNORE INTO MESSAGES(external_id, message, sender_pk, statement_kind, batch, mix_number) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            )?
        } else {
            connection.prepare(
                "INSERT INTO MESSAGES(external_id, message, sender_pk, statement_kind, batch, mix_number) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            )?
        };

        connection.execute("BEGIN TRANSACTION", [])?;
        for m in messages {
            if m.version != b3::get_schema_version() {
                return Err(anyhow::anyhow!(
                    "Mismatched schema version: {} != {}",
                    m.version,
                    b3::get_schema_version()
                ));
            }
            let message = Message::strand_deserialize(&m.message)?;
            let sender_pk = message.sender.pk.to_der_b64_string()?;
            let kind = message.statement.get_kind().to_string();
            let batch: i32 = message.statement.get_batch_number().try_into()?;
            let mix_number: i32 = message.statement.get_mix_number().try_into()?;

            // let hash = strand::hash::hash_b64(&m.message)?;
            if let Some(blob_store) = &self.blob_store {
                if !blob_store.exists() {
                    fs::create_dir_all(&blob_store)?;
                }
                let name = format!("{}-{}-{}-{}", kind, sender_pk, batch, mix_number);
                let path = blob_store.join(name.replace("/", ":"));
                if !path.exists() {
                    let mut file = File::create(&path)?;
                    file.write_all(&m.message)?;
                    tracing::info!(
                        "update_store: wrote {} bytes to {:?}",
                        m.message.len(),
                        path
                    );
                }
                statement.execute(params![m.id, vec![], sender_pk, kind, batch, mix_number])?;
            } else {
                statement.execute(params![m.id, m.message, sender_pk, kind, batch, mix_number])?;
            }
        }
        connection.execute("END TRANSACTION", [])?;

        drop(statement);

        if messages.len() > 0 {
            tracing::info!(
                "update_store: inserted {} messages in {}ms",
                messages.len(),
                now.elapsed().as_millis()
            );
        }

        Ok(())
    }

    /// Updates the message store and returns messages not yet in the board.
    ///
    /// First, the message store is updated with remote messages. Then the
    /// store is queried for messages that the board has not yet seen, as
    /// specified by last_message_id. This parameter refers to the last
    /// message id that the board _has_ seen.
    ///
    /// If a blob store exists, the message bytes will be retrieved from it
    /// and combined with the metadata in the message store.
    pub(crate) fn store_and_return_messages(
        &mut self,
        messages: &Vec<GrpcB3Message>,
        last_message_id: i64,
        ignore_existing: bool,
    ) -> Result<Vec<(Message, i64)>> {
        self.update_store(messages, ignore_existing)?;

        let connection = self.get_store()?;

        // The order by id asc clause is significant, as it ensures that the trustee
        // cannot be made to accept a different order than what it has established
        // locally. See self::get_store.
        let mut stmt = connection
            .prepare("SELECT id,message,sender_pk,statement_kind,batch,mix_number FROM MESSAGES where id > ?1 order by id asc")?;

        let rows = stmt.query_map([last_message_id], |row| {
            Ok(SqliteStoreMessageRow {
                id: row.get(0)?,
                message: row.get(1)?,
                sender_pk: row.get(2)?,
                kind: row.get(3)?,
                batch: row.get(4)?,
                mix_number: row.get(5)?,
            })
        })?;

        let messages: Result<Vec<(Message, i64)>> = rows
            .map(|mr| {
                let row = mr?;
                let id = row.id;
                // let message = Message::strand_deserialize(&row.message)?;
                let message = if let Some(blob_store) = &self.blob_store {
                    let name = format!(
                        "{}-{}-{}-{}",
                        row.kind, row.sender_pk, row.batch, row.mix_number
                    );
                    let path = blob_store.join(name.replace("/", ":"));
                    assert!(path.exists());
                    let mut file = File::open(&path)?;
                    let mut buffer = vec![];

                    let bytes = file.read_to_end(&mut buffer)?;
                    tracing::info!("store_read: read {} bytes from {:?}", bytes, path);
                    Message::strand_deserialize(&buffer)?
                } else {
                    Message::strand_deserialize(&row.message)?
                };

                Ok((message, id))
            })
            .collect();

        messages
    }

    /// Returns the largest id stored in the message store.
    ///
    /// If the store is empty -1 will be returned as a lower bound
    /// on all possible message ids. Note that the last external id
    /// seen by the store is _not_ the same as the last message id
    /// seen by the board.
    pub(crate) fn get_last_external_id(&mut self) -> Result<i64> {
        let connection = self.get_store()?;

        let external_last_id =
            connection.query_row("SELECT max(external_id) FROM messages;", [], |row| {
                row.get(0)
            });

        let external_last_id = external_last_id.unwrap_or(-1);

        Ok(external_last_id)
    }

    /// Returns a specific's artifact bytes from the store.
    ///
    /// The store_id identifies the requested artifact. If a
    /// blob store exists, the bytes will be retrieved from it.
    /// Otherwise the bytes will be directly in the message store.
    fn get_artifact_from_store(&self, store_id: i64) -> Result<Vec<u8>> {
        let connection = self.get_store()?;
        let mut stmt =
            connection.prepare("SELECT id,message,sender_pk,statement_kind,batch,mix_number FROM MESSAGES where id = ?1")?;

        let mut rows = stmt.query([store_id])?;
        let bytes: Vec<u8> = if let Some(row) = rows.next()? {
            let bytes = if let Some(blob_store) = &self.blob_store {
                let sender_pk: String = row.get(2)?;
                let kind: String = row.get(3)?;
                let batch: i32 = row.get(4)?;
                let mix_number: i32 = row.get(5)?;
                let name = format!("{}-{}-{}-{}", kind, sender_pk, batch, mix_number);
                let path = blob_store.join(name.replace("/", ":"));
                assert!(path.exists());
                let mut file = File::open(&path)?;
                let mut buffer = vec![];

                let bytes = file.read_to_end(&mut buffer)?;
                tracing::info!(
                    "get_artifact_from_store: read {} bytes from {:?}",
                    bytes,
                    path
                );
                buffer
            } else {
                row.get(1)?
            };

            bytes
        } else {
            // return Err(ProtocolError::BoardError(format!("Could not find artifact with id {}", store_id)));
            return Err(anyhow::anyhow!(
                "Could not find message with id {}",
                store_id
            ));
        };

        let message = Message::strand_deserialize(&bytes)?;

        let Some(bytes) = message.artifact else {
            return Err(anyhow::anyhow!(
                "Message with id {} did not contain artifact",
                store_id
            ));
        };

        Ok(bytes)
    }

    /// Returns a connection to the message store.
    ///
    /// If the messages table does not exist it is created.
    ///
    /// The autogenerated id column is used to establish an order that cannot be
    /// manipulated by the external board. Once a retrieved message is
    /// stored and assigned a local id, it is not possible for later messages
    /// to have an earlier id. The external bulletin board can therefore
    /// not alter history by prepending messages.
    /// See https://www.sqlite.org/autoinc.html
    /// Note also that messages store update functions are never called
    /// concurrently per board.
    ///
    /// The external_id column is used to retrieve _new_ messages as
    /// defined by the external board.
    fn get_store(&self) -> Result<Connection> {
        let store = self.store.as_ref().ok_or(anyhow::anyhow!(
            "Should be impossible: called get_store when store was None"
        ))?;
        let connection = Connection::open(&store)?;

        connection.execute("CREATE TABLE if not exists MESSAGES(id INTEGER PRIMARY KEY AUTOINCREMENT, external_id INT8 NOT NULL UNIQUE, message BLOB NOT NULL, sender_pk TEXT NOT NULL, statement_kind TEXT NOT NULL, batch INT4 NOT NULL, mix_number INT4 NOT NULL, UNIQUE(sender_pk, statement_kind, batch, mix_number))", [])?;

        Ok(connection)
    }

    /// The maximum number of messages this protocol will generate.
    ///
    /// A protocol is finished when all dkg messages are present and all tally
    /// messages are present given the existing batches.
    ///
    /// The number of messages for each phase are
    ///    DKG phase: 1 + 5n
    ///                        ballot  mix     mix signature     decrypt factors    plaintext + sig
    ///    Tally phase:    b * (1 +     t +    (t * (t - 1)) +    t +                 n)
    ///
    /// where
    /// n: trustees
    /// t: threshold
    /// b: batches
    ///
    pub(crate) fn max_messages(&self) -> usize {
        let Some(cfg) = &self.configuration else {
            return 0;
        };

        let mut sei = StatementEntryIdentifier {
            kind: StatementType::Ballots,
            signer_position: PROTOCOL_MANAGER_INDEX,
            batch: 1,
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
        if sei.batch == 0 {
            return dkg;
        }

        let per_batch_tally = 1 + (2 * t) + (t * (t - 1)) + n;

        dkg + ((sei.batch - 1) * per_batch_tally)

        // self.statements.len() == max
    }

    ///////////////////////////////////////////////////////////////////////////
    // Testing functions (used by tests and dbg)
    //
    // These functions assume there is no store and artifacts are in memory
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

/// Convenience to return artifact owned objects or references.
///
/// Owned objects are used when reading from the message store.
/// References are used when there is no message store and artifacts
/// are held in memory. This only occurs during testing.
pub enum ArtifactRef<'a, T> {
    Ref(&'a T),
    Owned(T),
}
impl<'a, T> ArtifactRef<'a, T> {
    pub fn get_ref(&'a self) -> &'a T {
        match self {
            ArtifactRef::Ref(ref v) => *v,
            ArtifactRef::Owned(v) => v,
        }
    }
    pub fn transform<U, F: FnOnce(&'a T) -> &'a U, G: FnOnce(T) -> U>(
        self,
        f: F,
        g: G,
    ) -> ArtifactRef<'a, U> {
        let ret = match self {
            ArtifactRef::Ref(ref v) => ArtifactRef::Ref(f(*v)),
            ArtifactRef::Owned(v) => ArtifactRef::Owned(g(v)),
        };

        ret
    }
}

/// Convenience to return entries to the trustee for inference.
pub(crate) struct BoardEntry {
    pub(crate) key: StatementEntryIdentifier,
    pub(crate) value: (Hash, Statement),
}

///////////////////////////////////////////////////////////////////////////
// LocalBoard keys
///////////////////////////////////////////////////////////////////////////

/// Key used to store statements in the statement map
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct StatementEntryIdentifier {
    pub kind: StatementType,
    pub signer_position: usize,
    // the batch number
    pub batch: usize,
    // When storing mix signature statementents in the local board they
    // will not be unique with the above fields only.
    // (mixes themselves are, since only one mix is produced by each trustee, so the signer position
    // is sufficient; on the other hand each trustee signs _all other mixes_).
    // Without including this field in the hash key, the different signature statements
    // would be rejected as duplicates.
    pub mix_number: usize,
}

/// Key used to store artifacts in the artifact map
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct ArtifactEntryIdentifier {
    pub statement_entry: StatementEntryIdentifier,
}

/// A row of the message store
struct SqliteStoreMessageRow {
    id: i64,
    message: Vec<u8>,
    sender_pk: String,
    kind: String,
    batch: i32,
    mix_number: i32,
}
