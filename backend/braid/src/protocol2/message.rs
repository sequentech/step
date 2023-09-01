use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use strand::context::Ctx;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignature, StrandSignaturePk};

use crate::protocol2::{artifact::*, PROTOCOL_MANAGER_INDEX};
use crate::protocol2::statement::ArtifactType;
use crate::protocol2::statement::Statement;
use crate::protocol2::statement::StatementType;
use crate::protocol2::trustee::Signer;

use crate::protocol2::artifact::Configuration;
use crate::protocol2::datalog::PublicKeyHash;
use crate::protocol2::predicate::BatchNumber;
use crate::protocol2::predicate::CiphertextsHash;
use crate::protocol2::predicate::CommitmentsHashes;
use crate::protocol2::predicate::DecryptionFactorsHashes;
use crate::protocol2::predicate::MixNumber;
use crate::protocol2::predicate::PlaintextsHash;
use crate::protocol2::predicate::SharesHashes;
use crate::protocol2::trustee::ProtocolManager;
use crate::protocol2::trustee::Trustee;

use crate::protocol2::statement::*;

use super::datalog::TrusteeSet;

///////////////////////////////////////////////////////////////////////////
// Message
///////////////////////////////////////////////////////////////////////////

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Message {
    pub signer_key: StrandSignaturePk,
    pub signature: StrandSignature,
    pub statement: Statement,
    pub artifact: Option<Vec<u8>>,
}
impl Message {
    ///////////////////////////////////////////////////////////////////////////
    // Message construction
    //
    // Message data is constructed here and then passed on to trustees that
    // construct and sign them. Statements are obtained from static Statement
    // functions.
    ///////////////////////////////////////////////////////////////////////////

    pub fn bootstrap_msg<C: Ctx>(
        cfg: &Configuration<C>,
        manager: &ProtocolManager<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);
        let statement = Statement::configuration_stmt(ConfigurationH(cfg_h));

        manager.sign(statement, Some(cfg_bytes))
    }

    pub(crate) fn configuration_msg<C: Ctx>(
        cfg: &Configuration<C>,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);

        let statement = Statement::configuration_signed_stmt(ConfigurationH(cfg_h));

        trustee.sign(statement, None)
    }

    pub(crate) fn commitments_msg<C: Ctx>(
        cfg: &Configuration<C>,
        commitments: &Commitments<C>,
        artifact: bool,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);
        let commitments_bytes = commitments.strand_serialize()?;
        let commitments_hash = strand::util::hash_array(&commitments_bytes);
        let statement =
            Statement::commitments_stmt(ConfigurationH(cfg_h), CommitmentsH(commitments_hash));

        if artifact {
            trustee.sign(statement, Some(commitments_bytes))
        } else {
            trustee.sign(statement, None)
        }
    }

    // Signs all the commitments for all trustees
    pub(crate) fn commitments_all_signed_msg<C: Ctx>(
        cfg: &Configuration<C>,
        commitments_hs: &CommitmentsHashes,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);

        let statement =
            Statement::commitments_all_stmt(ConfigurationH(cfg_h), CommitmentsHs(commitments_hs.0));

        trustee.sign(statement, None)
    }

    // Shares sent from one trustee to all trustees
    pub(crate) fn shares_msg<C: Ctx>(
        cfg: &Configuration<C>,
        shares: &Shares,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);
        let share_bytes = shares.strand_serialize()?;
        let shares_h = strand::util::hash_array(&share_bytes);

        let statement = Statement::shares_stmt(ConfigurationH(cfg_h), SharesH(shares_h));

        trustee.sign(statement, Some(share_bytes))
    }

    pub(crate) fn public_key_msg<C: Ctx>(
        cfg: &Configuration<C>,
        dkgpk: &DkgPublicKey<C>,
        shares_hs: &SharesHashes,
        commitments_hs: &CommitmentsHashes,
        artifact: bool,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);
        let pk_bytes = dkgpk.strand_serialize()?;
        let pk_h = strand::util::hash_array(&pk_bytes);

        // The messages are the same except for the artifact and the statement type
        if artifact {
            let statement = Statement::pk_stmt(
                ConfigurationH(cfg_h),
                PublicKeyH(pk_h),
                SharesHs(shares_hs.0),
                CommitmentsHs(commitments_hs.0),
            );
            trustee.sign(statement, Some(pk_bytes))
        } else {
            let statement = Statement::pk_signed_stmt(
                ConfigurationH(cfg_h),
                PublicKeyH(pk_h),
                SharesHs(shares_hs.0),
                CommitmentsHs(commitments_hs.0),
            );
            trustee.sign(statement, None)
        }
    }

    pub fn ballots_msg<C: Ctx>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        ballots: &Ballots<C>,
        selected_trustees: TrusteeSet,
        pk_h: PublicKeyHash,
        pm: &ProtocolManager<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);
        let ballots_bytes = ballots.strand_serialize()?;
        let bb_h = strand::util::hash_array(&ballots_bytes);

        let statement = Statement::ballots_stmt(
            ConfigurationH(cfg_h),
            CiphertextsH(bb_h),
            PublicKeyH(pk_h.0),
            Batch(batch),
            selected_trustees,
        );
        pm.sign(statement, Some(ballots_bytes))
    }

    pub(crate) fn mix_msg<C: Ctx>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        // Points to either Ballots or Mix
        previous_ciphertexts_h: CiphertextsHash,
        mix: &Mix<C>,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);
        let mix_bytes = mix.strand_serialize()?;
        let mix_h = strand::util::hash_array(&mix_bytes);

        let statement = Statement::mix_stmt(
            ConfigurationH(cfg_h),
            CiphertextsH(previous_ciphertexts_h.0),
            CiphertextsH(mix_h),
            Batch(batch),
            mix.mix_number,
        );
        trustee.sign(statement, Some(mix_bytes))
    }

    pub fn mix_signed_msg<C: Ctx>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        // Points to either Ballots or Mix
        previous_ciphertexts_h: CiphertextsHash,
        mix_h: CiphertextsHash,
        mix_number: MixNumber,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);

        let statement = Statement::mix_signed_stmt(
            ConfigurationH(cfg_h),
            CiphertextsH(previous_ciphertexts_h.0),
            CiphertextsH(mix_h.0),
            Batch(batch),
            mix_number,
        );
        trustee.sign(statement, None)
    }

    pub(crate) fn decryption_factors_msg<C: Ctx>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        dfactors: DecryptionFactors<C>,
        mix_h: CiphertextsHash,
        shares_hs: SharesHashes,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);

        let dfactors_bytes = dfactors.strand_serialize()?;
        let dfactors_h = strand::util::hash_array(&dfactors_bytes);

        let statement = Statement::decryption_factors_stmt(
            ConfigurationH(cfg_h),
            Batch(batch),
            DecryptionFactorsH(dfactors_h),
            CiphertextsH(mix_h.0),
            SharesHs(shares_hs.0),
        );

        trustee.sign(statement, Some(dfactors_bytes))
    }

    pub(crate) fn plaintexts_msg<C: Ctx>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        plaintexts: Plaintexts<C>,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);

        let plaintexts_bytes = plaintexts.strand_serialize()?;
        let plaintexts_h = strand::util::hash_array(&plaintexts_bytes);

        let statement = Statement::plaintexts_stmt(
            ConfigurationH(cfg_h),
            Batch(batch),
            PlaintextsH(plaintexts_h),
            DecryptionFactorsHs(dfactors_hs.0),
            CiphertextsH(cipher_h.0),
            PublicKeyH(pk_h.0)
        );

        trustee.sign(statement, Some(plaintexts_bytes))
    }

    pub(crate) fn plaintexts_signed_msg<C: Ctx>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        plaintexts_h: PlaintextsHash,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
        trustee: &Trustee<C>,
    ) -> Result<Message> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::util::hash_array(&cfg_bytes);

        let statement = Statement::plaintexts_signed_stmt(
            ConfigurationH(cfg_h),
            Batch(batch),
            PlaintextsH(plaintexts_h.0),
            DecryptionFactorsHs(dfactors_hs.0),
            CiphertextsH(cipher_h.0),
            PublicKeyH(pk_h.0)
        );

        trustee.sign(statement, None)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Message verification
    //
    // If valid, returns a VerifiedMessage which includes the sender position.
    // If invalid, returns None
    ///////////////////////////////////////////////////////////////////////////

    // FIXME add check for timestamp not older than some threshold
    pub(crate) fn verify<C: Ctx>(
        self,
        configuration: &Configuration<C>,
    ) -> Result<VerifiedMessage> {
        let (kind, st_cfg_h, _, mix_no, artifact_type, _) = self.statement.get_data();

        if mix_no > configuration.trustees.len() {
            return Err(anyhow!(
                "Received a message whose statement signature number is out of range"
            ));
        }

        // We don't care about doing a sequential search here as the size is small
        let index: usize = configuration
            .get_trustee_position(&self.signer_key)
            .ok_or(anyhow!(
                "Received a message from a trustee that is not part of the configuration {:?}",
                self.signer_key
            ))?;

        let bytes = self.statement.strand_serialize()?;
        // Verify signature
        let trustee_ = self
            .signer_key
            .verify(&self.signature, &bytes)
            .map(|_| index)
            .ok();

        if trustee_.is_none() {
            return Err(anyhow!(
                "Signature verification failed for message {:?}",
                self
            ));
        }
        let trustee = trustee_.expect("impossible");

        // The message must belong to the same context as the configuration
        let config_hash = strand::util::hash(&configuration.strand_serialize()?);
        if config_hash != st_cfg_h {
            return Err(anyhow!(
                "Received message with mismatched configuration hash"
            ));
        }
        assert_eq!(config_hash, st_cfg_h);

        // Statement-only message
        if self.artifact.is_none() {
            return Ok(VerifiedMessage::new(trustee, self.statement, None));
        }
        let artifact = self.artifact.expect("impossible");

        // Artifact present, get the type from the statement

        let artifact_hash = strand::util::hash_array(&artifact);
        // In the case of a Configuration statement, the cfg_h field should match the artifact
        if st_cfg_h == artifact_hash {
            assert!(kind == StatementType::Configuration);
            if trustee != PROTOCOL_MANAGER_INDEX {
                return Err(anyhow!(
                    "Configuration must be signed by protocol manager"
                ));
            }

            let artifact_field = Some((ArtifactType::Configuration, artifact));
            Ok(VerifiedMessage::new(
                trustee,
                self.statement,
                artifact_field,
            ))
        } else {
            // If the statement type were configuration, cfg_hash should have matched the artifact
            assert!(kind != StatementType::Configuration);

            if kind == StatementType::Ballots {
                if trustee != PROTOCOL_MANAGER_INDEX {
                    return Err(anyhow!(
                        "Ballots must be signed by protocol manager"
                    ));
                }
            }

            // Set the type of the artifact field
            if let Some(artifact_type) = artifact_type {
                let _ = verify_artifact(&configuration, &artifact_type, &artifact)?;
                let artifact_field = Some((artifact_type, artifact));
                Ok(VerifiedMessage::new(
                    trustee,
                    self.statement,
                    artifact_field,
                ))
            } else {
                return Err(anyhow!("Could not set artifact type"));
            }
        }
    }
}

fn verify_artifact<C: Ctx>(
    _cfg: &Configuration<C>,
    kind: &ArtifactType,
    _data: &Vec<u8>,
) -> Result<()> {
    match kind {
        ArtifactType::Ballots => {}
        ArtifactType::Commitments => {}
        ArtifactType::DecryptionFactors => {}
        ArtifactType::Mix => {}
        ArtifactType::Plaintexts => {}
        ArtifactType::PublicKey => {}
        ArtifactType::Shares => {}
        ArtifactType::Configuration => {}
    }

    Ok(())
}

///////////////////////////////////////////////////////////////////////////
// VerifiedMessage
///////////////////////////////////////////////////////////////////////////
#[derive()]
pub struct VerifiedMessage {
    pub(crate) signer_position: usize,
    pub(crate) statement: Statement,
    pub(crate) artifact: Option<(ArtifactType, Vec<u8>)>,
}

impl VerifiedMessage {
    pub(crate) fn new(
        signer_position: usize,
        statement: Statement,
        artifact: Option<(ArtifactType, Vec<u8>)>,
    ) -> VerifiedMessage {
        VerifiedMessage {
            signer_position,
            statement,
            artifact,
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Message{{ sender={:?} statement={:?} artifact={}}}",
            self.signer_key,
            &self.statement,
            self.artifact.is_some()
        )
    }
}

impl std::fmt::Debug for VerifiedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VerifiedMessage{{ sender={:?} statement={:?} is artifact={} }}",
            self.signer_position,
            self.statement,
            self.artifact.is_some()
        )
    }
}
