use anyhow::anyhow;
use anyhow::Result;
use log::{debug, error, trace, warn};
use std::collections::HashMap;

use strand::context::Ctx;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use crate::protocol2::artifact::Ballots;
use crate::protocol2::artifact::Commitments;
use crate::protocol2::artifact::Configuration;
use crate::protocol2::artifact::DecryptionFactors;
use crate::protocol2::artifact::Plaintexts;
use crate::protocol2::artifact::Shares;
use crate::protocol2::hash_from_vec;
use crate::protocol2::message::VerifiedMessage;
use crate::protocol2::predicate::CommitmentsHash;
use crate::protocol2::predicate::ConfigurationHash;
use crate::protocol2::predicate::PublicKeyHash;
use crate::protocol2::predicate::SharesHash;
use crate::protocol2::statement::{ArtifactType, Statement, StatementType};
use crate::protocol2::Hash;

use crate::protocol2::artifact::{DkgPublicKey, Mix};
use crate::protocol2::predicate::{BatchNumber, PlaintextsHash, TrusteePosition};
use crate::protocol2::predicate::{CiphertextsHash, DecryptionFactorsHash};

///////////////////////////////////////////////////////////////////////////
// LocalBoard
///////////////////////////////////////////////////////////////////////////

// A LocalBoard is a trustee's local copy of a bulletin board. It is specific to a protocol
// execution (session_id), referenced in the configuration
//
// Messages are composed of statements and optionally artifacts
//
#[derive(Clone)] // FIXME used by dbg
pub struct LocalBoard<C: Ctx> {
    pub(crate) configuration: Option<Configuration<C>>,
    cfg_hash: Option<Hash>,

    // All keys contain a statement type and a sender. For multi instance predicates
    // (eg multiple decryption/mixing), they also have a batch (usize)
    //
    // We put the hash in the value so that we can detect overwrite attempt,
    // the statement hash is checked on retrieval (it's not in the key)
    pub(crate) statements: HashMap<StatementEntryIdentifier, (Hash, Statement)>,

    // Artifacts entries include their source statement plus adding the ArtifactType
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
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add messages to LocalBoard
    ///////////////////////////////////////////////////////////////////////////

    pub fn add(&mut self, message: VerifiedMessage) -> Result<()> {
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

    fn add_bootstrap(&mut self, message: VerifiedMessage) -> Result<()> {
        let cfg_hash = message.statement.get_cfg_h();

        if self.configuration.is_none() {
            let artifact_bytes = &message
                .artifact
                .ok_or(anyhow!("Missing artifact in configuration message"))?
                .1;
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
                return Err(anyhow!("Failed deserializing configuration"));
            }
        }

        let message_hash = self
            .cfg_hash
            .expect("unexpected: cfg_hash always exists when cfg exists");

        if message_hash == cfg_hash {
            warn!("Configuration received when identical present, ignored");
        } else {
            error!("Configuration overwrite attempt, ignored");
        }

        Err(anyhow!("Artifact already present"))
    }

    ///////////////////////////////////////////////////////////////////////////
    // All other statements
    //
    // Other statements, including _signed_ configuration
    ///////////////////////////////////////////////////////////////////////////

    fn add_message(&mut self, message: VerifiedMessage) -> Result<()> {
        let bytes = message.statement.strand_serialize()?;
        let statement_hash = strand::util::hash(&bytes);

        let statement_identifier =
            self.get_statement_entry_identifier(&message.statement, message.signer_position);
        let statement_entry = self.statements.get(&statement_identifier);

        if let Some((existing_hash, _)) = statement_entry {
            if statement_hash == existing_hash {
                debug!(
                    "Statement identifier already exists (identical): {:?}",
                    statement_identifier
                );
            } else {
                error!(
                    "Statement identifier already exists (overwrite): {:?}, message was {:?}",
                    statement_identifier, message
                );
            }

            Err(anyhow!("Statement already present"))
        } else {
            debug!(
                "Statement identifier is new: {:?}",
                statement_identifier.kind
            );

            // The statement is new, we check the artifact
            if let Some((artifact_type, artifact)) = message.artifact {
                let artifact_identifier =
                    self.get_artifact_entry_identifier(&statement_identifier, &artifact_type);
                let artifact_hash = strand::util::hash_array(&artifact);
                trace!(
                    "Artifact found with hash {}",
                    hex::encode(artifact_hash)[0..10].to_string()
                );

                let artifact_entry = self.artifacts.get(&artifact_identifier);

                if let Some((existing_hash, _)) = artifact_entry {
                    if artifact_hash == *existing_hash {
                        warn!("Artifact identical, ignored");
                    } else {
                        error!("Artifact overwrite attempt, ignored");
                    }

                    Err(anyhow!("Artifact already present"))
                } else {
                    debug!(
                        "Artifact identifier is new: {:?}",
                        artifact_identifier.statement_entry.kind
                    );

                    // Both statement and artifact are new, insert
                    self.statements.insert(
                        statement_identifier,
                        (hash_from_vec(&statement_hash)?, message.statement),
                    );

                    self.artifacts
                        .insert(artifact_identifier, (artifact_hash, artifact));
                    debug!("Artifact inserted");

                    Ok(())
                }
            } else {
                // Only a statement was sent, insert
                self.statements.insert(
                    statement_identifier,
                    (hash_from_vec(&statement_hash)?, message.statement),
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

    pub(crate) fn get_commitments(
        &self,
        commitments_h: &CommitmentsHash,
        signer_position: TrusteePosition,
    ) -> Option<Commitments<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Commitments,
            signer_position,
            0,
            0,
            &ArtifactType::Commitments,
        );
        let entry = self.artifacts.get(&aei)?;
        if commitments_h.0 != entry.0 {
            warn!("Hash mismatch when attempting to retrieve commitments");
            None
        } else {
            Commitments::<C>::strand_deserialize(&entry.1).ok()
        }
    }

    pub(crate) fn get_shares(
        &self,
        shares_h: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Option<Shares> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Shares,
            signer_position,
            0,
            0,
            &ArtifactType::Shares,
        );
        let entry = self.artifacts.get(&aei)?;
        if shares_h.0 != entry.0 {
            warn!("Hash mismatch when attempting to retrieve shares");
            None
        } else {
            Shares::strand_deserialize(&entry.1).ok()
        }
    }

    pub(crate) fn get_dkg_public_key(
        &self,
        pk_h: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Option<DkgPublicKey<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::PublicKey,
            signer_position,
            0,
            0,
            &ArtifactType::PublicKey,
        );
        let entry = self.artifacts.get(&aei)?;
        if pk_h.0 != entry.0 {
            warn!("Hash mismatch when attempting to retrieve public key");
            None
        } else {
            DkgPublicKey::<C>::strand_deserialize(&entry.1).ok()
        }
    }

    pub(crate) fn get_ballots(
        &self,
        b_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Ballots<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Ballots,
            signer_position,
            batch,
            0,
            &ArtifactType::Ballots,
        );
        let entry = self.artifacts.get(&aei)?;
        if b_h.0 != entry.0 {
            warn!("Hash mismatch when attempting to retrieve ballots");
            None
        } else {
            Ballots::<C>::strand_deserialize(&entry.1).ok()
        }
    }

    pub(crate) fn get_mix(
        &self,
        m_h: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Mix<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Mix,
            signer_position,
            batch,
            0,
            &ArtifactType::Mix,
        );
        let entry = self.artifacts.get(&aei)?;
        if m_h.0 != entry.0 {
            warn!("Hash mismatch when attempting to retrieve mix");
            None
        } else {
            Mix::<C>::strand_deserialize(&entry.1).ok()
        }
    }

    pub(crate) fn get_decryption_factors(
        &self,
        m_h: &DecryptionFactorsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<DecryptionFactors<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::DecryptionFactors,
            signer_position,
            batch,
            0,
            &ArtifactType::DecryptionFactors,
        );
        let entry = self.artifacts.get(&aei)?;
        if m_h.0 != entry.0 {
            warn!("Hash mismatch when attempting to retrieve decryption factors");
            None
        } else {
            DecryptionFactors::<C>::strand_deserialize(&entry.1).ok()
        }
    }

    pub(crate) fn get_plaintexts(
        &self,
        m_h: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Plaintexts,
            signer_position,
            batch,
            0,
            &ArtifactType::Plaintexts,
        );
        let entry = self.artifacts.get(&aei)?;
        if m_h.0 != entry.0 {
            warn!("Hash mismatch when attempting to retrieve plaintexts");
            None
        } else {
            Plaintexts::<C>::strand_deserialize(&entry.1).ok()
        }
    }

    // FIXME "outside" function
    // Used to get the public key from the outside
    pub fn get_dkg_public_key_nohash(
        &self,
        signer_position: TrusteePosition,
    ) -> Option<DkgPublicKey<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::PublicKey,
            signer_position,
            0,
            0,
            &ArtifactType::PublicKey,
        );
        let entry = self.artifacts.get(&aei)?;
        DkgPublicKey::<C>::strand_deserialize(&entry.1).ok()
    }

    // // FIXME "outside" function
    // Used to get the plaintexts from the outside
    pub fn get_plaintexts_nohash(
        &self,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        let aei = self.get_artifact_entry_identifier_ext(
            StatementType::Plaintexts,
            signer_position,
            batch,
            0,
            &ArtifactType::Plaintexts,
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
        let (kind, _, batch, mix_signature_number, _, _) = statement.get_data();

        StatementEntryIdentifier {
            kind,
            signer_position,
            batch,
            mix_signature_number,
        }
    }
    pub(crate) fn get_artifact_entry_identifier(
        &self,
        statement_entry: &StatementEntryIdentifier,
        artifact_type: &ArtifactType,
    ) -> ArtifactEntryIdentifier {
        self.get_artifact_entry_identifier_ext(
            statement_entry.kind.clone(),
            statement_entry.signer_position,
            statement_entry.batch,
            statement_entry.mix_signature_number,
            &artifact_type.clone(),
        )
    }

    pub(crate) fn get_artifact_entry_identifier_ext(
        &self,
        statement_type: StatementType,
        signer_position: usize,
        batch: usize,
        mix_signature_number: usize,
        artifact_type: &ArtifactType,
    ) -> ArtifactEntryIdentifier {
        let sti = StatementEntryIdentifier {
            kind: statement_type,
            signer_position,
            batch,
            mix_signature_number,
        };
        ArtifactEntryIdentifier {
            statement_entry: sti,
            artifact_type: artifact_type.clone(),
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
    // The need to make this distinction is only for the purposes of storage in the local board,
    // without this field, the different signature statements would be rejected as duplicates.
    // The field is not used explicitly besides this purpose.
    pub mix_signature_number: usize,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct ArtifactEntryIdentifier {
    pub statement_entry: StatementEntryIdentifier,
    pub artifact_type: ArtifactType,
}
