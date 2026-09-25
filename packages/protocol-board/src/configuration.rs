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

use anyhow::{Context as _, Result};
use cryptography::utils::serialization::{Deserializable, Serializable};
use uuid::Uuid;
use wbraid::messages::artifact::Configuration;
use wbraid::messages::newtypes::ConfigurationHash;
use wbraid::messages::wire::ProtocolMessage;

use crate::committee::Committee;
use crate::encoding::{manager_verifying_key, HashHex};
use crate::ids::{configuration_id, BoardName};
use crate::{BoardManager, Ctx};

/// The number of group elements one ballot ciphertext carries. The ballot
/// codec packs a ballot into a single element, so every board the platform
/// creates is one element wide.
const CIPHERTEXT_WIDTH: usize = 1;

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
            wbraid::native::timestamp(),
            &configuration,
        );
        Ok(DkgBoard {
            name: BoardName::for_dkg(keys_ceremony_id),
            configuration: SignedConfiguration::from_message(message)?,
            _private: (),
        })
    }
}

/// A `Configuration` message as the protocol manager signed it: what the
/// database stores and what the board serves.
#[derive(Debug, Clone)]
pub struct SignedConfiguration {
    message: ProtocolMessage<Ctx>,
    hash: HashHex,
}

impl SignedConfiguration {
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
        Ok(SignedConfiguration { message, hash })
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
