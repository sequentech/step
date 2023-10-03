use anyhow::{anyhow, Context, Result};

use log::{debug, error, info, trace, warn};
use rayon::prelude::*;
use std::collections::HashSet;
use std::marker::PhantomData;
use tracing_attributes::instrument;

use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::{context::Ctx, elgamal::PrivateKey};

use crate::protocol2::action::Action;
use braid_messages::artifact::Channel;
use braid_messages::artifact::Configuration;
use braid_messages::artifact::DkgPublicKey;
use braid_messages::artifact::Shares;
use braid_messages::artifact::{Ballots, DecryptionFactors, Mix, Plaintexts};
use braid_messages::statement::StatementType;
use braid_messages::message::Message;
use braid_messages::newtypes::*;
use crate::protocol2::board::local::LocalBoard;

use crate::protocol2::predicate::Predicate;

use braid_messages::newtypes::PROTOCOL_MANAGER_INDEX;

use strand::symm;

///////////////////////////////////////////////////////////////////////////
// Trustee
//
// Represents the instantiation of a trustee within a specific protocol
// session. Runs the main loop for the trustee's participation in the session.
//
// 1) Receive messages from RemoteBoard
// 2) Update LocalBoard with Statements and Artifacts
// 3) Derive Predicates from Statements on LocalBoard
// 4) Invoke Datalog with input predicates
//      4.1) Pass output predicates from 4) to subsequent datalog Phases
// 5) Run Actions resulting from 4)
// 6) Return resulting Messages for subsequent posting on RemoteBoard
//
// Does not post the messages itself.
///////////////////////////////////////////////////////////////////////////

pub struct Trustee<C: Ctx> {
    pub(crate) signing_key: StrandSignatureSk,
    // A ChaCha20Poly1305 encryption key
    pub(crate) encryption_key: symm::SymmetricKey,
    local_board: LocalBoard<C>,
}

impl<C: Ctx> braid_messages::message::Signer for Trustee<C> {
    fn get_signing_key(&self) -> &StrandSignatureSk {
        &self.signing_key
    }
}

impl<C: Ctx> Trustee<C> {
    pub fn new(signing_key: StrandSignatureSk, encryption_key: symm::SymmetricKey) -> Trustee<C> {
        let local_board = LocalBoard::new();

        Trustee {
            signing_key,
            encryption_key,
            local_board,
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Protocol step: update->derive predicates->infer&run
    ///////////////////////////////////////////////////////////////////////////

    #[instrument(name = "Trustee::step", skip(messages, self))]
    pub(crate) fn step(
        &mut self,
        messages: Vec<Message>,
    ) -> Result<(Vec<Message>, HashSet<Action>)> {
        let added_messages = self.update_local_board(messages)?;

        info!("Update added {} messages", added_messages);
        let predicates = self.derive_predicates(false)?;
        info!("Derived {} predicates", predicates.len());
        let (messages, actions, _) = self.infer_and_run_actions(&predicates, false)?;

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

        Ok((messages, actions))
    }

    ///////////////////////////////////////////////////////////////////////////
    // Update
    ///////////////////////////////////////////////////////////////////////////

    #[instrument(name = "Trustee::update_local_board", skip_all, level = "trace")]
    fn update_local_board(&mut self, messages: Vec<Message>) -> Result<i32> {
        trace!("Updating with {} messages", messages.len());

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
    ///////////////////////////////////////////////////////////////////////////
    fn update(&mut self, messages: Vec<Message>, configuration: Configuration<C>) -> Result<i32> {
        let mut added = 0;

        // Sanity check: field cfg_hash must exist at this point
        let cfg_hash = self.local_board.get_cfg_hash();
        if cfg_hash.is_none() {
            return Err(anyhow!("Local field cfg_hash not set")); 
        }
        
        let cfg_hash = cfg_hash.expect("impossible");

        for message in messages {
            let verified = message.verify(&configuration);

            if verified.is_err() {
                warn!("Message failed verification {:?}", verified);
                // FIXME will crash on bad data
                panic!();
            }
            let verified = verified.expect("impossible");

            if verified.statement.get_cfg_h() != cfg_hash {
                warn!("Message has mismatched configuration hash");
                // FIXME will crash on bad data
                panic!();
            }

            let _ = self.local_board.add(verified)?;
            added += 1;
        }

        Ok(added)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Bootstrapping update
    //
    // There is no configuration. We retrieve message zero, check that it's the
    // configuration and add it to the local board.
    ///////////////////////////////////////////////////////////////////////////
    fn update_bootstrap(&mut self, mut messages: Vec<Message>) -> Result<i32> {
        let mut added = 0;

        trace!("Configuration not present in board, getting first remote message");
        if messages.is_empty() {
            return Err(anyhow!(
                "Zero messages received, cannot retrieve configuration"
            ));
        }
        let zero = messages.remove(0);

        if zero.statement.get_kind() != StatementType::Configuration {
            return Err(anyhow!("Invalid statement type for zeroth message"));
        }

        if zero.artifact.is_none() {
            return Err(anyhow!("No artifact for configuration message"));
        }

        let artifact = zero.artifact.as_ref().expect("impossible");
        let configuration = Configuration::strand_deserialize(artifact)?;
        // FIXME will crash on bad data
        assert!(configuration.is_valid());

        let verified = zero.verify(&configuration)?;

        assert!(verified.signer_position == PROTOCOL_MANAGER_INDEX);
        trace!("Verified signature, Configuration signed by Protocol Manager");

        // The configuration is not verified here, but in the SignConfiguration action
        let added_ = self.local_board.add(verified);
        if added_.is_ok() {
            added += 1;
        } else {
            return Err(anyhow!("Configuration should have been added"));
        }
        // Process the rest of the messages
        if !messages.is_empty() {
            return self.update(messages, configuration);
        }

        Ok(added)
    }

    ///////////////////////////////////////////////////////////////////////////
    // derive
    ///////////////////////////////////////////////////////////////////////////
    #[instrument(name = "Trustee::derive_predicates", skip(self), level = "trace")]
    fn derive_predicates(&self, verifying_mode: bool) -> Result<Vec<Predicate>> {
        let mut predicates = vec![];

        let configuration = self.local_board.get_configuration_raw();
        let configuration =
            configuration.ok_or(anyhow!("Cannot derive predicates without a configuration"))?;

        let configuration_p_ = if !verifying_mode {
            Predicate::get_bootstrap_predicate(&configuration, &self.get_pk()?)
        } else {
            Predicate::get_verifier_bootstrap_predicate(&configuration)
        };

        let configuration_p =
            configuration_p_.ok_or(anyhow!("Self authority not found in configuration"))?;
        predicates.push(configuration_p);
        trace!("Adding bootstrap predicate {:?}", configuration_p);

        let entries = self.local_board.get_entries();

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
                Predicate::from_statement(statement, entry.key.signer_position, &configuration);
            predicates.push(next);
        }

        trace!("Derived {} predicates", predicates.len());

        Ok(predicates)
    }

    ///////////////////////////////////////////////////////////////////////////
    // infer&run
    ///////////////////////////////////////////////////////////////////////////

    fn infer_and_run_actions(
        &self,
        predicates: &Vec<Predicate>,
        verifying_mode: bool,
    ) -> Result<(Vec<Message>, HashSet<Action>, Vec<Predicate>)> {
        let _ = self
            .local_board
            .get_configuration_raw()
            .ok_or(anyhow!("Cannot run actions without a configuration"))?;

        let (actions, predicates) = crate::protocol2::datalog::run(predicates);
        trace!(
            "Datalog derived {} actions, {:?}",
            actions.len(),
            actions
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<String>>()
        );

        let ret_actions = actions.clone();

        if actions.len() == 0 {
            info!("-- Idle --");
        }

        // Cross-Action parallelism (which in effect is cross-batch parallelism)
        let results: Result<Vec<Vec<Message>>> = actions
            .into_par_iter()
            .map(|a| {
                let m = if !verifying_mode {
                    a.run(self)
                } else {
                    a.run_for_verifier(self)
                };

                if m.is_err() {
                    error!("Action {:?} returned error {:?} (propagating)", a, m);
                    m.with_context(|| format!("When executing Action {:?}", a))
                } else {
                    m
                }
            })
            .collect();

        // flatten all messages
        let result = results?.into_iter().flatten().collect();

        Ok((result, ret_actions, predicates))
    }

    ///////////////////////////////////////////////////////////////////////////
    // Trustee verifying mode
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn verify(
        &mut self,
        messages: Vec<Message>,
    ) -> Result<(Vec<Message>, Vec<Predicate>)> {
        self.update_local_board(messages)?;

        let predicates = self.derive_predicates(true)?;
        let (messages, _, predicates) = self.infer_and_run_actions(&predicates, true)?;

        Ok((messages, predicates))
    }

    ///////////////////////////////////////////////////////////////////////////
    // Artifact accessors for Actions
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn get_configuration(&self, hash: &ConfigurationHash) -> Result<&Configuration<C>> {
        self.local_board
            .get_configuration(hash)
            .ok_or(anyhow!("Could not retrieve configuration",))
    }

    // FIXME Used by dbg::status, remove
    pub(crate) fn copy_local_board(&self) -> LocalBoard<C> {
        self.local_board.clone()
    }

    pub(crate) fn get_channel(
        &self,
        hash: &ChannelHash,
        signer_position: TrusteePosition,
    ) -> Option<Channel<C>> {
        self.local_board.get_channel(hash, signer_position)
    }

    pub(crate) fn get_shares(
        &self,
        hash: &SharesHash,
        signer_position: TrusteePosition,
    ) -> Option<Shares<C>> {
        self.local_board.get_shares(hash, signer_position)
    }

    pub(crate) fn get_dkg_public_key(
        &self,
        hash: &PublicKeyHash,
        signer_position: TrusteePosition,
    ) -> Option<DkgPublicKey<C>> {
        self.local_board.get_dkg_public_key(hash, signer_position)
    }

    pub(crate) fn get_ballots(
        &self,
        hash: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Ballots<C>> {
        self.local_board.get_ballots(hash, batch, signer_position)
    }

    pub(crate) fn get_mix(
        &self,
        hash: &CiphertextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Mix<C>> {
        self.local_board.get_mix(hash, batch, signer_position)
    }

    pub(crate) fn get_decryption_factors(
        &self,
        hash: &DecryptionFactorsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<DecryptionFactors<C>> {
        self.local_board
            .get_decryption_factors(hash, batch, signer_position)
    }

    pub(crate) fn get_plaintexts(
        &self,
        hash: &PlaintextsHash,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        self.local_board
            .get_plaintexts(hash, batch, signer_position)
    }

    // FIXME "outside" function
    pub fn get_dkg_public_key_nohash(&self) -> Option<DkgPublicKey<C>> {
        self.local_board.get_dkg_public_key_nohash(0)
    }

    // FIXME "outside" function
    pub fn get_plaintexts_nohash(
        &self,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        self.local_board
            .get_plaintexts_nohash(batch, signer_position)
    }

    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn is_config_approved(&self, _config: &Configuration<C>) -> bool {
        // FIXME validate (called by action/cfg)
        true
    }

    pub fn get_pk(&self) -> Result<StrandSignaturePk> {
        Ok(StrandSignaturePk::from(&self.signing_key)?)
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "fips")] {
            pub(crate) fn encrypt_share_sk(&self, sk: &PrivateKey<C>, cfg: &Configuration<C>) -> Result<Channel<C>> {
                let identifier: String = self.get_pk()?.try_into()?;
                // 0 is a dummy batch value
                let aad = cfg.label(0, format!("encrypted by {}", identifier));
                let bytes: &[u8] = &sk.strand_serialize()?;
                let ed = symm::encrypt(self.encryption_key, bytes, &aad)?;

                Ok(Channel::new(sk.pk_element().clone(), ed))
            }

            pub(crate) fn decrypt_share_sk(&self, c: &Channel<C>, cfg: &Configuration<C>) -> Result<PrivateKey<C>> {
                let identifier: String = self.get_pk()?.try_into()?;
                // 0 is a dummy batch value
                let aad = cfg.label(0, format!("encrypted by {}", identifier));
                let decrypted = symm::decrypt(&self.encryption_key, &c.encrypted_channel_sk, &aad)?;
                let ret = PrivateKey::<C>::strand_deserialize(&decrypted)?;

                Ok(ret)
            }
        }
        else {
            pub(crate) fn encrypt_share_sk(&self, sk: &PrivateKey<C>, _cfg: &Configuration<C>) -> Result<Channel<C>> {
                let bytes: &[u8] = &sk.strand_serialize()?;
                let ed = symm::encrypt(self.encryption_key, bytes)?;

                Ok(Channel::new(sk.pk_element().clone(), ed))
            }

            pub(crate) fn decrypt_share_sk(&self, c: &Channel<C>, _cfg: &Configuration<C>) -> Result<PrivateKey<C>> {
                let decrypted = symm::decrypt(&self.encryption_key, &c.encrypted_channel_sk)?;
                let ret = PrivateKey::<C>::strand_deserialize(&decrypted)?;

                Ok(ret)
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// ProtocolManager
///////////////////////////////////////////////////////////////////////////

pub struct ProtocolManager<C: Ctx> {
    pub signing_key: StrandSignatureSk,
    pub phantom: PhantomData<C>,
}

impl<C: Ctx> braid_messages::message::Signer for ProtocolManager<C> {
    fn get_signing_key(&self) -> &StrandSignatureSk {
        &self.signing_key
    }
}

///////////////////////////////////////////////////////////////////////////
// Signer (commonality to sign messages for Trustee and Protocolmanager)
///////////////////////////////////////////////////////////////////////////

/*pub(crate) trait Signer {
    fn get_signing_key(&self) -> &StrandSignatureSk;
    fn sign(&self, statement: Statement, artifact: Option<Vec<u8>>) -> Result<Message> {
        let sk = self.get_signing_key();
        let bytes = statement.strand_serialize()?;
        let signature: StrandSignature = sk.sign(&bytes)?;

        Ok(Message {
            signer_key: StrandSignaturePk::from(sk)?,
            signature,
            statement,
            artifact,
        })
    }
}*/

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl<C: Ctx> std::fmt::Debug for Trustee<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trustee{{ No info }}")
    }
}

impl<C: Ctx> std::fmt::Debug for ProtocolManager<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtcolManager{{ No info }}")
    }
}
