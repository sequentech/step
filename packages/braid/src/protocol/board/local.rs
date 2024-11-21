// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use log::{debug, error, warn};
use std::collections::HashMap;

use strand::context::Ctx;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use b3::messages::artifact::*;
use b3::messages::message::VerifiedMessage;
use b3::messages::statement::{Statement, StatementType};

use b3::messages::newtypes::*;
use strand::hash::Hash;

use crate::util::{ProtocolContext, ProtocolError};

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
    pub(crate) artifacts: HashMap<ArtifactEntryIdentifier, (Hash, Vec<u8>)>,
    // For efficiency we store these artifacts in deserialized form,
    // which requires separate collections
    pub(crate) mixes: HashMap<ArtifactEntryIdentifier, (Hash, Mix<C>)>,
    pub(crate) ballots: HashMap<ArtifactEntryIdentifier, (Hash, Ballots<C>)>,
    pub(crate) decryption_factors: HashMap<ArtifactEntryIdentifier, (Hash, DecryptionFactors<C>)>,
    pub(crate) plaintexts: HashMap<ArtifactEntryIdentifier, (Hash, Plaintexts<C>)>,
}

impl<C: Ctx> LocalBoard<C> {
    pub(crate) fn new() -> LocalBoard<C> {
        let statements = HashMap::new();
        let artifacts = HashMap::new();

        LocalBoard {
            configuration: None,
            cfg_hash: None,
            statements,
            artifacts,
            mixes: HashMap::new(),
            ballots: HashMap::new(),
            decryption_factors: HashMap::new(),
            plaintexts: HashMap::new(),
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add messages to LocalBoard
    ///////////////////////////////////////////////////////////////////////////

    pub fn add(&mut self, message: VerifiedMessage) -> Result<(), ProtocolError> {
        if message.statement.get_kind() == StatementType::Configuration {
            self.add_bootstrap(message)
        } else {
            self.add_message(message)
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

                // let artifact_entry = self.artifacts.get(&artifact_identifier);
                let existing_hash = self.get_artifact_hash(&artifact_identifier);

                if let Some(existing_hash) = existing_hash {
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

                    /*self.artifacts
                        .insert(artifact_identifier, (artifact_hash, artifact));*/
                    self.insert_artifact(artifact_identifier, artifact_hash, artifact)?;

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

    fn get_artifact_hash(&self, ai: &ArtifactEntryIdentifier) -> Option<&[u8; 64]> {
        match ai.statement_entry.kind {
            StatementType::Ballots => self.ballots.get(ai).map(|v| &v.0),
            StatementType::Mix => self.mixes.get(ai).map(|v| &v.0),
            StatementType::DecryptionFactors => self.decryption_factors.get(ai).map(|v| &v.0),
            StatementType::Plaintexts => self.plaintexts.get(ai).map(|v| &v.0),
            _ => self.artifacts.get(&ai).map(|v| &v.0)
        }
    }

    fn insert_artifact(&mut self, ai: ArtifactEntryIdentifier, hash: [u8; 64], bytes: Vec<u8>) -> Result<(), ProtocolError> {
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
            _ => { self.artifacts.insert(ai, (hash, bytes)); },
        };

        Ok(())
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
        let aei =
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
            Ok(Channel::<C>::strand_deserialize(&entry.1)?)
        }
    }

    pub(crate) fn get_shares(
        &self,
        shares_h: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError> {
        let aei =
            self.get_artifact_entry_identifier_ext(StatementType::Shares, signer_position, 0, 0);
        let entry = self
            .artifacts
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Shares))?;
        if shares_h.0 != entry.0 {
            Err(ProtocolError::MismatchedArtifactHash(StatementType::Shares))
        } else {
            Ok(Shares::strand_deserialize(&entry.1)?)
        }
    }

    pub(crate) fn get_dkg_public_key(
        &self,
        pk_h: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError> {
        let aei =
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
            Ok(DkgPublicKey::<C>::strand_deserialize(&entry.1)?)
        }
    }

    pub(crate) fn get_ballots(
        &self,
        b_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<&Ballots<C>, ProtocolError> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Ballots,
            signer_position,
            batch,
            0,
        );
        let entry = self
            .ballots
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Ballots))?;
        if b_h.0 != entry.0 {
            Err(ProtocolError::MismatchedArtifactHash(
                StatementType::Ballots,
            ))
        } else {
            // Ok(Ballots::<C>::strand_deserialize(&entry.1)?)
            Ok(&entry.1)
        }
    }

    pub(crate) fn get_mix(
        &self,
        m_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<&Mix<C>, ProtocolError> {
        let aei =
            self.get_artifact_entry_identifier_ext(StatementType::Mix, signer_position, batch, 0);
        let entry = self
            .mixes
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Mix))?;
        if m_h.0 != entry.0 {
            Err(ProtocolError::MismatchedArtifactHash(StatementType::Mix))
        } else {
            // Ok(Mix::<C>::strand_deserialize(&entry.1)?)
            Ok(&entry.1)
        }
    }

    pub(crate) fn get_decryption_factors(
        &self,
        m_h: &DecryptionFactorsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<&DecryptionFactors<C>, ProtocolError> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::DecryptionFactors,
            signer_position,
            batch,
            0,
        );
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
            // Ok(DecryptionFactors::<C>::strand_deserialize(&entry.1)?)
            Ok(&entry.1)
        }
    }

    pub(crate) fn get_plaintexts(
        &self,
        m_h: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Result<&Plaintexts<C>, ProtocolError> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Plaintexts,
            signer_position,
            batch,
            0,
        );
        let entry = self
            .plaintexts
            .get(&aei)
            .ok_or(ProtocolError::MissingArtifact(StatementType::Plaintexts))?;
        if m_h.0 != entry.0 {
            Err(ProtocolError::MismatchedArtifactHash(
                StatementType::Plaintexts,
            ))
        } else {
            // Ok(Plaintexts::<C>::strand_deserialize(&entry.1)?)
            Ok(&entry.1)
        }
    }

    // FIXME "outside" function
    // Used to get the public key from the outside
    pub(crate) fn get_dkg_public_key_nohash(
        &self,
        signer_position: TrusteePosition,
    ) -> Option<DkgPublicKey<C>> {
        let aei =
            self.get_artifact_entry_identifier_ext(StatementType::PublicKey, signer_position, 0, 0);
        let entry = self.artifacts.get(&aei)?;
        DkgPublicKey::<C>::strand_deserialize(&entry.1).ok()
    }

    // // FIXME "outside" function
    // Used to get the plaintexts from the outside
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
        let entry = self.artifacts.get(&aei)?;

        Plaintexts::<C>::strand_deserialize(&entry.1).ok()
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
