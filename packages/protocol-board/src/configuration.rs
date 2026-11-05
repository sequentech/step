// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The `Configuration` a committee produces, and the board it will live on.
//!
//! braid's `Configuration` is the domain of a protocol run: every message ever
//! posted names the hash of one. The platform builds it once, when the
//! ceremony is created, signs it as the board's protocol manager and stores
//! the signed message. Publishing posts those stored bytes, so every retry
//! puts the same Configuration on the board and nothing is recomputed from
//! trustee rows that may have changed since.

use std::marker::PhantomData;

use anyhow::{ensure, Context as _, Result};
use cryptography::utils::serialization::{Deserializable, Serializable};
use uuid::Uuid;
use wbraid::messages::artifact::Configuration;
use wbraid::messages::newtypes::ConfigurationHash;
use wbraid::messages::wire::{MessageType, ProtocolMessage};

use crate::committee::Committee;
use crate::encoding::{manager_verifying_key, HashHex};
use crate::ids::{configuration_id, BoardName};
use crate::{BoardManager, Ctx};

/// The number of group elements one ballot ciphertext carries. The ballot
/// codec packs a ballot into a single element, so every board the platform
/// creates is one element wide.
pub(crate) const CIPHERTEXT_WIDTH: usize = 1;

/// A keys ceremony's DKG board: its name and the Configuration it serves,
/// both decided when the ceremony is created.
#[derive(Debug, Clone)]
pub struct DkgBoard {
    pub name: BoardName,
    pub configuration: SignedConfiguration,
    _private: (),
}

impl DkgBoard {
    /// Attempt building a new board for the dky, a.k.a. key ceremony.
    /// Errors on any protocol rule violations.
    pub fn new(
        keys_ceremony_id: &Uuid,
        manager: &BoardManager,
        committee: &Committee,
        threshold: usize,
    ) -> Result<DkgBoard> {
        let configuration = Configuration::<Ctx>::new(
            configuration_id(keys_ceremony_id),
            manager_verifying_key(manager),
            committee.signing_keys(),
            threshold,
            CIPHERTEXT_WIDTH,
            committee.share_encryption_keys(),
            PhantomData,
        )?;
        let message = ProtocolMessage::configuration(
            manager,
            crate::timestamp(),
            &configuration,
        );
        Ok(DkgBoard {
            name: BoardName::for_dkg(keys_ceremony_id),
            configuration: SignedConfiguration::from_message(message)?,
            _private: (),
        })
    }

    pub fn from_signed_configuration(
        keys_ceremony_id: &Uuid,
        configuration: SignedConfiguration,
    ) -> DkgBoard {
        DkgBoard {
            name: BoardName::for_dkg(keys_ceremony_id),
            configuration,
            _private: (),
        }
    }

    /// How many trustees a tally of this board's key needs: the size of every
    /// tally quorum.
    pub fn threshold(&self) -> usize {
        self.configuration.threshold
    }

    /// The Configuration the board serves, as its readers accept it.
    pub(crate) fn configuration(&self) -> Result<Configuration<Ctx>> {
        self.configuration.message.verify_configuration()
    }
}

/// A `Configuration` message as the protocol manager signed it: what the
/// database stores and what the board serves.
#[derive(Debug, Clone)]
pub struct SignedConfiguration {
    message: ProtocolMessage<Ctx>,
    hash: HashHex,
    threshold: usize,
}

impl SignedConfiguration {
    /// Check a restored private identity before using it to manage this board.
    pub fn verify_manager_key(&self, manager: &BoardManager) -> Result<()> {
        ensure!(
            manager_verifying_key(manager)
                == self.message.verify_configuration()?.protocol_manager,
            "protocol manager key does not match the archived configuration"
        );
        Ok(())
    }

    /// Validate an archived message against this immutable configuration.
    pub fn verify_message(&self, bytes: &[u8]) -> Result<()> {
        let message = ProtocolMessage::<Ctx>::deser(bytes)?;
        // wbraid accepts Configuration at construction, outside predicate
        // verification. The archive anchor must name this immutable domain.
        if message.message_type == MessageType::Configuration {
            ensure!(
                Self::from_message(message)?.hash == self.hash,
                "archive configuration mismatch"
            );
            return Ok(());
        }
        let (predicate, _) = wbraid::board::verify::verify(
            &message,
            &self.message.verify_configuration()?,
        )?;
        // Signature verification reconstructs the predicate; the protocol
        // engine normally checks its domain. Archive admission must do so too.
        ensure!(
            HashHex::of(&predicate.get_configuration().0) == self.hash,
            "archive message configuration mismatch"
        );
        Ok(())
    }
    /// The immutable pair registered at this committee position.
    pub fn trustee_public_keys(
        &self,
        position: usize,
    ) -> Result<crate::TrusteePublicKeys> {
        let cfg = self.message.verify_configuration()?;
        let signing = cfg
            .trustees
            .get(position)
            .context("trustee position outside committee")?;
        let share = cfg
            .share_encryption_keys
            .get(position)
            .context("missing share encryption key")?;
        Ok(crate::TrusteePublicKeys {
            signing_public_key: crate::encoding::encode_signing_public_key(
                signing,
            )?,
            share_encryption_public_key:
                crate::encoding::encode_share_encryption_public_key(share),
        })
    }
    /// A stored Configuration message, checked the way the board's readers
    /// check it: the manager named inside must have signed it.
    pub fn parse(bytes: &[u8]) -> Result<SignedConfiguration> {
        let message = ProtocolMessage::<Ctx>::deser(bytes)
            .context("a Configuration message")?;
        SignedConfiguration::from_message(message)
    }

    fn from_message(
        message: ProtocolMessage<Ctx>,
    ) -> Result<SignedConfiguration> {
        let configuration = message.verify_configuration()?;
        let hash = HashHex::of(
            &ConfigurationHash::from_configuration(&configuration)?.0,
        );
        Ok(SignedConfiguration {
            message,
            hash,
            threshold: configuration.threshold,
        })
    }

    /// Immutable cryptographic domain; importing may remap application UUIDs.
    pub fn configuration_id(&self) -> Result<u128> {
        Ok(self.message.verify_configuration()?.id)
    }

    /// The message's canonical bytes, which is what the database stores.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.message.ser()
    }

    /// The hash every message on the board names.
    pub fn hash(&self) -> &HashHex {
        &self.hash
    }

    pub(crate) fn message(&self) -> &ProtocolMessage<Ctx> {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::committee::fixtures::{committee_of, inputs, trustees};
    use crate::encoding::generate_manager;
    use wbraid::messages::newtypes::MAX_TRUSTEES;

    fn dkg_board(trustee_count: usize, threshold: usize) -> Result<DkgBoard> {
        DkgBoard::new(
            &Uuid::new_v4(),
            &generate_manager(),
            &committee_of(&trustees(trustee_count)),
            threshold,
        )
    }

    fn configuration(signed: &SignedConfiguration) -> Configuration<Ctx> {
        signed.message().verify_configuration().unwrap()
    }

    #[test]
    fn a_committee_the_protocol_cannot_run_is_an_error_not_a_panic() {
        for (trustee_count, threshold) in
            [(1, 1), (1, 2), (3, 1), (3, 4), (MAX_TRUSTEES + 1, 2)]
        {
            assert!(
                dkg_board(trustee_count, threshold).is_err(),
                "{trustee_count} trustees, threshold {threshold}"
            );
        }

        let mut rows = inputs(&trustees(2));
        rows[1].signing_public_key = rows[0].signing_public_key.clone();
        let twice = Committee::new(&rows).unwrap();
        assert!(
            DkgBoard::new(&Uuid::new_v4(), &generate_manager(), &twice, 2)
                .is_err()
        );

        assert!(dkg_board(2, 2).is_ok());
        assert!(dkg_board(MAX_TRUSTEES, MAX_TRUSTEES).is_ok());
    }

    #[test]
    fn a_board_reads_its_threshold_from_the_configuration_it_serves() {
        for (trustee_count, threshold) in [(3, 2), (3, 3), (5, 4)] {
            let board = dkg_board(trustee_count, threshold).unwrap();
            let stored = DkgBoard::from_signed_configuration(
                &Uuid::new_v4(),
                SignedConfiguration::parse(&board.configuration.to_bytes())
                    .unwrap(),
            );
            assert_eq!(stored.threshold(), threshold);
        }
    }

    #[test]
    fn restored_manager_keys_must_match_the_signed_configuration() {
        let manager = generate_manager();
        let board = DkgBoard::new(
            &Uuid::new_v4(),
            &manager,
            &committee_of(&trustees(2)),
            2,
        )
        .unwrap();
        let stored =
            SignedConfiguration::parse(&board.configuration.to_bytes())
                .unwrap();
        let restored = crate::parse_manager_key(
            &crate::encode_manager_key(&manager).unwrap(),
        )
        .unwrap();
        stored.verify_manager_key(&restored).unwrap();
        let error = stored.verify_manager_key(&generate_manager()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "protocol manager key does not match the archived configuration"
        );
    }

    #[test]
    fn a_stored_configuration_reads_back_as_the_one_that_was_signed() {
        let board = dkg_board(3, 2).unwrap();
        let stored =
            SignedConfiguration::parse(&board.configuration.to_bytes())
                .unwrap();

        assert_eq!(stored.hash(), board.configuration.hash());
        assert_eq!(stored.to_bytes(), board.configuration.to_bytes());
        assert_eq!(
            stored.hash(),
            &HashHex::of(
                &ConfigurationHash::from_configuration(&configuration(&stored))
                    .unwrap()
                    .0
            )
        );
    }

    #[test]
    fn archive_accepts_its_signed_configuration_and_rejects_another_anchor() {
        let manager = generate_manager();
        let committee = committee_of(&trustees(2));
        let board =
            DkgBoard::new(&Uuid::new_v4(), &manager, &committee, 2).unwrap();
        board
            .configuration
            .verify_message(&board.configuration.to_bytes())
            .unwrap();
        let other =
            DkgBoard::new(&Uuid::new_v4(), &manager, &committee, 2).unwrap();
        let error = board
            .configuration
            .verify_message(&other.configuration.to_bytes())
            .unwrap_err();
        assert!(error.to_string().contains("archive configuration mismatch"));
        let forged = ProtocolMessage::<Ctx>::configuration(
            &generate_manager(),
            0,
            &configuration(&board.configuration),
        );
        assert!(board.configuration.verify_message(&forged.ser()).is_err());
    }

    #[test]
    fn archive_rejects_a_valid_trustee_signature_from_another_configuration() {
        let manager = generate_manager();
        let members = trustees(2);
        let committee = committee_of(&members);
        let board =
            DkgBoard::new(&Uuid::new_v4(), &manager, &committee, 2).unwrap();
        let other =
            DkgBoard::new(&Uuid::new_v4(), &manager, &committee, 2).unwrap();
        let trustee = wbraid::trustee::Trustee::<Ctx>::new(
            members[0].input.name.clone(),
            members[0].signing_key.clone(),
            members[0].share_encryption.clone(),
            &configuration(&board.configuration),
        )
        .unwrap();
        let message = |signed: &SignedConfiguration| {
            ProtocolMessage::<Ctx>::shares(
                &trustee,
                0,
                ConfigurationHash::from_configuration(&configuration(signed))
                    .unwrap(),
                &vec![1u8, 2, 3],
            )
            .ser()
        };
        board
            .configuration
            .verify_message(&message(&board.configuration))
            .unwrap();
        let error = board
            .configuration
            .verify_message(&message(&other.configuration))
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("archive message configuration mismatch"));
    }

    #[test]
    fn a_stored_configuration_that_is_not_one_is_refused() {
        assert!(SignedConfiguration::parse(b"not a message").is_err());

        // A well-formed message that is not a Configuration.
        let manager = generate_manager();
        let board = dkg_board(2, 2).unwrap();
        let shares = ProtocolMessage::<Ctx>::shares(
            &manager,
            0,
            ConfigurationHash::from_configuration(&configuration(
                &board.configuration,
            ))
            .unwrap(),
            &vec![1u8, 2, 3],
        );
        assert!(SignedConfiguration::parse(&shares.ser()).is_err());
    }

    #[test]
    fn a_configuration_signed_by_someone_else_is_refused() {
        let board = dkg_board(2, 2).unwrap();
        let forged = ProtocolMessage::<Ctx>::configuration(
            &generate_manager(),
            0,
            &configuration(&board.configuration),
        );
        assert!(SignedConfiguration::parse(&forged.ser()).is_err());
    }
}
