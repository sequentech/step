// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use board_messages::grpc::GrpcB3Message;
use log::{debug, error, warn};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;

use strand::context::Ctx;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use board_messages::braid::artifact::*;
use board_messages::braid::message::{Message, VerifiedMessage};
use board_messages::braid::statement::{Statement, StatementType};

use board_messages::braid::newtypes::*;
use strand::hash::Hash;

use crate::util::{ProtocolContext, ProtocolError};

use super::ArtifactRef;

///////////////////////////////////////////////////////////////////////////
// LocalBoard
///////////////////////////////////////////////////////////////////////////

// A LocalBoard is a trustee's in-memory copy of a bulletin board. It is specific to a protocol
// execution (session_id), referenced in the configuration
//
// Messages are composed of statements and optionally artifacts
//
// #[derive(Clone)] // FIXME used by dbg
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
    pub(crate) artifacts: HashMap<ArtifactEntryIdentifier, (Hash, i64)>,
    pub(crate) store: Option<PathBuf>,
    pub(crate) in_memory: bool,
    pub(crate) artifacts_memory: HashMap<ArtifactEntryIdentifier, (Hash, Vec<u8>)>,
    // For efficiency we store these artifacts in deserialized form,
    // which requires separate collections
    pub(crate) mixes: HashMap<ArtifactEntryIdentifier, (Hash, Mix<C>)>,
    pub(crate) ballots: HashMap<ArtifactEntryIdentifier, (Hash, Ballots<C>)>,
    pub(crate) decryption_factors: HashMap<ArtifactEntryIdentifier, (Hash, DecryptionFactors<C>)>,
    pub(crate) plaintexts: HashMap<ArtifactEntryIdentifier, (Hash, Plaintexts<C>)>,
}

impl<C: Ctx> LocalBoard<C> {
    pub(crate) fn new(store: Option<PathBuf>, in_memory: bool) -> LocalBoard<C> {
        
        let mem = if store.is_none() {
            true
        }
        else {
            in_memory
        };

        tracing::info!("LocalBoard in_memory: {}", mem);
        
        LocalBoard {
            configuration: None,
            cfg_hash: None,
            statements: HashMap::new(),
            artifacts: HashMap::new(),
            store,
            in_memory: mem,
            artifacts_memory: HashMap::new(),
            mixes: HashMap::new(),
            ballots: HashMap::new(),
            decryption_factors: HashMap::new(),
            plaintexts: HashMap::new(),
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add messages to LocalBoard
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn add(&mut self, message: VerifiedMessage, store_id: i64) -> Result<(), ProtocolError> {
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

     fn add_message(&mut self, message: VerifiedMessage, store_id: i64) -> Result<(), ProtocolError> {
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

                    if self.store.is_some() && !self.in_memory {
                        self.artifacts
                        .insert(artifact_identifier, (artifact_hash, store_id));
                    }
                    else {
                        self.insert_artifact_memory(artifact_identifier, artifact_hash, artifact)?;
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

    pub(crate) fn get_cfg_hash(&self) -> Option<Hash> {
        self.cfg_hash
    }

    pub(crate) fn get_configuration_raw(&self) -> Option<Configuration<C>> {
        self.configuration.clone()
    }

    pub(crate) fn get_entries(&self) -> Vec<BoardEntry> {
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
        let bytes = bytes.get_ref();
        Ok(Channel::<C>::strand_deserialize(&bytes)?)
       
        /*let aei =
            self.get_artifact_entry_identifier_ext(StatementType::Channel, signer_position, 0, 0);
        let entry = self
            .artifacts
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Channel))?;

        if channel_h.0 != entry.0 {
            Err(ProtocolError::MismatchedArtifactHash(
                StatementType::Channel,
            ))
        } else {
            let bytes = self.get_artifact_from_store(entry.1);

            let Ok(bytes) = bytes else {
                error!("Error retrieving artifact: {}", bytes.err().unwrap());
                return Err(ProtocolError::MissingArtifact(StatementType::Channel));
            };
            
            Ok(Channel::<C>::strand_deserialize(&bytes)?)
        }*/
    }

    pub(crate) fn get_shares(
        &self,
        shares_h: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError> {
        
        let bytes = self.get_dkg_artifact(StatementType::Shares, shares_h.0, signer_position)?;
        let bytes = bytes.get_ref();
        Ok(Shares::strand_deserialize(&bytes)?)
        /*let aei =
            self.get_artifact_entry_identifier_ext(StatementType::Shares, signer_position, 0, 0);
        let entry = self
            .artifacts
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Shares))?;
        if shares_h.0 != entry.0 {
            Err(ProtocolError::MismatchedArtifactHash(StatementType::Shares))
        } else {
            let bytes = self.get_artifact_from_store(entry.1);

            let Ok(bytes) = bytes else {
                error!("Error retrieving artifact: {}", bytes.err().unwrap());
                return Err(ProtocolError::MissingArtifact(StatementType::Shares));
            };
            
            Ok(Shares::strand_deserialize(&bytes)?)
        }*/
    }

    pub(crate) fn get_dkg_public_key(
        &self,
        pk_h: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError> {
        
        let bytes = self.get_dkg_artifact(StatementType::PublicKey, pk_h.0, signer_position)?;
        let bytes = bytes.get_ref();
        Ok(DkgPublicKey::<C>::strand_deserialize(&bytes)?)
        /*let aei =
            self.get_artifact_entry_identifier_ext(StatementType::PublicKey, signer_position, 0, 0);
        let entry = self
            .artifacts
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::PublicKey))?;
        if pk_h.0 != entry.0 {
            Err(ProtocolError::MismatchedArtifactHash(
                StatementType::PublicKey,
            ))
        } else {
            let bytes = self.get_artifact_from_store(entry.1);

            let Ok(bytes) = bytes else {
                error!("Error retrieving artifact: {}", bytes.err().unwrap());
                return Err(ProtocolError::MissingArtifact(StatementType::PublicKey));
            };
            
            Ok(DkgPublicKey::<C>::strand_deserialize(&bytes)?)
        }*/
    }

    pub(crate) fn get_ballots(
        &self,
        b_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<ArtifactRef<Ballots<C>>, ProtocolError> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Ballots,
            signer_position,
            batch,
            0,
        );

        if self.store.is_some() && !self.in_memory {
            let entry = self
                .artifacts
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(StatementType::Ballots))?;
            if b_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(
                    StatementType::Ballots,
                ))
            } else {
                let bytes = self.get_artifact_from_store(entry.1);

                let Ok(bytes) = bytes else {
                    error!("Error retrieving artifact: {}", bytes.err().unwrap());
                    return Err(ProtocolError::MissingArtifact(StatementType::Ballots));
                };
                Ok(ArtifactRef::Owned(Ballots::<C>::strand_deserialize(&bytes)?))
            }
        }
        else {
            let entry = self
                .ballots
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(StatementType::Ballots))?;
            if b_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(
                    StatementType::Ballots,
                ))
            } else {
                Ok(ArtifactRef::Ref(&entry.1))
            }
        }
        
    }

    pub(crate) fn get_mix(
        &self,
        m_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<ArtifactRef<Mix<C>>, ProtocolError> {
        let aei =
            self.get_artifact_entry_identifier_ext(StatementType::Mix, signer_position, batch, 0);
        
        if self.store.is_some() && !self.in_memory {
            let entry = self
                .artifacts
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(StatementType::Mix))?;
            if m_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(StatementType::Mix))
            } else {
                
                let bytes = self.get_artifact_from_store(entry.1);

                let Ok(bytes) = bytes else {
                    error!("Error retrieving artifact: {}", bytes.err().unwrap());
                    return Err(ProtocolError::MissingArtifact(StatementType::Mix));
                };
                Ok(ArtifactRef::Owned(Mix::<C>::strand_deserialize(&bytes)?))
                
            }
        }
        else {
            let entry = self
                .mixes
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(StatementType::Mix))?;
            if m_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(StatementType::Mix))
            } else {
                Ok(ArtifactRef::Ref(&entry.1))
            }
        }
        
    }

    pub(crate) fn get_decryption_factors(
        &self,
        m_h: &DecryptionFactorsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<ArtifactRef<DecryptionFactors<C>>, ProtocolError> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::DecryptionFactors,
            signer_position,
            batch,
            0,
        );

        if self.store.is_some() && !self.in_memory {
            let entry = self
                .artifacts
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(
                    StatementType::DecryptionFactors,
                ))?;

            if m_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(
                    StatementType::DecryptionFactors,
                ))
            } else {
                let bytes = self.get_artifact_from_store(entry.1);

                let Ok(bytes) = bytes else {
                    error!("Error retrieving artifact: {}", bytes.err().unwrap());
                    return Err(ProtocolError::MissingArtifact(StatementType::DecryptionFactors));
                };
                
                Ok(ArtifactRef::Owned(DecryptionFactors::<C>::strand_deserialize(&bytes)?))
            }
        }
        else {
            let entry = self
                .decryption_factors
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(
                    StatementType::DecryptionFactors,
                ))?;
            if m_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(
                    StatementType::DecryptionFactors,
                ))
            } else {
                Ok(ArtifactRef::Ref(&entry.1))
            }
        }
    }

    pub(crate) fn get_plaintexts(
        &self,
        m_h: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<ArtifactRef<Plaintexts<C>>, ProtocolError> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Plaintexts,
            signer_position,
            batch,
            0,
        );

        if self.store.is_some() && !self.in_memory {
            let entry = self
            .artifacts
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Plaintexts))?;
            if m_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(
                    StatementType::Plaintexts,
                ))
            } else {
                let bytes = self.get_artifact_from_store(entry.1);

                let Ok(bytes) = bytes else {
                    error!("Error retrieving artifact: {}", bytes.err().unwrap());
                    return Err(ProtocolError::MissingArtifact(StatementType::Plaintexts));
                };
                
                Ok(ArtifactRef::Owned(Plaintexts::<C>::strand_deserialize(&bytes)?))
            }
        }
        else {
            let entry = self
                .plaintexts
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(StatementType::Plaintexts))?;
            if m_h.0 != entry.0 {
                Err(ProtocolError::MismatchedArtifactHash(
                    StatementType::Plaintexts,
                ))
            } else {
                Ok(ArtifactRef::Ref(&entry.1))
            }
        }
        
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
            batch: batch,
            mix_number: mix_number,
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

    // Updates the message store with the passed in messages. This method can
    // be called independently of step, to only update the store (when a truncated 
    // message is received from the bulletin board)
    pub(crate) fn update_store(
        &mut self,
        messages: &Vec<GrpcB3Message>,
        ignore_existing: bool,
    ) -> Result<()> {
        let connection = self.get_store()?;

        // FIXME verify message signatures before inserting in local store
        let mut statement = if ignore_existing {
            connection.prepare(
                "INSERT OR IGNORE INTO MESSAGES(external_id, message, blob_hash) VALUES(?1, ?2, ?3)",
            )?
        } else {
            connection.prepare(
                "INSERT INTO MESSAGES(external_id, message, blob_hash) VALUES(?1, ?2, ?3)",
            )?
        };

        connection.execute("BEGIN TRANSACTION", [])?;
        for m in messages {
            let hash = strand::hash::hash(&m.message)?;
            statement.execute(params![m.id, m.message, hash])?;
        }
        connection.execute("END TRANSACTION", [])?;

        drop(statement);

        Ok(())
    }

    pub(crate) fn store_and_return_messages(
        &mut self,
        messages: &Vec<GrpcB3Message>,
        last_message_id: i64,
        ignore_existing: bool
    ) -> Result<Vec<(Message, i64)>> {
        
        self.update_store(messages, ignore_existing)?;

        let connection = self.get_store()?;

        let mut stmt =
            connection.prepare("SELECT id,message FROM MESSAGES where id > ?1 order by id asc")?;

        let rows = stmt.query_map([last_message_id], |row| {
            Ok(SqliteStoreMessageRow {
                id: row.get(0)?,
                message: row.get(1)?,
            })
        })?;

        let messages: Result<Vec<(Message, i64)>> = rows
            .map(|mr| {
                let row = mr?;
                let id = row.id;
                let message = Message::strand_deserialize(&row.message)?;
                Ok((message, id))
            })
            .collect();

        messages
    }

    // Returns the largest id stored in the local message store
    pub(crate) fn get_last_external_id(&mut self) -> Result<i64> {
        let connection = self.get_store()?;

        let external_last_id =
            connection.query_row("SELECT max(external_id) FROM messages;", [], |row| {
                row.get(0)
            });

        let external_last_id = external_last_id.unwrap_or(-1);

        Ok(external_last_id)
    }

    fn get_artifact_from_store(&self, store_id: i64) -> Result<Vec<u8>> {
        let connection = self.get_store()?;
        let mut stmt =
            connection.prepare("SELECT id,message FROM MESSAGES where id = ?1")?;

        let mut rows = stmt.query([store_id])?;
        let bytes: Vec<u8> = if let Some(row) = rows.next()? {
            row.get(1)?
        }
        else {
            // return Err(ProtocolError::BoardError(format!("Could not find artifact with id {}", store_id)));
            return Err(anyhow::anyhow!("Could not find message with id {}", store_id));
        };

        let message = Message::strand_deserialize(&bytes)?;

        let Some(bytes) = message.artifact else {
            return Err(anyhow::anyhow!("Message with id {} did not contain artifact", store_id));
        };
        
        Ok(bytes)
    }

    fn get_store(&self) -> Result<Connection> {
        let store = self.store.as_ref().ok_or(anyhow::anyhow!("Should be impossible: called get_store when store was None"))?;
        let connection = Connection::open(&store)?;
        // The autogenerated id column is used to establish an order that cannot be manipulated by the external board. Once a retrieved message is
        // stored and assigned a local id, it is not possible for later messages to have an earlier id.
        // The external_id column is used to retrieve _new_ messages as defined by the external board (to optimize bandwidth).
        connection.execute("CREATE TABLE if not exists MESSAGES(id INTEGER PRIMARY KEY AUTOINCREMENT, external_id INT8 NOT NULL UNIQUE, message BLOB NOT NULL, blob_hash BLOB NOT NULL UNIQUE)", [])?;

        Ok(connection)
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
    ) -> Option<&Plaintexts<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Plaintexts,
            signer_position,
            batch,
            0,
        );
        let entry = self.plaintexts.get(&aei)?;

        Some(&entry.1)
    }

    fn insert_artifact_memory(&mut self, ai: ArtifactEntryIdentifier, hash: [u8; 64], bytes: Vec<u8>) -> Result<(), ProtocolError> {
        match ai.statement_entry.kind {
            StatementType::Ballots => {
                let ballots = Ballots::<C>::strand_deserialize(&bytes)?;
                self.ballots.insert(ai, (hash, ballots));
            },
            StatementType::Mix => {
                let mix = Mix::<C>::strand_deserialize(&bytes)?;
                self.mixes.insert(ai, (hash, mix));
            },
            StatementType::DecryptionFactors => {
                let decryption_factors = DecryptionFactors::<C>::strand_deserialize(&bytes)?;
                self.decryption_factors.insert(ai, (hash, decryption_factors));
            },
            StatementType::Plaintexts => {
                let plaintexts = Plaintexts::<C>::strand_deserialize(&bytes)?;
                self.plaintexts.insert(ai, (hash, plaintexts));
            },
            _ => { self.artifacts_memory.insert(ai, (hash, bytes)); },
        };

        Ok(())
    }
    
    // Common code to get dkg artifacts
    fn get_dkg_artifact(
        &self, 
        kind: StatementType, 
        hash: Hash, 
        signer_position: TrusteePosition,
    ) -> Result<ArtifactRef<Vec<u8>>, ProtocolError> {
        let aei =
            self.get_artifact_entry_identifier_ext(kind.clone(), signer_position, 0, 0);
        
        let bytes = if self.store.is_some() && !self.in_memory {
            let entry = self
                .artifacts
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(kind.clone()))?;

            if hash != entry.0 {
                return Err(ProtocolError::MismatchedArtifactHash(
                    kind,
                ))
            } else {
                let bytes = self.get_artifact_from_store(entry.1);
    
                let Ok(bytes) = bytes else {
                    error!("Error retrieving artifact: {}", bytes.err().unwrap());
                    return Err(ProtocolError::MissingArtifact(kind));
                };
                
                ArtifactRef::Owned(bytes)
            }
        }
        else {
            let entry = self
                .artifacts_memory
                .get(&aei)
                .ok_or(ProtocolError::MissingArtifact(kind.clone()))?;
            
            if hash != entry.0 {
                return Err(ProtocolError::MismatchedArtifactHash(
                    kind,
                ))
            } else {
                ArtifactRef::Ref(&entry.1)
            }
        };

        Ok(bytes)
        
    }
}

pub(crate) struct BoardEntry {
    pub(crate) key: StatementEntryIdentifier,
    pub(crate) value: (Hash, Statement),
}

///////////////////////////////////////////////////////////////////////////
// LocalBoard keys
///////////////////////////////////////////////////////////////////////////

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

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct ArtifactEntryIdentifier {
    pub statement_entry: StatementEntryIdentifier,
}

struct SqliteStoreMessageRow {
    id: i64,
    message: Vec<u8>,
}