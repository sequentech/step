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

#[cfg(test)]
mod authentication_tests {
    use super::*;
    use crate::messages::protocol_manager::ProtocolManager;
    use std::marker::PhantomData;
    use strand::backend::ristretto::RistrettoCtx;

    #[test]
    fn verification_rejects_modified_artifact_with_valid_statement_signature() {
        let manager = ProtocolManager::<RistrettoCtx>::new(StrandSignatureSk::gen().unwrap());
        let trustees = (0..2)
            .map(|_| StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap())
            .collect();
        let cfg = Configuration::<RistrettoCtx>::new(
            1,
            StrandSignaturePk::from_sk(&manager.signing_key).unwrap(),
            trustees,
            2,
            PhantomData,
        );
        let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
        selected[0] = 1;
        selected[1] = 2;
        let mut message = Message::ballots_msg(
            &cfg,
            1,
            &Ballots::new(vec![]),
            selected,
            PublicKeyHash([0; 64]),
            &manager,
        )
        .unwrap();
        assert!(message.verify(&cfg).is_ok());
        message.artifact.as_mut().unwrap().push(1);
        assert!(
            message.verify(&cfg).is_err(),
            "the artifact must match its signed hash"
        );
    }
}

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
        // Privileged statement kinds must be authorized even without an artifact.
        if matches!(kind, StatementType::Configuration | StatementType::Ballots)
            && trustee != PROTOCOL_MANAGER_INDEX
        {
            return Err(anyhow!(
                "Configuration and ballots must be signed by protocol manager"
            ));
        }

        if let Some(artifact) = &self.artifact {
            let expected = match &self.statement {
                Statement::Configuration(_, h) => h.0,
                Statement::Channel(_, _, h) => h.0,
                Statement::Shares(_, _, h) => h.0,
                Statement::PublicKey(_, _, h, _, _) => h.0,
                Statement::Ballots(_, _, _, h, _, _) => h.0,
                Statement::Mix(_, _, _, _, h, _) => h.0,
                Statement::DecryptionFactors(_, _, _, h, _, _) => h.0,
                Statement::Plaintexts(_, _, _, h, _, _, _) => h.0,
                _ => return Err(anyhow!("Artifact is not allowed for this statement type")),
            };
            if strand::hash::hash_to_array(artifact)? != expected {
                return Err(anyhow!("Artifact does not match its signed hash"));
            }
        }
        Ok(VerifiedMessage::new(
            trustee,
            self.statement.clone(),
            self.artifact.clone(),
        ))
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
