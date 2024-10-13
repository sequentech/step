// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use strum::Display;

pub(self) use log::{debug, info, trace};
pub(self) use strand::context::Ctx;
pub(self) use strand::context::Element;
pub(self) use strand::context::Exponent;

pub(self) use crate::protocol::datalog::NULL_HASH;
pub(self) use crate::protocol::trustee2::Trustee;
pub(self) use crate::util::{ProtocolContext, ProtocolError};
pub(self) use b3::messages::artifact::{DecryptionFactors, DkgPublicKey, Mix, Plaintexts, Shares};
pub(self) use b3::messages::message::Message;
pub(self) use b3::messages::newtypes::*;

// Used by submodules
use crate::util::dbg_hash;

///////////////////////////////////////////////////////////////////////////
// Action
///////////////////////////////////////////////////////////////////////////

/// A protocol action that produces Messages that advances
/// the protocol state.
///
/// Actions are the core operations that make up the protocol
/// as a multi party (trustee) computation. An Action is variant
/// which defines what Action to compute, and contains pointers
/// to all the input artifacts that it needs for its computation.
/// These artifacts are retrieved from the trustee's LocalBoard.
///
/// The pointers are always hashes of the target artifact, and
/// will have been signed by the producer of said artifact.
/// The binding of trustees, their signatures, the artifact
/// hashes and their computations is implemented through datalog
/// relations. Actions are datalog predicates that are inferred
/// as required for the protocol to execute, based on existing
/// messages on the bulletin board.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Display, Debug)]
pub enum Action {
    SignConfiguration(ConfigurationHash),
    GenChannel(ConfigurationHash),
    SignChannels(
        ConfigurationHash,
        ChannelsHashes,
        TrusteePosition,
        TrusteeCount,
    ),
    ComputeShares(
        ConfigurationHash,
        ChannelsHashes,
        TrusteeCount,
        TrusteeCount,
    ),
    ComputePublicKey(
        ConfigurationHash,
        SharesHashes,
        ChannelsHashes,
        TrusteePosition,
        TrusteeCount,
        TrusteeCount,
    ),
    SignPublicKey(
        ConfigurationHash,
        PublicKeyHash,
        SharesHashes,
        ChannelsHashes,
        TrusteePosition,
        TrusteeCount,
        TrusteeCount,
    ),
    Mix(
        ConfigurationHash,
        BatchNumber,
        CiphertextsHash,
        PublicKeyHash,
        TrusteePosition,
        MixNumber,
        TrusteeSet,
    ),
    SignMix(
        ConfigurationHash,
        BatchNumber,
        CiphertextsHash,
        TrusteePosition,
        CiphertextsHash,
        TrusteePosition,
        PublicKeyHash,
        MixNumber,
    ),

    ComputeDecryptionFactors(
        ConfigurationHash,
        BatchNumber,
        ChannelsHashes,
        CiphertextsHash,
        TrusteePosition,
        PublicKeyHash,
        SharesHashes,
        TrusteePosition,
        TrusteeCount,
        TrusteeCount,
        TrusteeSet,
    ),
    ComputePlaintexts(
        ConfigurationHash,
        BatchNumber,
        PublicKeyHash,
        DecryptionFactorsHashes,
        CiphertextsHash,
        TrusteePosition,
        TrusteeSet,
        TrusteeCount,
    ),
    SignPlaintexts(
        ConfigurationHash,
        BatchNumber,
        PublicKeyHash,
        PlaintextsHash,
        DecryptionFactorsHashes,
        CiphertextsHash,
        TrusteePosition,
        TrusteeSet,
        TrusteeCount,
    ),
}

impl Action {
    ///////////////////////////////////////////////////////////////////////////
    // Action dispatch to target functions
    ///////////////////////////////////////////////////////////////////////////

    /// Runs this Action.
    ///
    /// Actions produce Messages that will be posted to the bulletin board.
    /// The significant steps are
    ///
    /// 1) Dispatch action to target function (pattern match on self)
    /// 2) The target function will: retrieve necessary input artifacts
    /// from trustee (LocalBoard)
    /// 3) Compute necessary data
    /// 4) Create messages through Message static functions
    ///      4.1) Message::<function> Computes hashes and artifact data
    ///      4.2) Trustee::<message> Signs the statement and returns Message
    pub(crate) fn run<C: Ctx>(&self, trustee: &Trustee<C>) -> Result<Vec<Message>, ProtocolError> {
        info!("Running action {}..", &self);
        match self {
            Self::SignConfiguration(cfg_h) => cfg::sign_config(cfg_h, trustee),
            Self::GenChannel(cfg_h) => dkg::gen_channel(cfg_h, trustee),
            Self::SignChannels(cfg_h, chs, self_pos, th) => {
                dkg::sign_channels(cfg_h, chs, self_pos, th, trustee)
            }
            Self::ComputeShares(cfg_h, channels_hs, num_t, th) => {
                dkg::compute_shares(cfg_h, channels_hs, num_t, th, trustee)
            }
            Self::ComputePublicKey(cfg_h, sh_hs, cm_hs, self_pos, num_t, th) => {
                dkg::compute_pk(cfg_h, sh_hs, cm_hs, self_pos, num_t, th, trustee)
            }
            Self::SignPublicKey(cfg_h, pk_h, sh_hs, cm_hs, self_pos, num_t, th) => {
                dkg::sign_pk(cfg_h, pk_h, sh_hs, cm_hs, self_pos, num_t, th, trustee)
            }
            Self::Mix(cfg_h, batch, ciphertexts_h, pk_h, signer_t, mix_n, trustees) => {
                shuffle::mix(
                    cfg_h,
                    batch,
                    ciphertexts_h,
                    pk_h,
                    *signer_t,
                    mix_n,
                    trustees,
                    trustee,
                )
            }
            Self::SignMix(
                cfg_h,
                batch,
                source_h,
                signers_t,
                ciphertexts_h,
                signert_t,
                pk_h,
                mix_n,
            ) => shuffle::sign_mix(
                cfg_h,
                batch,
                source_h,
                *signers_t,
                ciphertexts_h,
                *signert_t,
                pk_h,
                mix_n,
                trustee,
            ),
            Self::ComputeDecryptionFactors(
                cfg_h,
                batch,
                channels_hs,
                ciphertexts_h,
                signer_t,
                pk_h,
                shares_hs,
                self_p,
                num_t,
                _threshold,
                _selected,
            ) => decrypt::compute_decryption_factors(
                cfg_h,
                batch,
                channels_hs,
                ciphertexts_h,
                signer_t,
                pk_h,
                shares_hs,
                self_p,
                num_t,
                trustee,
            ),
            Self::ComputePlaintexts(
                cfg_h,
                batch,
                pk_h,
                dfactors_hs,
                ciphertexts_h,
                mix_signer,
                ts,
                th,
            ) => decrypt::compute_plaintexts(
                cfg_h,
                batch,
                pk_h,
                dfactors_hs,
                ciphertexts_h,
                mix_signer,
                ts,
                th,
                trustee,
            ),
            Self::SignPlaintexts(
                cfg_h,
                batch,
                pk_h,
                plaintexts_h,
                dfactor_hs,
                ciphertexts_h,
                mix_signer,
                trustees,
                threshold,
            ) => decrypt::sign_plaintexts(
                cfg_h,
                batch,
                pk_h,
                plaintexts_h,
                dfactor_hs,
                ciphertexts_h,
                mix_signer,
                trustees,
                threshold,
                trustee,
            ),
        }
    }

    /// Runs this Action in verifying mode.
    ///
    /// Only three actions are relevant for a verifier.
    pub(crate) fn run_for_verifier<C: Ctx>(
        &self,
        trustee: &Trustee<C>,
    ) -> Result<Vec<Message>, ProtocolError> {
        match self {
            Self::SignPublicKey(cfg_h, pk_h, sh_hs, cm_hs, self_pos, num_t, th) => {
                dkg::sign_pk(cfg_h, pk_h, sh_hs, cm_hs, self_pos, num_t, th, trustee)
            }
            Self::SignMix(
                cfg_h,
                batch,
                source_h,
                signers_t,
                ciphertexts_h,
                signert_t,
                pk_h,
                mix_n,
            ) => shuffle::sign_mix(
                cfg_h,
                batch,
                source_h,
                *signers_t,
                ciphertexts_h,
                *signert_t,
                pk_h,
                mix_n,
                trustee,
            ),
            Self::SignPlaintexts(
                cfg_h,
                batch,
                pk_h,
                plaintexts_h,
                dfactor_hs,
                ciphertexts_h,
                mix_signer,
                trustees,
                threshold,
            ) => decrypt::sign_plaintexts(
                cfg_h,
                batch,
                pk_h,
                plaintexts_h,
                dfactor_hs,
                ciphertexts_h,
                mix_signer,
                trustees,
                threshold,
                trustee,
            ),
            // none of the other actions are relevant for a verifier
            _ => Ok(vec![]),
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// Target function modules
///////////////////////////////////////////////////////////////////////////

/// Configuration approval and signing.
mod cfg;
/// Distributed verifiable decryption.
///
/// As described in Cortier et al.; based on Pedersen. Decryption
/// is verifiable through Chaum-Pedersen proofs of discrete
/// log equality.
mod decrypt;
/// Distributed key generation.
///
/// As described in Cortier et al.; based on Pedersen.
mod dkg;
/// Verifiable shuffling.
///
/// As described in Haenni et al.; Haines, based on Wikstrom et al.
mod shuffle;
