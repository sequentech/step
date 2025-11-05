// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use strand::context::Ctx;
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignature, StrandSignaturePk, StrandSignatureSk};
use strand::util::StrandError;

use crate::messages::artifact::*;
use crate::messages::statement::Statement;
use crate::messages::statement::StatementType;

use crate::messages::newtypes::*;

///////////////////////////////////////////////////////////////////////////
// Message
///////////////////////////////////////////////////////////////////////////

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Message {
    pub sender: Sender,
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

    pub fn bootstrap_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        manager: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;
        let statement = Statement::configuration_stmt(ConfigurationHash(cfg_h));

        manager.sign(statement, Some(cfg_bytes))
    }

    pub fn configuration_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;

        let statement = Statement::configuration_signed_stmt(ConfigurationHash(cfg_h));

        trustee.sign(statement, None)
    }

    pub fn channel_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        channel: &Channel<C>,
        artifact: bool,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;
        let commitments_bytes = channel.strand_serialize()?;
        let commitments_hash = strand::hash::hash_to_array(&commitments_bytes)?;
        let statement =
            Statement::channel_stmt(ConfigurationHash(cfg_h), ChannelHash(commitments_hash));

        if artifact {
            trustee.sign(statement, Some(commitments_bytes))
        } else {
            trustee.sign(statement, None)
        }
    }

    // Signs all the commitments for all trustees
    pub fn channels_all_signed_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        commitments_hs: &ChannelsHashes,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;

        let statement = Statement::channels_all_stmt(
            ConfigurationHash(cfg_h),
            ChannelsHashes(commitments_hs.0),
        );

        trustee.sign(statement, None)
    }

    // Shares sent from one trustee to all trustees
    pub fn shares_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        shares: &Shares<C>,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;
        let share_bytes = shares.strand_serialize()?;
        let shares_h = strand::hash::hash_to_array(&share_bytes)?;

        let statement = Statement::shares_stmt(ConfigurationHash(cfg_h), SharesHash(shares_h));

        trustee.sign(statement, Some(share_bytes))
    }

    pub fn public_key_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        dkgpk: &DkgPublicKey<C>,
        shares_hs: &SharesHashes,
        commitments_hs: &ChannelsHashes,
        artifact: bool,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;
        let pk_bytes = dkgpk.strand_serialize()?;
        let pk_h = strand::hash::hash_to_array(&pk_bytes)?;

        // The messages are the same except for the artifact and the statement type
        if artifact {
            let statement = Statement::pk_stmt(
                ConfigurationHash(cfg_h),
                PublicKeyHash(pk_h),
                SharesHashes(shares_hs.0),
                ChannelsHashes(commitments_hs.0),
            );
            trustee.sign(statement, Some(pk_bytes))
        } else {
            let statement = Statement::pk_signed_stmt(
                ConfigurationHash(cfg_h),
                PublicKeyHash(pk_h),
                SharesHashes(shares_hs.0),
                ChannelsHashes(commitments_hs.0),
            );
            trustee.sign(statement, None)
        }
    }

    pub fn ballots_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        ballots: &Ballots<C>,
        selected_trustees: TrusteeSet,
        pk_h: PublicKeyHash,
        pm: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;
        let ballots_bytes = ballots.strand_serialize()?;
        let bb_h = strand::hash::hash_to_array(&ballots_bytes)?;

        let statement = Statement::ballots_stmt(
            ConfigurationHash(cfg_h),
            CiphertextsHash(bb_h),
            PublicKeyHash(pk_h.0),
            batch,
            selected_trustees,
        );
        pm.sign(statement, Some(ballots_bytes))
    }

    pub fn mix_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        // Points to either Ballots or Mix
        previous_ciphertexts_h: CiphertextsHash,
        mix: &Mix<C>,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;
        let mix_bytes = mix.strand_serialize()?;
        let mix_h = strand::hash::hash_to_array(&mix_bytes)?;

        let statement = Statement::mix_stmt(
            ConfigurationHash(cfg_h),
            CiphertextsHash(previous_ciphertexts_h.0),
            CiphertextsHash(mix_h),
            batch,
            mix.mix_number,
        );
        trustee.sign(statement, Some(mix_bytes))
    }

    pub fn mix_signed_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        // Points to either Ballots or Mix
        previous_ciphertexts_h: CiphertextsHash,
        mix_h: CiphertextsHash,
        mix_number: MixNumber,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;

        let statement = Statement::mix_signed_stmt(
            ConfigurationHash(cfg_h),
            CiphertextsHash(previous_ciphertexts_h.0),
            CiphertextsHash(mix_h.0),
            batch,
            mix_number,
        );
        trustee.sign(statement, None)
    }

    pub fn decryption_factors_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        dfactors: DecryptionFactors<C>,
        mix_h: CiphertextsHash,
        shares_hs: SharesHashes,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;

        let dfactors_bytes = dfactors.strand_serialize()?;
        let dfactors_h = strand::hash::hash_to_array(&dfactors_bytes)?;

        let statement = Statement::decryption_factors_stmt(
            ConfigurationHash(cfg_h),
            batch,
            DecryptionFactorsHash(dfactors_h),
            CiphertextsHash(mix_h.0),
            SharesHashes(shares_hs.0),
        );

        trustee.sign(statement, Some(dfactors_bytes))
    }

    // TODO Add those messages to board
    pub fn plaintexts_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        plaintexts: Plaintexts<C>,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;

        let plaintexts_bytes = plaintexts.strand_serialize()?;
        let plaintexts_h = strand::hash::hash_to_array(&plaintexts_bytes)?;

        let statement = Statement::plaintexts_stmt(
            ConfigurationHash(cfg_h),
            batch,
            PlaintextsHash(plaintexts_h),
            DecryptionFactorsHashes(dfactors_hs.0),
            CiphertextsHash(cipher_h.0),
            PublicKeyHash(pk_h.0),
        );

        trustee.sign(statement, Some(plaintexts_bytes))
    }

    pub fn plaintexts_signed_msg<C: Ctx, S: Signer>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        plaintexts_h: PlaintextsHash,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
        trustee: &S,
    ) -> Result<Message, StrandError> {
        let cfg_bytes = cfg.strand_serialize()?;
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;

        let statement = Statement::plaintexts_signed_stmt(
            ConfigurationHash(cfg_h),
            batch,
            PlaintextsHash(plaintexts_h.0),
            DecryptionFactorsHashes(dfactors_hs.0),
            CiphertextsHash(cipher_h.0),
            PublicKeyHash(pk_h.0),
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
    pub fn verify<C: Ctx>(&self, configuration: &Configuration<C>) -> Result<VerifiedMessage> {
        let (kind, st_cfg_h, _, mix_no, _) = self.statement.get_data();

        if mix_no > configuration.trustees.len() {
            return Err(anyhow!(
                "Received a message whose mix signature number is out of range"
            ));
        }

        // We don't care about doing a sequential search here as the size is small
        let index: usize = configuration
            .get_trustee_position(&self.sender.pk)
            .ok_or(anyhow!(
                "Received a message from a trustee that is not part of the configuration {:?}",
                self.sender.pk
            ))?;

        let bytes = self.statement.strand_serialize()?;
        // Verify signature
        let verified = self.sender.pk.verify(&self.signature, &bytes);

        if verified.is_err() {
            return Err(anyhow!(
                "Signature verification failed for message {:?}",
                self
            ));
        }
        let trustee = index;

        // The message must belong to the same context as the configuration
        let config_hash = strand::hash::hash(&configuration.strand_serialize()?)?;
        if config_hash != st_cfg_h {
            return Err(anyhow!(
                "Received message with mismatched configuration hash"
            ));
        }
        assert_eq!(config_hash, st_cfg_h);

        // Statement-only message
        if self.artifact.is_none() {
            return Ok(VerifiedMessage::new(trustee, self.statement.clone(), None));
        }
        let artifact = self.artifact.as_ref().expect("impossible");
        // Use this to move the bytes out of self to avoid copying below (artifact.clone())
        // This will require taking ownership of self in the method signature
        // let artifact = self.artifact.take().unwrap();

        // Artifact present

        let artifact_hash = strand::hash::hash_to_array(&artifact)?;
        // If the cfg_h field matches the artifact, the artifact must be Configuration
        if st_cfg_h == artifact_hash {
            assert!(kind == StatementType::Configuration);
            if trustee != PROTOCOL_MANAGER_INDEX {
                return Err(anyhow!("Configuration must be signed by protocol manager"));
            }

            // FIXME remove this potentially expensive clone
            // See above line: let artifact = self.artifact.take().unwrap();
            Ok(VerifiedMessage::new(
                trustee,
                self.statement.clone(),
                Some(artifact.clone()),
            ))
        } else {
            // If the statement type were configuration, cfg_hash should have matched the artifact above
            assert!(kind != StatementType::Configuration);

            if kind == StatementType::Ballots {
                if trustee != PROTOCOL_MANAGER_INDEX {
                    return Err(anyhow!("Ballots must be signed by protocol manager"));
                }
            }

            let _ = verify_artifact(&configuration, &kind, &artifact)?;
            // FIXME remove this potentially expensive clone
            // See above line: let artifact = self.artifact.take().unwrap();
            Ok(VerifiedMessage::new(
                trustee,
                self.statement.clone(),
                Some(artifact.clone()),
            ))
        }
    }

    /// Clone this message.
    ///
    /// Clone is fallible when signature is implemented with OpenSSL
    pub fn try_clone(&self) -> Result<Message> {
        let ret = Message {
            sender: self.sender.clone(),
            signature: self.signature.try_clone()?,
            statement: self.statement.clone(),
            artifact: self.artifact.clone(),
        };

        Ok(ret)
    }
}

// Placeholder for possible further verifications
fn verify_artifact<C: Ctx>(
    _cfg: &Configuration<C>,
    kind: &StatementType,
    _data: &Vec<u8>,
) -> Result<()> {
    match kind {
        StatementType::Ballots => {}
        StatementType::Channel => {}
        StatementType::DecryptionFactors => {}
        StatementType::Mix => {}
        StatementType::Plaintexts => {}
        StatementType::PublicKey => {}
        StatementType::Shares => {}
        StatementType::Configuration => {}
        _ => {}
    }

    Ok(())
}

///////////////////////////////////////////////////////////////////////////
// VerifiedMessage
///////////////////////////////////////////////////////////////////////////
#[derive()]
pub struct VerifiedMessage {
    pub signer_position: usize,
    pub statement: Statement,
    pub artifact: Option<Vec<u8>>,
}

impl VerifiedMessage {
    pub(crate) fn new(
        signer_position: usize,
        statement: Statement,
        artifact: Option<Vec<u8>>,
    ) -> VerifiedMessage {
        VerifiedMessage {
            signer_position,
            statement,
            artifact,
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// Signer (commonality to sign messages for Trustee and Protocolmanager)
///////////////////////////////////////////////////////////////////////////
pub trait Signer {
    fn get_signing_key(&self) -> &StrandSignatureSk;
    fn get_name(&self) -> String;
    fn sign(
        &self,
        statement: Statement,
        artifact: Option<Vec<u8>>,
    ) -> Result<Message, StrandError> {
        let sk = self.get_signing_key();
        let bytes = statement.strand_serialize()?;
        let signature: StrandSignature = sk.sign(&bytes)?;
        let pk = StrandSignaturePk::from_sk(sk)?;
        let sender = Sender::new(self.get_name(), pk);

        Ok(Message {
            sender,
            signature,
            statement,
            artifact,
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Sender {
    pub name: String,
    pub pk: StrandSignaturePk,
}
impl Sender {
    pub fn new(name: String, pk: StrandSignaturePk) -> Sender {
        Sender { name, pk }
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
            self.sender.name,
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
