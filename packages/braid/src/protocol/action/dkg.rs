#![allow(clippy::too_many_arguments)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::protocol::datalog;
use anyhow::Result;
use b4::messages::artifact::{Channel, Configuration, DkgCommitmentProofPolicy};
use strand::elgamal::PublicKey;
use strand::zkp::Zkp;

/// Generates a private communication channel for this trustee.
///
/// Used to send shares privately to this trustee. A trustee will
/// receive a share from each of its peers. These shares are elgamal
/// encrypted with the Channel's public key. The corresponding
/// private key is symmetrically encrypted with an private symmetric
/// key belonging to the trustee, and is also part of the Channel data.
/// This allows restoring all information from the bulletin board, as well
/// as securely downloading a Channel by its trustee during the key
/// ceremony.
///
/// Channels include a schnorr proof for knowledge of the secret
/// key corresponding to the public key.
///
/// Returns a Message of type Channel signed by this trustee.
pub(super) fn gen_channel<C: Ctx, S: crate::protocol::board::LocalBoardStorage>(
    configuration_h: &ConfigurationHash,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message>, ProtocolError> {
    let ctx: C = Default::default();

    let cfg = trustee.get_configuration(configuration_h)?;

    // Generate a keypair for share transport
    let sk = strand::elgamal::PrivateKey::gen(&ctx);
    let label = cfg.label(0, format!("channel pk proof"));
    let (pk, proof) = sk.get_pk_and_proof(&label)?;

    let ed = trustee.encrypt_share_sk(&sk, &cfg)?;
    let channel = Channel::new(pk.element().clone(), proof, ed);

    let m = Message::channel_msg(cfg, &channel, true, trustee)?;
    Ok(vec![m])
}

/// Verifies all the posted Channels.
///
/// Channel verification checks schnorr proofs for the
/// public keys. Additionally, each trustee self verifies
/// their own Channel's private key by decrypting it.
///
/// Returns a Message of type ChannelsAllSigned signed by this trustee.
pub(super) fn sign_channels<C: Ctx, S: crate::protocol::board::LocalBoardStorage>(
    configuration_h: &ConfigurationHash,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_trustees: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message>, ProtocolError> {
    let ctx: C = Default::default();
    let cfg = trustee.get_configuration(configuration_h)?;
    let zkp = Zkp::new(&ctx);
    let label = cfg.label(0, format!("channel pk proof"));

    assert_eq!(
        datalog::hashes_count(&channels_hs.0),
        *num_trustees,
        "Unexpected number of channels"
    );

    for (i, h) in channels_hs
        .0
        .iter()
        .filter(|h| **h != NULL_HASH)
        .enumerate()
    {
        let hash = *h;
        let channel = trustee.get_channel(&ChannelHash(hash), i)?;
        let pk_element = channel.channel_pk.clone();
        let ok = zkp.schnorr_verify(&pk_element, None, &channel.pk_proof, &label);
        if !ok {
            return Err(ProtocolError::VerificationError(format!(
                "Failed to verify schnorr proof on channel"
            )));
        }

        // Check that our own Channel is at the correct posistion and decrypts correctly
        if i == *self_pos {
            let sk = trustee.decrypt_share_sk(&channel, cfg)?;
            if *sk.pk_element() != pk_element {
                return Err(ProtocolError::VerificationError(format!(
                    "Failed to decrypt self channel"
                )));
            }
        }
    }

    let m = Message::channels_all_signed_msg(cfg, channels_hs, trustee)?;
    Ok(vec![m])
}

/// Computes the shares for all trustees.
///
/// Each trustee computes a share and commitments for all trustees
/// including itself. These shares are encrypted with the recipient's public
/// key as present in their Channel. Shares are verified by their
/// recipient trustees as part of their public key verification.
///
/// Under DkgCommitmentProofPolicy::SchnorrPok the Shares also include
/// a proof of knowledge of the polynomial's constant term.
///
/// Returns a Message of type Shares signed by this trustee.
///
/// As described in Cortier et al.; based on Pedersen.
pub(super) fn compute_shares<C: Ctx, S: crate::protocol::board::LocalBoardStorage>(
    configuration_h: &ConfigurationHash,
    channels_hs: &ChannelsHashes,
    num_trustees: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message>, ProtocolError> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(configuration_h)?;

    let (coeffs, commitments) = strand::threshold::gen_coefficients(*threshold, &ctx);

    let mut s = vec![];

    for i in 0..*num_trustees {
        let share = strand::threshold::eval_poly(i + 1, *threshold, &coeffs, &ctx);

        // Obtain the public key for the recipient of the share
        let target_channel_h = channels_hs.0.get(i).ok_or(ProtocolError::InternalError(
            "Could not retrieve channel hash".to_string(),
        ))?;

        let target_hash = *target_channel_h;

        let target_channel = trustee.get_channel(&ChannelHash(target_hash), i)?;

        // Encrypt share for target trustee
        let encryption_pk = PublicKey::<C>::from_element(&target_channel.channel_pk, &ctx);

        let share_bytes = ctx.encrypt_exp(&share, encryption_pk);

        s.push(share_bytes?)
    }

    let commitment_proof = match cfg.dkg_commitment_proof_policy {
        DkgCommitmentProofPolicy::Legacy => None,
        DkgCommitmentProofPolicy::SchnorrPok => {
            let (Some(constant), Some(commitment)) = (coeffs.first(), commitments.first()) else {
                return Err(ProtocolError::InternalError(
                    "Missing polynomial coefficients".to_string(),
                ));
            };
            let position = cfg
                .get_trustee_position(&trustee.get_pk()?)
                .ok_or_else(|| {
                    ProtocolError::InternalError("Trustee not found in configuration".to_string())
                })?;
            let label = commitment_proof_label(cfg, position);
            let proof = Zkp::new(&ctx).schnorr_prove(constant, commitment, None, &label)?;
            Some(proof)
        }
    };

    let shares = Shares {
        commitments: commitments,
        encrypted_shares: s,
        commitment_proof,
    };
    let m = Message::shares_msg(cfg, &shares, trustee)?;
    Ok(vec![m])
}

/// Builds the label the commitment proof of a dealer is bound to.
fn commitment_proof_label<C: Ctx>(cfg: &Configuration<C>, dealer: TrusteePosition) -> Vec<u8> {
    cfg.label(0, format!("shares commitment proof {dealer}"))
}

/// Checks that a dealer posted one commitment per threshold step and one encrypted share per trustee.
fn check_shares_shape<C: Ctx>(
    share: &Shares<C>,
    threshold: usize,
    num_t: usize,
    sender: TrusteePosition,
) -> Result<(), ProtocolError> {
    if share.commitments.len() != threshold {
        return Err(ProtocolError::VerificationError(format!(
            "Shares from trustee {} have {} commitments, expected {}",
            sender,
            share.commitments.len(),
            threshold
        )));
    }
    if share.encrypted_shares.len() != num_t {
        return Err(ProtocolError::VerificationError(format!(
            "Shares from trustee {} have {} encrypted shares, expected {}",
            sender,
            share.encrypted_shares.len(),
            num_t
        )));
    }
    Ok(())
}

/// Under DkgCommitmentProofPolicy::SchnorrPok, verifies the proof of
/// knowledge of the discrete log of the sender's first commitment.
fn verify_commitment_proof<C: Ctx>(
    cfg: &Configuration<C>,
    share: &Shares<C>,
    sender: TrusteePosition,
    zkp: &Zkp<C>,
) -> Result<(), ProtocolError> {
    match cfg.dkg_commitment_proof_policy {
        DkgCommitmentProofPolicy::Legacy => Ok(()),
        DkgCommitmentProofPolicy::SchnorrPok => {
            let (Some(commitment), Some(proof)) =
                (share.commitments.first(), share.commitment_proof.as_ref())
            else {
                return Err(ProtocolError::VerificationError(format!(
                    "Shares from trustee {} have no commitment proof",
                    sender
                )));
            };
            let label = commitment_proof_label(cfg, sender);
            if zkp.schnorr_verify(commitment, None, proof, &label) {
                Ok(())
            } else {
                Err(ProtocolError::VerificationError(format!(
                    "Failed to verify commitment proof on shares from trustee {}",
                    sender
                )))
            }
        }
    }
}

/// Computes the public key corresponding to the shares.
///
/// Includes verifying this trustee's shares.
///
/// Returns a Message of type PublicKey signed by
/// this trustee.
pub(super) fn compute_pk<C: Ctx, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    let pk = compute_pk_(
        cfg_h,
        shares_hs,
        channels_hs,
        self_pos,
        num_t,
        threshold,
        trustee,
    )
    .add_context("Computing pk")?;

    let public_key: DkgPublicKey<C> = DkgPublicKey::new(pk.0, pk.1);

    let m = Message::public_key_msg(cfg, &public_key, shares_hs, channels_hs, true, trustee)?;
    Ok(vec![m])
}

/// Verifies the public key re-computing it independently.
///
/// Includes verifying this trustee's shares.
///
/// Returns a Message of type PublicKeySigned signed by
/// this trustee.
pub(super) fn sign_pk<C: Ctx, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    pk_h: &PublicKeyHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<Vec<Message>, ProtocolError> {
    let cfg = trustee.get_configuration(cfg_h)?;
    info!(
        "SignPk verifying public key [{}] ({})..",
        dbg_hash(&pk_h.0),
        num_t,
    );

    let expected = compute_pk_(
        cfg_h,
        shares_hs,
        channels_hs,
        self_pos,
        num_t,
        threshold,
        trustee,
    )?;

    let actual = trustee
        .get_dkg_public_key(pk_h, 0)
        .add_context("Signing pk")?;

    if (expected.0 == actual.pk) && (expected.1 == actual.verification_keys) {
        info!(
            "SignPk verifying public key [{}] ({}), ok",
            dbg_hash(&pk_h.0),
            num_t,
        );
        let m = Message::public_key_msg(cfg, &actual, shares_hs, channels_hs, false, trustee)?;
        Ok(vec![m])
    } else {
        Err(ProtocolError::VerificationError(format!(
            "Mismatch when comparing computed public key with retrieved one"
        )))
    }
}

/// Computes the public key from the shares.
///
/// Each trustee's Shares are first checked for the expected number of
/// commitments and encrypted shares and, under
/// DkgCommitmentProofPolicy::SchnorrPok, for a valid commitment proof.
/// Then this trustee's Channel is retrieved, and the private
/// key is decrypted. This key is then used to decrypts the shares
/// sent to this trustee, which are verified using the commitments.
/// The share commitments are then used to compute the public key as
/// well as the all trustee's verification keys (used to verify
/// decryptions).
///
/// Returns the public key and the verification keys for
/// all trustees.
///
/// As described in Cortier et al.; based on Pedersen.
fn compute_pk_<C: Ctx, S: crate::protocol::board::LocalBoardStorage>(
    cfg_h: &ConfigurationHash,
    shares_hs: &SharesHashes,
    channels_hs: &ChannelsHashes,
    self_pos: &TrusteePosition,
    num_t: &TrusteeCount,
    threshold: &TrusteeCount,
    trustee: &Trustee<C, S>,
) -> Result<(C::E, Vec<C::E>), ProtocolError> {
    let ctx = C::default();
    let cfg = trustee.get_configuration(cfg_h)?;
    let zkp = Zkp::new(&ctx);
    let mut pk = C::E::mul_identity();
    let mut verification_keys = vec![C::E::mul_identity(); *num_t];

    // Iterate over sender shares
    for (i, _h) in shares_hs.0.iter().filter(|h| **h != NULL_HASH).enumerate() {
        let share_h = shares_hs.0[i];
        let share = trustee.get_shares(&SharesHash(share_h), i)?;
        check_shares_shape(&share, *threshold, *num_t, i)?;
        verify_commitment_proof(cfg, &share, i, &zkp)?;

        pk = pk.mul(&share.commitments[0]).modp(&ctx);

        // Iterate over receiver trustees to compute their verification key
        for (j, vk) in verification_keys.iter_mut().enumerate().take(*num_t) {
            let vkf =
                strand::threshold::verification_key_factor(&share.commitments, *threshold, j, &ctx);

            *vk = vk.mul(&vkf).modp(&ctx);

            // Our share is sent from trustee i to j, when j = us
            if j == *self_pos {
                // Construct our private key to decrypt our share
                let my_channel_h =
                    channels_hs
                        .0
                        .get(*self_pos)
                        .ok_or(ProtocolError::InternalError(
                            "Could not retrieve channel hash for self".to_string(),
                        ))?;

                let my_channel = trustee
                    .get_channel(&ChannelHash(*my_channel_h), *self_pos)
                    .add_context("Retrieving channel for self")?;

                let sk = trustee.decrypt_share_sk(&my_channel, &cfg)?;

                // Decrypt the share sent from i to us
                let value = ctx.decrypt_exp(&share.encrypted_shares[*self_pos], sk)?;
                // Verify the share
                let ok = strand::threshold::verify_share(&value, &vkf, &ctx);
                if !ok {
                    return Err(ProtocolError::VerificationError(format!(
                        "Trustee {} failed to verify share from {}..",
                        j, i
                    )));
                }
                trace!("Trustee {} verified share received from {}", j, i);
            }
        }
    }

    Ok((pk, verification_keys))
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use crate::native::board::NoOpStorage;
    use crate::native::test::vector_board::VectorBoard;
    use b4::messages::protocol_manager::ProtocolManager;
    use b4::messages::statement::StatementType;
    use std::marker::PhantomData;
    use strand::backend::ristretto::RistrettoCtx;
    use strand::serialization::{StrandDeserialize, StrandSerialize};
    use strand::signature::{StrandSignaturePk, StrandSignatureSk};

    type C = RistrettoCtx;

    const NUM_TRUSTEES: usize = 2;
    const THRESHOLD: usize = 2;
    const DEALER: TrusteePosition = 1;
    const MAX_CYCLES: usize = 10;

    /// Creates a board with a configuration and the trustees needed to run the DKG tests.
    fn create_dkg(
        policy: DkgCommitmentProofPolicy,
    ) -> (Configuration<C>, Vec<Trustee<C, NoOpStorage>>, VectorBoard) {
        let pm = ProtocolManager::<C> {
            signing_key: StrandSignatureSk::generate().unwrap(),
            phantom: PhantomData,
        };
        let (trustees, trustee_pks): (Vec<_>, Vec<_>) = (0..NUM_TRUSTEES)
            .map(|i| {
                let sk = StrandSignatureSk::generate().unwrap();
                let pk = StrandSignaturePk::from_sk(&sk).unwrap();
                let trustee = Trustee::new(
                    i.to_string(),
                    "dkg".to_string(),
                    sk,
                    strand::symm::gen_key(),
                    NoOpStorage::new(),
                    None,
                );
                (trustee, pk)
            })
            .unzip();
        let mut cfg = Configuration::<C>::new(
            0,
            StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
            trustee_pks,
            THRESHOLD,
            PhantomData,
        );
        cfg.dkg_commitment_proof_policy = policy;
        let mut board = VectorBoard::new(0);
        board.add(Message::bootstrap_msg(&cfg, &pm).unwrap());

        (cfg, trustees, board)
    }

    /// Returns the `Shares` messages posted on the board.
    fn posted_shares(board: &VectorBoard) -> Vec<Shares<C>> {
        board
            .messages
            .iter()
            .map(|m| Message::strand_deserialize(&m.message).unwrap())
            .filter(|m| m.statement.get_kind() == StatementType::Shares)
            .map(|m| Shares::<C>::strand_deserialize(m.artifact.as_ref().unwrap()).unwrap())
            .collect()
    }

    /// The outcome of running the dkg.
    struct DkgRun {
        /// The public key posted by trustee 0, if any.
        public_key: Option<DkgPublicKey<C>>,
        /// The errors returned by the trustees' steps.
        errors: Vec<String>,
    }

    impl DkgRun {
        /// Returns true if the result is an error whose text contains `reason`.
        fn failed_with(&self, reason: &str) -> bool {
            self.errors.iter().any(|error| error.contains(reason))
        }
    }

    /// Runs the dkg. DEALER posts its Shares once the Shares of every other
    /// trustee are on the board, after passing them through `replace` along
    /// with those other Shares.
    fn run_dkg(
        cfg: &Configuration<C>,
        trustees: &mut [Trustee<C, NoOpStorage>],
        board: &mut VectorBoard,
        replace: impl Fn(Shares<C>, &[Shares<C>]) -> Shares<C>,
    ) -> DkgRun {
        let mut errors = vec![];
        let mut last_ids = vec![-1i64; trustees.len()];
        for _ in 0..MAX_CYCLES {
            for (position, trustee) in trustees.iter_mut().enumerate() {
                let messages = board.get(last_ids[position]);
                last_ids[position] += messages.len() as i64;
                let result = match trustee.step(&messages) {
                    Ok(result) => result,
                    Err(error) => {
                        errors.push(error.to_string());
                        continue;
                    }
                };
                for message in result.messages {
                    let message = if position == DEALER
                        && message.statement.get_kind() == StatementType::Shares
                    {
                        let others = posted_shares(board);
                        if others.len() < NUM_TRUSTEES - 1 {
                            continue;
                        }
                        let bytes = message.artifact.as_ref().unwrap();
                        let shares = Shares::<C>::strand_deserialize(bytes).unwrap();
                        Message::shares_msg(cfg, &replace(shares, &others), trustee).unwrap()
                    } else {
                        message
                    };
                    board.add(message);
                }
            }
        }

        DkgRun {
            public_key: trustees[0]._get_dkg_public_key_nohash(),
            errors,
        }
    }

    /// Leaves the shares as the trustee produced them.
    fn unchanged(shares: Shares<C>, _others: &[Shares<C>]) -> Shares<C> {
        shares
    }

    /// Removes the commitment proof from the shares.
    fn without_commitment_proof(shares: Shares<C>, _others: &[Shares<C>]) -> Shares<C> {
        let bytes = (shares.commitments, shares.encrypted_shares)
            .strand_serialize()
            .unwrap();
        Shares::<C>::strand_deserialize(&bytes).unwrap()
    }

    /// A DKG with well formed shares publishes the public key.
    #[test]
    fn public_key_published_from_well_formed_shares() {
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::SchnorrPok);

        let run = run_dkg(&cfg, &mut trustees, &mut board, unchanged);

        assert!(run.public_key.is_some());
        assert!(run.errors.is_empty());
    }

    /// Under the `Legacy` policy, shares without a commitment proof are accepted.
    #[test]
    fn legacy_policy_accepts_shares_without_commitment_proof() {
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::Legacy);

        let run = run_dkg(&cfg, &mut trustees, &mut board, without_commitment_proof);

        assert!(run.public_key.is_some());
        assert!(run.errors.is_empty());
    }

    /// Under `SchnorrPok`, a dealer without a commitment proof is rejected.
    #[test]
    fn public_key_requires_commitment_proof_from_every_trustee() {
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::SchnorrPok);

        let run = run_dkg(&cfg, &mut trustees, &mut board, without_commitment_proof);

        assert!(run.public_key.is_none());
        assert!(run.failed_with("have no commitment proof"));
    }

    /// Under `SchnorrPok`, a commitment proof that does not verify is rejected.
    #[test]
    fn public_key_requires_valid_commitment_proof() {
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::SchnorrPok);

        let run = run_dkg(&cfg, &mut trustees, &mut board, |mut shares, _| {
            if let Some(proof) = shares.commitment_proof.as_mut() {
                proof.response = proof.response.add(&proof.challenge);
            }
            shares
        });

        assert!(run.public_key.is_none());
        assert!(run.failed_with("Failed to verify commitment proof"));
    }

    /// The proof is bound to the position of the trustee that posts it.
    #[test]
    fn public_key_rejects_commitment_proof_posted_by_another_trustee() {
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::SchnorrPok);

        let run = run_dkg(&cfg, &mut trustees, &mut board, |_, others| {
            let bytes = others[0].strand_serialize().unwrap();
            Shares::<C>::strand_deserialize(&bytes).unwrap()
        });

        assert!(run.public_key.is_none());
        assert!(run.failed_with("Failed to verify commitment proof"));
    }

    /// The replaced commitments still verify the share sent to trustee 0.
    #[test]
    fn public_key_rejects_commitments_derived_from_other_trustees_shares() {
        let ctx = C::default();
        let chosen_pk = ctx.gmod_pow(&ctx.rnd_exp(&mut ctx.get_rng()));
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::SchnorrPok);

        let run = run_dkg(&cfg, &mut trustees, &mut board, |mut shares, others| {
            let first_share = shares.commitments[0].mul(&shares.commitments[1]).modp(&ctx);
            let constant = others.iter().fold(chosen_pk.clone(), |acc, other| {
                acc.divp(&other.commitments[0], &ctx).modp(&ctx)
            });
            let linear = first_share.divp(&constant, &ctx).modp(&ctx);
            shares.commitments = vec![constant, linear];
            shares
        });

        assert_ne!(
            run.public_key.as_ref().map(|pk| pk.pk.clone()),
            Some(chosen_pk)
        );
        assert!(run.public_key.is_none());
        assert!(run.failed_with("Failed to verify commitment proof"));
    }

    /// A dealer with a number of commitments other than the threshold is rejected.
    #[test]
    fn public_key_requires_threshold_commitments_from_every_trustee() {
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::SchnorrPok);

        let run = run_dkg(&cfg, &mut trustees, &mut board, |mut shares, _| {
            shares.commitments.push(shares.commitments[0].clone());
            shares
        });

        assert!(run.public_key.is_none());
        assert!(run.failed_with(&format!(
            "have {} commitments, expected {}",
            THRESHOLD + 1,
            THRESHOLD
        )));
    }

    /// A dealer with a number of encrypted shares other than the trustee count is rejected.
    #[test]
    fn public_key_requires_one_encrypted_share_per_trustee() {
        let (cfg, mut trustees, mut board) = create_dkg(DkgCommitmentProofPolicy::SchnorrPok);

        let run = run_dkg(&cfg, &mut trustees, &mut board, |mut shares, _| {
            shares
                .encrypted_shares
                .push(shares.encrypted_shares[0].clone());
            shares
        });

        assert!(run.public_key.is_none());
        assert!(run.failed_with(&format!(
            "have {} encrypted shares, expected {}",
            NUM_TRUSTEES + 1,
            NUM_TRUSTEES
        )));
    }
}
