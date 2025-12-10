// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use cryptography::context::Context;
use cryptography::utils::serialization::VSerializable;
use cryptography::VSerializable as VSer;
use cryptography::utils::signatures::SignatureScheme;
use cryptography::utils::error::Error as CryptoError;
use sha3::Digest;

use crate::messages::artifact::*;
use crate::messages::statement::Statement;
use crate::messages::statement::StatementType;
use crate::messages::newtypes::*;
use crate::{CryptographicContext, Signature, VerifyingKey};

///////////////////////////////////////////////////////////////////////////
// Message
///////////////////////////////////////////////////////////////////////////

#[derive(VSer)]
pub struct Message {
    pub sender: Sender,
    pub signature: Signature,
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

    pub fn bootstrap_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        manager: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();
        let statement = Statement::configuration_stmt(ConfigurationHash(cfg_h));

        manager.sign(statement, Some(cfg_bytes))
    }

    pub fn configuration_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();

        let statement = Statement::configuration_signed_stmt(ConfigurationHash(cfg_h));

        trustee.sign(statement, None)
    }

    pub fn channel_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        channel: &Channel<C>,
        artifact: bool,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();
        
        let commitments_bytes = channel.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&commitments_bytes);
        let commitments_hash = hasher.finalize();
        
        let statement =
            Statement::channel_stmt(ConfigurationHash(cfg_h), ChannelHash(commitments_hash));

        if artifact {
            trustee.sign(statement, Some(commitments_bytes))
        } else {
            trustee.sign(statement, None)
        }
    }

    // Signs all the commitments for all trustees
    pub fn channels_all_signed_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        commitments_hs: &ChannelsHashes,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();

        let statement = Statement::channels_all_stmt(
            ConfigurationHash(cfg_h),
            ChannelsHashes(commitments_hs.0),
        );

        trustee.sign(statement, None)
    }

    // Shares sent from one trustee to all trustees
    pub fn shares_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        shares: &Shares<C>,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();
        
        let share_bytes = shares.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&share_bytes);
        let shares_h = hasher.finalize();

        let statement = Statement::shares_stmt(ConfigurationHash(cfg_h), SharesHash(shares_h));

        trustee.sign(statement, Some(share_bytes))
    }

    pub fn public_key_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        dkgpk: &DkgPublicKey<C>,
        shares_hs: &SharesHashes,
        commitments_hs: &ChannelsHashes,
        artifact: bool,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();
        
        let pk_bytes = dkgpk.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&pk_bytes);
        let pk_h = hasher.finalize();

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

    pub fn ballots_msg<C: Context, S: Signer<C>, const W: usize>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        ballots: &Ballots<C, W>,
        selected_trustees: TrusteeSet,
        pk_h: PublicKeyHash,
        pm: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();
        
        let ballots_bytes = ballots.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&ballots_bytes);
        let bb_h = hasher.finalize();

        let statement = Statement::ballots_stmt(
            ConfigurationHash(cfg_h),
            CiphertextsHash(bb_h),
            PublicKeyHash(pk_h.0),
            batch,
            selected_trustees,
        );
        pm.sign(statement, Some(ballots_bytes))
    }

    pub fn mix_msg<C: Context, S: Signer<C>, const W: usize>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        // Points to either Ballots or Mix
        previous_ciphertexts_h: CiphertextsHash,
        mix: &Mix<C, W>,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();
        
        let mix_bytes = mix.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&mix_bytes);
        let mix_h = hasher.finalize();

        let statement = Statement::mix_stmt(
            ConfigurationHash(cfg_h),
            CiphertextsHash(previous_ciphertexts_h.0),
            CiphertextsHash(mix_h),
            batch,
            mix.mix_number,
        );
        trustee.sign(statement, Some(mix_bytes))
    }

    pub fn mix_signed_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        // Points to either Ballots or Mix
        previous_ciphertexts_h: CiphertextsHash,
        mix_h: CiphertextsHash,
        mix_number: MixNumber,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();

        let statement = Statement::mix_signed_stmt(
            ConfigurationHash(cfg_h),
            CiphertextsHash(previous_ciphertexts_h.0),
            CiphertextsHash(mix_h.0),
            batch,
            mix_number,
        );
        trustee.sign(statement, None)
    }

    pub fn decryption_factors_msg<C: Context, S: Signer<C>, const W: usize>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        dfactors: PartialDecryption<C, W>,
        mix_h: CiphertextsHash,
        shares_hs: SharesHashes,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();

        let dfactors_bytes = dfactors.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&dfactors_bytes);
        let dfactors_h = hasher.finalize();

        let statement = Statement::decryption_factors_stmt(
            ConfigurationHash(cfg_h),
            batch,
            DecryptionFactorsHash(dfactors_h),
            CiphertextsHash(mix_h.0),
            SharesHashes(shares_hs.0),
        );

        trustee.sign(statement, Some(dfactors_bytes))
    }

    pub fn plaintexts_msg<C: Context, S: Signer<C>, const W: usize>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        plaintexts: Plaintexts<C, W>,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();

        let plaintexts_bytes = plaintexts.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&plaintexts_bytes);
        let plaintexts_h = hasher.finalize();

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

    pub fn plaintexts_signed_msg<C: Context, S: Signer<C>>(
        cfg: &Configuration<C>,
        batch: BatchNumber,
        plaintexts_h: PlaintextsHash,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
        trustee: &S,
    ) -> Result<Message, CryptoError> {
        let cfg_bytes = cfg.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&cfg_bytes);
        let cfg_h = hasher.finalize();

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
    pub fn verify<C: Context>(&self, configuration: &Configuration<C>) -> Result<VerifiedMessage> {
        let (kind, st_cfg_h, _, mix_no, _) = self.statement.get_data();

        if mix_no > configuration.trustees.len() {
            return Err(anyhow!(
                "Received a message whose mix signature number is out of range"
            ));
        }

        // Convert concrete VerifyingKey to generic verifier type
        use cryptography::utils::serialization::{VSerializable, VDeserializable};
        let pk_bytes = self.sender.pk.ser();
        let generic_pk: <C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier = 
            VDeserializable::deser(&pk_bytes)
                .map_err(|e| anyhow!("Failed to deserialize sender public key: {:?}", e))?;

        // We don't care about doing a sequential search here as the size is small
        let index: usize = configuration
            .get_trustee_position(&generic_pk)
            .ok_or(anyhow!(
                "Received a message from a trustee that is not part of the configuration {:?}",
                self.sender.pk
            ))?;

        let bytes = self.statement.ser();
        // Verify signature using the verifier from the configuration
        use cryptography::utils::signatures::Verifier;
        let verifier = if index == PROTOCOL_MANAGER_INDEX as usize {
            &configuration.protocol_manager
        } else {
            &configuration.trustees[index]
        };
        
        // Convert concrete signature to generic type
        let sig_bytes = self.signature.ser();
        let generic_sig: <C::SignatureScheme as SignatureScheme<C::Rng>>::Signature = 
            VDeserializable::deser(&sig_bytes)
                .map_err(|e| anyhow!("Failed to deserialize signature: {:?}", e))?;
        
        let verified = verifier.verify(&bytes, &generic_sig);

        if verified.is_err() {
            return Err(anyhow!(
                "Signature verification failed for message {:?}",
                self
            ));
        }
        let trustee = index;

        // The message must belong to the same context as the configuration
        let config_bytes = configuration.ser();
        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(&config_bytes);
        let config_hash = hasher.finalize();
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

        let mut hasher = CryptographicContext::get_hasher();
        hasher.update(artifact);
        let artifact_hash = hasher.finalize();
        // If the cfg_h field matches the artifact, the artifact must be Configuration
        if st_cfg_h == artifact_hash {
            assert!(kind == StatementType::Configuration);
            if trustee != PROTOCOL_MANAGER_INDEX as usize {
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
                if trustee != PROTOCOL_MANAGER_INDEX as usize {
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
            signature: self.signature.clone(),
            statement: self.statement.clone(),
            artifact: self.artifact.clone(),
        };

        Ok(ret)
    }
}

// Placeholder for possible further verifications
fn verify_artifact<C: Context>(
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
pub trait Signer<C: Context> {
    fn get_signing_key(&self) -> &<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer;
    fn get_name(&self) -> String;
    fn sign(
        &self,
        statement: Statement,
        artifact: Option<Vec<u8>>,
    ) -> Result<Message, CryptoError> {
        use cryptography::utils::signatures::{Signer as CryptoSigner, SignatureScheme as SigScheme};
        use cryptography::utils::serialization::{VSerializable, VDeserializable};
        use crate::Signature as ConcreteSignature;
        
        let sk = self.get_signing_key();
        let bytes = statement.ser();
        let signature_generic = sk.sign(&bytes);
        
        // Convert to concrete signature type
        let signature_bytes = signature_generic.ser();
        let signature: ConcreteSignature = VDeserializable::deser(&signature_bytes)
            .map_err(|e| CryptoError::DeserializationError(format!("Signature conversion failed: {:?}", e)))?;
        
        // Get verifying key and convert to concrete type
        let pk_generic = C::SignatureScheme::verifying_key(sk);
        let pk_bytes = pk_generic.ser();
        let pk: VerifyingKey = VDeserializable::deser(&pk_bytes)
            .map_err(|e| CryptoError::DeserializationError(format!("VerifyingKey conversion failed: {:?}", e)))?;
        
        let sender = Sender::new(self.get_name(), pk);

        Ok(Message {
            sender,
            signature,
            statement,
            artifact,
        })
    }
}

#[derive(VSer)]
pub struct Sender {
    pub name: String,
    pub pk: VerifyingKey,
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Sender {
            name: self.name.clone(),
            pk: self.pk.clone(),
        }
    }
}

impl Sender {
    pub fn new(name: String, pk: VerifyingKey) -> Sender {
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
