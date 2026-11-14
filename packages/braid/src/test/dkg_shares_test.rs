// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::artifact::{Configuration, DkgPublicKey};
use b3::messages::message::Message;
use b3::messages::newtypes::*;
use b3::messages::statement::StatementType;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::{Ctx, Element};
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;

use crate::protocol::action::Action;
use crate::protocol::datalog::{dkg, hashes_init, NULL_HASH};
use crate::protocol::predicate::Predicate;
use crate::protocol::trustee2::{StepResult, Trustee};
use crate::test::protocol_test_memory::create_protocol_test;
use crate::test::vector_board::VectorBoard;
use crate::util::ProtocolError;

type C = RistrettoCtx;
type E = <C as Ctx>::E;

const NUM_TRUSTEES: usize = 3;
const THRESHOLD: usize = 2;
const CFG_SEED: u8 = 1;
const PK_SEED: u8 = 2;
const EMPTY_SEED: u8 = 0;
const UNKNOWN_SEED: u8 = 99;
const CHANNELS_SEEDS: [u8; NUM_TRUSTEES] = [10, 11, 12];
const SHARES_SEEDS: [u8; NUM_TRUSTEES] = [20, 21, 22];

///////////////////////////////////////////////////////////////////////////
// Inference rules
///////////////////////////////////////////////////////////////////////////

/// Builds a hashes array from seeds, EMPTY_SEED standing for NULL_HASH.
fn hashes(seeds: &[u8]) -> THashes {
    let mut ret = [NULL_HASH; MAX_TRUSTEES];
    for (i, seed) in seeds.iter().enumerate() {
        ret[i] = [*seed; 64];
    }

    ret
}

/// Whether the trustee at self_position must sign the public key statement
/// from trustee 0 that lists the given shares and channels, when all trustees
/// have signed the channels of CHANNELS_SEEDS and posted the shares of SHARES_SEEDS.
fn must_sign_public_key(
    self_position: TrusteePosition,
    shares_seeds: &[u8],
    channels_seeds: &[u8],
) -> bool {
    let cfg_h = ConfigurationHash([CFG_SEED; 64]);
    let pk_h = PublicKeyHash([PK_SEED; 64]);
    let shares_hs = SharesHashes(hashes(shares_seeds));
    let channels_hs = ChannelsHashes(hashes(channels_seeds));
    let board_channels = hashes(&CHANNELS_SEEDS);
    let board_shares = hashes(&SHARES_SEEDS);

    let mut predicates = vec![Predicate::ConfigurationSignedAll(
        cfg_h,
        self_position,
        NUM_TRUSTEES,
        THRESHOLD,
    )];
    for i in 0..NUM_TRUSTEES {
        predicates.push(Predicate::Channel(cfg_h, ChannelHash(board_channels[i]), i));
        predicates.push(Predicate::ChannelsSigned(
            cfg_h,
            ChannelsHashes(board_channels),
            i,
        ));
        predicates.push(Predicate::Shares(cfg_h, SharesHash(board_shares[i]), i));
    }
    predicates.push(Predicate::PublicKey(cfg_h, pk_h, shares_hs, channels_hs, 0));

    let (_, actions, errors) = dkg::D.run(&predicates);
    assert!(errors.is_empty());

    actions.contains(&Action::SignPublicKey(
        cfg_h,
        pk_h,
        shares_hs,
        channels_hs,
        self_position,
        NUM_TRUSTEES,
        THRESHOLD,
    ))
}

#[test]
fn test_sign_public_key_requires_shares_and_channels_on_the_board() {
    let [first, second, third] = SHARES_SEEDS;

    for position in 1..NUM_TRUSTEES {
        assert!(must_sign_public_key(
            position,
            &SHARES_SEEDS,
            &CHANNELS_SEEDS
        ));

        assert!(!must_sign_public_key(position, &[first], &CHANNELS_SEEDS));
        assert!(!must_sign_public_key(position, &[], &CHANNELS_SEEDS));
        assert!(!must_sign_public_key(
            position,
            &[first, EMPTY_SEED, third],
            &CHANNELS_SEEDS
        ));
        assert!(!must_sign_public_key(
            position,
            &[first, second, UNKNOWN_SEED],
            &CHANNELS_SEEDS
        ));
        assert!(!must_sign_public_key(
            position,
            &SHARES_SEEDS,
            &[CHANNELS_SEEDS[0], CHANNELS_SEEDS[1], UNKNOWN_SEED]
        ));
    }
}

///////////////////////////////////////////////////////////////////////////
// Trustees and verifier
///////////////////////////////////////////////////////////////////////////

/// A key ceremony in which all trustees have posted their shares.
struct Ceremony {
    cfg: Configuration<C>,
    trustees: Vec<Trustee<C>>,
    board: VectorBoard,
    seen: Vec<i64>,
}

impl Ceremony {
    fn new() -> Ceremony {
        let test = create_protocol_test(NUM_TRUSTEES, &[1, 2], RistrettoCtx).unwrap();
        let mut ceremony = Ceremony {
            cfg: test.cfg,
            trustees: test.trustees,
            board: test.remote,
            seen: vec![-1; NUM_TRUSTEES],
        };

        'cycles: for _ in 0..10 {
            for i in 0..NUM_TRUSTEES {
                let result = ceremony.step(i);
                ceremony.post(&result.messages);
                if ceremony.count(StatementType::Shares) == NUM_TRUSTEES {
                    break 'cycles;
                }
            }
        }
        assert_eq!(ceremony.count(StatementType::Shares), NUM_TRUSTEES);
        assert_eq!(ceremony.count(StatementType::PublicKey), 0);

        ceremony
    }

    fn count(&self, kind: StatementType) -> usize {
        self.board
            .messages
            .iter()
            .filter(|m| {
                Message::strand_deserialize(&m.message)
                    .unwrap()
                    .statement
                    .get_kind()
                    == kind
            })
            .count()
    }

    /// Runs a step of the trustee, without posting the resulting messages.
    fn step(&mut self, trustee: usize) -> StepResult {
        let messages = self.board.get(self.seen[trustee]);
        self.seen[trustee] += messages.len() as i64;

        self.trustees[trustee].step(&messages).unwrap()
    }

    fn post(&mut self, messages: &[Message]) {
        for message in messages {
            self.board.add(message.try_clone().unwrap());
        }
    }

    /// The public key message that trustee 0 computes from the shares on the board.
    fn honest_public_key(&mut self) -> Message {
        self.step(0).messages.remove(0)
    }

    /// A public key message from the signer that lists the given shares hashes,
    /// with the key computed from the listed shares only. Trustee 0 must have
    /// seen all the shares.
    fn public_key_for(
        &self,
        honest: &Message,
        shares_hs: SharesHashes,
        signer: usize,
        artifact: bool,
    ) -> Message {
        let (_, _, _, channels_hs) = public_key_data(&self.cfg, honest);
        let ctx = C::default();
        let mut pk = E::mul_identity();
        let mut verification_keys = vec![E::mul_identity(); NUM_TRUSTEES];

        for (i, h) in shares_hs
            .0
            .iter()
            .enumerate()
            .filter(|(_, h)| **h != NULL_HASH)
        {
            let shares = self.trustees[0].get_shares(&SharesHash(*h), i).unwrap();
            pk = pk.mul(&shares.commitments[0]).modp(&ctx);
            for (j, vk) in verification_keys.iter_mut().enumerate() {
                let factor = strand::threshold::verification_key_factor(
                    &shares.commitments,
                    THRESHOLD,
                    j,
                    &ctx,
                );
                *vk = vk.mul(&factor).modp(&ctx);
            }
        }

        let public_key = DkgPublicKey::new(pk, verification_keys);
        Message::public_key_msg(
            &self.cfg,
            &public_key,
            &shares_hs,
            &channels_hs,
            artifact,
            &self.trustees[signer],
        )
        .unwrap()
    }

    /// Whether the braid verifier accepts the public key on the board.
    ///
    /// Follows Verifier::run, which reads the messages from a grpc board.
    fn verifier_accepts_public_key(&self) -> bool {
        let messages = || -> Vec<(Message, i64)> {
            self.board
                .messages
                .iter()
                .map(|m| (Message::strand_deserialize(&m.message).unwrap(), m.id))
                .collect()
        };

        // Skip the configuration message
        let mut predicates: Vec<Predicate> = messages()[1..]
            .iter()
            .map(|(m, _)| {
                let verified = m.verify(&self.cfg).unwrap();
                Predicate::from_statement::<C>(
                    &verified.statement,
                    verified.signer_position,
                    &self.cfg,
                )
                .unwrap()
            })
            .collect();
        predicates.push(Predicate::get_verifier_bootstrap_predicate(&self.cfg).unwrap());

        let mut verifier = Trustee::<C>::new(
            "Verifier".to_string(),
            "".to_string(),
            StrandSignatureSk::gen().unwrap(),
            strand::symm::gen_key(),
            None,
            None,
        );
        for message in verifier.verify(messages()).unwrap() {
            predicates.push(
                Predicate::from_statement::<C>(&message.statement, VERIFIER_INDEX, &self.cfg)
                    .unwrap(),
            );
        }

        let (root, _, _) = crate::verify::datalog::S.run(&predicates);

        !root.is_empty()
    }
}

fn public_key_data(
    cfg: &Configuration<C>,
    message: &Message,
) -> (
    ConfigurationHash,
    PublicKeyHash,
    SharesHashes,
    ChannelsHashes,
) {
    match Predicate::from_statement::<C>(&message.statement, 0, cfg).unwrap() {
        Predicate::PublicKey(cfg_h, pk_h, shares_hs, channels_hs, _) => {
            (cfg_h, pk_h, shares_hs, channels_hs)
        }
        _ => panic!("Not a public key statement"),
    }
}

/// Shares hashes of trustee 0 only, and no shares hashes at all.
fn omitted_shares(shares_hs: &SharesHashes) -> [SharesHashes; 2] {
    [
        SharesHashes(hashes_init(shares_hs.0[0])),
        SharesHashes([NULL_HASH; MAX_TRUSTEES]),
    ]
}

/// All the ways in which shares hashes can fail to list the shares of exactly
/// the trustees.
fn incomplete_shares(shares_hs: &SharesHashes) -> Vec<SharesHashes> {
    let mut missing_one = shares_hs.0;
    missing_one[1] = NULL_HASH;
    let mut one_too_many = shares_hs.0;
    one_too_many[NUM_TRUSTEES] = [UNKNOWN_SEED; 64];

    let mut ret = omitted_shares(shares_hs).to_vec();
    ret.extend([SharesHashes(missing_one), SharesHashes(one_too_many)]);

    ret
}

fn is_verification_error(error: &ProtocolError) -> bool {
    match error {
        ProtocolError::VerificationError(_) => true,
        ProtocolError::WrappedError(_, inner) => is_verification_error(inner),
        _ => false,
    }
}

#[test]
fn test_public_key_with_incomplete_shares_is_not_signed() {
    for index in 0..2 {
        let mut ceremony = Ceremony::new();
        let honest = ceremony.honest_public_key();
        let (_, _, shares_hs, _) = public_key_data(&ceremony.cfg, &honest);
        let incomplete = omitted_shares(&shares_hs)[index];

        let message = ceremony.public_key_for(&honest, incomplete, 0, true);
        ceremony.post(&[message]);

        for trustee in 1..NUM_TRUSTEES {
            let result = ceremony.step(trustee);
            assert!(!result
                .actions
                .iter()
                .any(|a| matches!(a, Action::SignPublicKey(..))));
            assert!(result.messages.is_empty());
        }
    }
}

#[test]
fn test_public_key_with_all_shares_is_signed() {
    let mut ceremony = Ceremony::new();
    let honest = ceremony.honest_public_key();
    ceremony.post(&[honest]);

    for trustee in 1..NUM_TRUSTEES {
        let result = ceremony.step(trustee);
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, Action::SignPublicKey(..))));
        assert_eq!(result.messages.len(), 1);
        assert_eq!(
            result.messages[0].statement.get_kind(),
            StatementType::PublicKeySigned
        );
    }
}

#[test]
fn test_compute_public_key_rejects_incomplete_shares_hashes() {
    let mut ceremony = Ceremony::new();
    let honest = ceremony.honest_public_key();
    let (cfg_h, pk_h, shares_hs, channels_hs) = public_key_data(&ceremony.cfg, &honest);

    let message = ceremony.public_key_for(&honest, omitted_shares(&shares_hs)[0], 0, true);
    ceremony.post(&[message]);
    ceremony.step(1);

    let compute = |shares_hs| {
        Action::ComputePublicKey(cfg_h, shares_hs, channels_hs, 0, NUM_TRUSTEES, THRESHOLD)
            .run(&ceremony.trustees[0])
    };
    let sign = |shares_hs| {
        Action::SignPublicKey(
            cfg_h,
            pk_h,
            shares_hs,
            channels_hs,
            1,
            NUM_TRUSTEES,
            THRESHOLD,
        )
        .run(&ceremony.trustees[1])
    };

    for incomplete in incomplete_shares(&shares_hs) {
        assert!(is_verification_error(&compute(incomplete).unwrap_err()));
        assert!(is_verification_error(&sign(incomplete).unwrap_err()));
    }

    assert_eq!(compute(shares_hs).unwrap().len(), 1);
}

#[test]
fn test_compute_decryption_factors_rejects_incomplete_shares_hashes() {
    let mut ceremony = Ceremony::new();
    let honest = ceremony.honest_public_key();
    let (cfg_h, pk_h, shares_hs, channels_hs) = public_key_data(&ceremony.cfg, &honest);
    ceremony.post(&[honest]);
    ceremony.step(1);

    let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected[..THRESHOLD].copy_from_slice(&[1, 2]);
    let decrypt = |shares_hs| {
        Action::ComputeDecryptionFactors(
            cfg_h,
            1,
            channels_hs,
            CiphertextsHash([UNKNOWN_SEED; 64]),
            0,
            pk_h,
            shares_hs,
            1,
            NUM_TRUSTEES,
            THRESHOLD,
            selected,
        )
        .run(&ceremony.trustees[1])
    };

    for incomplete in incomplete_shares(&shares_hs) {
        assert!(is_verification_error(&decrypt(incomplete).unwrap_err()));
    }

    // There is no mix on the board, so decryption fails after the shares check
    assert!(!is_verification_error(&decrypt(shares_hs).unwrap_err()));
}

#[test]
fn test_verifier_rejects_public_key_with_incomplete_shares() {
    let mut ceremony = Ceremony::new();
    let honest = ceremony.honest_public_key();
    let (_, _, shares_hs, _) = public_key_data(&ceremony.cfg, &honest);
    let incomplete = omitted_shares(&shares_hs)[0];

    let message = ceremony.public_key_for(&honest, incomplete, 0, true);
    ceremony.post(&[message]);
    for trustee in 1..NUM_TRUSTEES {
        let signature = ceremony.public_key_for(&honest, incomplete, trustee, false);
        ceremony.post(&[signature]);
    }

    assert!(!ceremony.verifier_accepts_public_key());
}

#[test]
fn test_verifier_accepts_public_key_with_all_shares() {
    let mut ceremony = Ceremony::new();
    let honest = ceremony.honest_public_key();
    ceremony.post(&[honest]);
    for trustee in 1..NUM_TRUSTEES {
        let result = ceremony.step(trustee);
        ceremony.post(&result.messages);
    }

    assert!(ceremony.verifier_accepts_public_key());
}
