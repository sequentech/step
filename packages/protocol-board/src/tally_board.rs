// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A tally board: one weight batch of a tally session, run as a child of its
//! keys ceremony's DKG board.
//!
//! Everything the board's single `Ballots` message says is decided once, when
//! the ballots are extracted: the ciphertexts, the quorum of trustees that
//! mixes and decrypts them, and the tally identifier. The platform signs it as
//! the DKG board's protocol manager, the only signer the Configuration accepts
//! for it, and stores the signed message. Publishing posts those stored bytes,
//! so every retry puts the same `Ballots` on the board: the slot is single,
//! and a second, different one halts every trustee.

use anyhow::{bail, Context as _, Result};
use cryptography::utils::serialization::{Deserializable, Serializable};
use uuid::Uuid;
use wbraid::board::verify::verify;
use wbraid::messages::artifact::Ballots;
use wbraid::messages::newtypes::{
    ConfigurationHash, PublicKeyHash, TrusteeIndex,
};
use wbraid::messages::predicate::Predicate;
use wbraid::messages::wire::ProtocolMessage;

use crate::ballot::BallotCiphertext;
use crate::configuration::{DkgBoard, SignedConfiguration, CIPHERTEXT_WIDTH};
use crate::encoding::HashHex;
use crate::ids::{tally_id, BoardName};
use crate::{BoardManager, Ctx};

/// The trustees that mix and decrypt one tally, by protocol index, in mixing
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quorum(Vec<TrusteeIndex>);

impl Quorum {
    /// The quorum of the chosen trustees.
    ///
    /// `committee_names` is the ceremony's trustee list, in `Configuration`
    /// order, and a trustee's protocol index is its position there plus one.
    /// Anything but `threshold` distinct members of the committee is refused:
    /// braid's trustees halt on any other quorum, and a halt must never come
    /// from the platform's own input.
    pub fn of_names(
        dkg: &DkgBoard,
        committee_names: &[String],
        chosen: &[String],
    ) -> Result<Quorum> {
        let configuration = dkg.configuration()?;
        if committee_names.len() != configuration.trustees.len() {
            bail!(
                "the ceremony lists {} trustees, but board {} is configured \
                 for {}",
                committee_names.len(),
                dkg.name,
                configuration.trustees.len()
            );
        }
        if chosen.len() != configuration.threshold {
            bail!(
                "a quorum of {} trustees ({}) was chosen for board {}, whose \
                 threshold is {}",
                chosen.len(),
                chosen.join(", "),
                dkg.name,
                configuration.threshold
            );
        }

        let mut indices = Vec::with_capacity(chosen.len());
        for name in chosen {
            let position = committee_names
                .iter()
                .position(|member| member == name)
                .with_context(|| {
                    format!(
                        "trustee {name} is not in the committee of board {}",
                        dkg.name
                    )
                })?;
            let index = position + 1;
            if indices.contains(&index) {
                bail!("trustee {name} was chosen twice for board {}", dkg.name);
            }
            indices.push(index);
        }
        Ok(Quorum(indices))
    }
}

/// One tally board: its name and the `Ballots` message it serves, both decided
/// when the ballots are extracted.
#[derive(Debug, Clone)]
pub struct TallyBoard {
    pub name: BoardName,
    pub input: SignedBallots,
    _private: (),
}

impl TallyBoard {
    /// The board of one weight batch, with its signed `Ballots` message.
    ///
    /// `row_id` is the board's row, whose id becomes the message's tally
    /// identifier; `public_key_hash` is the one the keys ceremony recorded,
    /// which is what the trustees know the joint key by.
    #[expect(
        clippy::too_many_arguments,
        reason = "every argument is a separate fact the Ballots message binds"
    )]
    pub fn new(
        row_id: &Uuid,
        tally_session_id: &Uuid,
        batch: i64,
        dkg: &DkgBoard,
        public_key_hash: &HashHex,
        quorum: &Quorum,
        manager: &BoardManager,
        ballots: Vec<BallotCiphertext>,
    ) -> Result<TallyBoard> {
        let name = BoardName::for_tally(tally_session_id, batch);
        let ciphertexts = ballots
            .into_iter()
            .map(BallotCiphertext::into_ciphertext)
            .collect();
        let message = ProtocolMessage::ballots(
            manager,
            wbraid::native::timestamp(),
            ConfigurationHash(dkg.configuration.hash().to_hash()),
            PublicKeyHash(public_key_hash.to_hash()),
            quorum.0.clone(),
            tally_id(row_id),
            &Ballots::<Ctx, CIPHERTEXT_WIDTH>::new(ciphertexts),
        );
        let input = SignedBallots::from_message(message, &dkg.configuration)
            .with_context(|| format!("the Ballots message of board {name}"))?;
        Ok(TallyBoard {
            name,
            input,
            _private: (),
        })
    }
}

/// A tally board's `Ballots` message as the protocol manager signed it: what
/// the database stores and what the board serves.
#[derive(Debug, Clone)]
pub struct SignedBallots {
    message: ProtocolMessage<Ctx>,
    predicate: Predicate,
    quorum: Vec<TrusteeIndex>,
}

impl SignedBallots {
    /// A stored `Ballots` message, checked the way the board's readers check
    /// it: signed by the protocol manager the Configuration names, and scoped
    /// to that Configuration.
    pub fn parse(
        bytes: &[u8],
        configuration: &SignedConfiguration,
    ) -> Result<SignedBallots> {
        let message = ProtocolMessage::<Ctx>::deser(bytes)
            .context("a Ballots message")?;
        SignedBallots::from_message(message, configuration)
    }

    fn from_message(
        message: ProtocolMessage<Ctx>,
        configuration: &SignedConfiguration,
    ) -> Result<SignedBallots> {
        let (predicate, _body) =
            verify(&message, &configuration.message().verify_configuration()?)?;
        let Predicate::Ballots(ballots) = &predicate else {
            bail!(
                "a {:?} message is not a Ballots message",
                message.message_type
            );
        };
        let named = HashHex::of(&ballots.configuration.0);
        if named != *configuration.hash() {
            bail!(
                "the Ballots message names configuration {}, not {}",
                named.short(),
                configuration.hash().short()
            );
        }
        let quorum = ballots.trustees.clone();
        Ok(SignedBallots {
            message,
            predicate,
            quorum,
        })
    }

    /// The message's canonical bytes, which is what the database stores.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.message.ser()
    }

    pub(crate) fn message(&self) -> &ProtocolMessage<Ctx> {
        &self.message
    }

    /// The `Ballots` predicate the message stands for: the whole of it is
    /// what occupies the board's single `Ballots` slot.
    pub(crate) fn predicate(&self) -> &Predicate {
        &self.predicate
    }

    pub(crate) fn quorum(&self) -> &[TrusteeIndex] {
        &self.quorum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::committee::fixtures::{committee_of, trustees, TrusteeFixture};
    use crate::encoding::generate_manager;
    use wbraid::messages::newtypes::hash_bytes;
    use wbraid::trustee::Trustee;

    struct Fixture {
        fixtures: Vec<TrusteeFixture>,
        names: Vec<String>,
        manager: BoardManager,
        dkg: DkgBoard,
    }

    fn setup(count: usize, threshold: usize) -> Fixture {
        let fixtures = trustees(count);
        let manager = generate_manager();
        let dkg = DkgBoard::new(
            &Uuid::new_v4(),
            &manager,
            &committee_of(&fixtures),
            threshold,
        )
        .unwrap();
        let names = fixtures
            .iter()
            .map(|fixture| fixture.input.name.clone())
            .collect();
        Fixture {
            fixtures,
            names,
            manager,
            dkg,
        }
    }

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn public_key_hash() -> HashHex {
        HashHex::of(&hash_bytes(b"the joint key"))
    }

    fn tally_board(
        fixture: &Fixture,
        manager: &BoardManager,
    ) -> Result<TallyBoard> {
        let quorum = Quorum::of_names(
            &fixture.dkg,
            &fixture.names,
            &names(&["trustee3", "trustee1"]),
        )
        .unwrap();
        TallyBoard::new(
            &Uuid::new_v4(),
            &Uuid::new_v4(),
            7,
            &fixture.dkg,
            &public_key_hash(),
            &quorum,
            manager,
            Vec::new(),
        )
    }

    fn ballots_of(
        predicate: &Predicate,
    ) -> &wbraid::messages::predicate::Ballots {
        match predicate {
            Predicate::Ballots(ballots) => ballots,
            other => panic!("expected a Ballots predicate, got {other:?}"),
        }
    }

    #[test]
    fn a_quorum_numbers_trustees_by_their_place_in_the_committee() {
        let fixture = setup(3, 2);
        assert_eq!(
            Quorum::of_names(
                &fixture.dkg,
                &fixture.names,
                &names(&["trustee3", "trustee1"])
            )
            .unwrap(),
            Quorum(vec![3, 1])
        );
    }

    #[test]
    fn a_quorum_the_trustees_would_halt_on_is_refused() {
        let fixture = setup(3, 2);
        for chosen in [
            names(&["trustee1"]),
            names(&["trustee1", "trustee2", "trustee3"]),
            names(&[]),
            names(&["trustee1", "trustee4"]),
            names(&["trustee2", "trustee2"]),
        ] {
            assert!(
                Quorum::of_names(&fixture.dkg, &fixture.names, &chosen)
                    .is_err(),
                "{chosen:?}"
            );
        }

        // A trustee list that is not the one the board was configured with.
        let fewer = names(&["trustee1", "trustee2"]);
        assert!(Quorum::of_names(&fixture.dkg, &fewer, &fewer).is_err());
    }

    #[test]
    fn a_stored_ballots_message_reads_back_as_the_one_that_was_signed() {
        let fixture = setup(3, 2);
        let board = tally_board(&fixture, &fixture.manager).unwrap();
        let stored = SignedBallots::parse(
            &board.input.to_bytes(),
            &fixture.dkg.configuration,
        )
        .unwrap();

        assert_eq!(stored.predicate(), board.input.predicate());
        assert_eq!(stored.to_bytes(), board.input.to_bytes());
        assert_eq!(stored.quorum(), &[3, 1]);

        // The head binds the recorded joint key and the parent's configuration.
        let ballots = ballots_of(stored.predicate());
        assert_eq!(HashHex::of(&ballots.public_key.0), public_key_hash());
        assert_eq!(
            HashHex::of(&ballots.configuration.0),
            *fixture.dkg.configuration.hash()
        );
    }

    #[test]
    fn every_board_gets_its_own_tally_id() {
        let fixture = setup(3, 2);
        let first = tally_board(&fixture, &fixture.manager).unwrap();
        let second = tally_board(&fixture, &fixture.manager).unwrap();
        assert_ne!(
            ballots_of(first.input.predicate()).tally_id,
            ballots_of(second.input.predicate()).tally_id
        );
    }

    #[test]
    fn a_ballots_message_the_configuration_does_not_accept_is_refused() {
        let fixture = setup(3, 2);

        // Signed by a manager the Configuration does not name.
        assert!(tally_board(&fixture, &generate_manager()).is_err());

        let board = tally_board(&fixture, &fixture.manager).unwrap();
        let configuration = &fixture.dkg.configuration;
        assert!(SignedBallots::parse(b"not a message", configuration).is_err());
        assert!(
            SignedBallots::parse(&configuration.to_bytes(), configuration)
                .is_err()
        );

        // The same bytes against another ceremony's Configuration.
        let other = setup(3, 2);
        assert!(SignedBallots::parse(
            &board.input.to_bytes(),
            &other.dkg.configuration
        )
        .is_err());

        let cfg_hash = ConfigurationHash(configuration.hash().to_hash());
        let body = Ballots::<Ctx, CIPHERTEXT_WIDTH>::new(Vec::new());
        let signed_by_a_trustee = ProtocolMessage::<Ctx>::ballots(
            &Trustee::<Ctx>::new(
                fixture.fixtures[0].input.name.clone(),
                fixture.fixtures[0].signing_key.clone(),
                fixture.fixtures[0].share_encryption.clone(),
                &fixture.dkg.configuration().unwrap(),
            )
            .unwrap(),
            0,
            cfg_hash,
            PublicKeyHash(public_key_hash().to_hash()),
            vec![3, 1],
            1,
            &body,
        );
        assert!(SignedBallots::parse(
            &signed_by_a_trustee.ser(),
            configuration
        )
        .is_err());

        // Signed by this board's manager, for another configuration.
        let elsewhere = ProtocolMessage::<Ctx>::ballots(
            &fixture.manager,
            0,
            ConfigurationHash(hash_bytes(b"another ceremony")),
            PublicKeyHash(public_key_hash().to_hash()),
            vec![3, 1],
            1,
            &body,
        );
        assert!(SignedBallots::parse(&elsewhere.ser(), configuration).is_err());
    }
}
