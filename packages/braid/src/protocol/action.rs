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
pub(self) use board_messages::braid::artifact::{
    DecryptionFactors, DkgPublicKey, Mix, Plaintexts, Shares,
};
pub(self) use board_messages::braid::message::Message;
pub(self) use board_messages::braid::newtypes::*;

use crate::util::dbg_hash;

///////////////////////////////////////////////////////////////////////////
// Action
//
// Actions produce Messages that will be posted to the bulletin board.
//
// 1) Dispatch action to target function (pattern match on enum)
// 2) Retrieve necessary artifacts from trustee (localboard)
// 3) Compute necessary data
// 44) Create message through Message static functions
//      4.1) Message::<function> Computes hashes and artifact data
//      4.2) Trustee::<message> Signs the statement and returns Message
///////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq, Hash, Display, Debug)]
pub enum Action {
    SignConfiguration(ConfigurationHash),
    GenChannel(ConfigurationHash),
    SignChannels(ConfigurationHash, ChannelsHashes),
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
    pub(crate) fn run<C: Ctx>(&self, trustee: &Trustee<C>) -> Result<Vec<Message>, ProtocolError> {
        info!("Running action {}..", &self);
        match self {
            Self::SignConfiguration(cfg_h) => cfg::sign_config(cfg_h, trustee),
            Self::GenChannel(cfg_h) => dkg::gen_channel(cfg_h, trustee),
            Self::SignChannels(cfg_h, chs) => dkg::sign_channels(cfg_h, chs, trustee),
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

    // Only three actions are relevant for a verifier
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

mod cfg;
mod decrypt;
mod dkg;
mod shuffle;
