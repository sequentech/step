use anyhow::Result;
use strum::Display;

pub(self) use log::{debug, error, info, trace};
pub(self) use strand::context::Ctx;
pub(self) use strand::context::Element;
pub(self) use strand::context::Exponent;

pub(self) use crate::protocol2::datalog::NULL_HASH;
pub(self) use braid_messages::message::Message;
pub(self) use braid_messages::artifact::{
    DecryptionFactors, DkgPublicKey, Mix, Plaintexts, Shares,
};
pub(self) use braid_messages::newtypes::*;
pub(self) use crate::protocol2::trustee::Trustee;

/*pub(crate) use crate::protocol2::predicate::BatchNumber;
pub(crate) use crate::protocol2::predicate::ChannelHash;
pub(crate) use crate::protocol2::predicate::ChannelsHashes;
pub(crate) use crate::protocol2::predicate::CiphertextsHash;
pub(crate) use crate::protocol2::predicate::ConfigurationHash;
pub(crate) use crate::protocol2::predicate::DecryptionFactorsHash;
pub(crate) use crate::protocol2::predicate::DecryptionFactorsHashes;
pub(crate) use crate::protocol2::predicate::PlaintextsHash;
pub(crate) use crate::protocol2::predicate::PublicKeyHash;
pub(crate) use crate::protocol2::predicate::SharesHash;
pub(crate) use crate::protocol2::predicate::SharesHashes;
pub(crate) use crate::protocol2::predicate::TrusteeSet;
pub(crate) use crate::protocol2::predicate::{MixNumber, TrusteeCount, TrusteePosition};

pub(crate) use crate::protocol2::PROTOCOL_MANAGER_INDEX;*/



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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Display)]
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
    pub(crate) fn run<C: Ctx>(&self, trustee: &Trustee<C>) -> Result<Vec<Message>> {
        info!("Running action {}..", &self);
        match self {
            Self::SignConfiguration(cfg_h) => cfg::sign_config(cfg_h, trustee),
            Self::GenChannel(cfg_h) => dkg::gen_channel(cfg_h, trustee),
            Self::SignChannels(cfg_h, chs) => dkg::sign_channels(cfg_h, chs, trustee),
            Self::ComputeShares(cfg_h, commitments_hs, num_t, th) => {
                dkg::compute_shares(cfg_h, commitments_hs, num_t, th, trustee)
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
                commitments_hs,
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
                commitments_hs,
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
    pub(crate) fn run_for_verifier<C: Ctx>(&self, trustee: &Trustee<C>) -> Result<Vec<Message>> {
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

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignConfiguration(h) => {
                write!(f, "SignConfig{{ cfg hash={:?} }}", dbg_hash(&h.0))
            }
            Self::GenChannel(h) => {
                write!(f, "GenChannel{{ cfg hash={:?} }}", dbg_hash(&h.0),)
            }
            Self::SignChannels(h, chs) => {
                write!(
                    f,
                    "SignChannels{{ cfg hash={:?}, commitments_hs={:?} }}",
                    dbg_hash(&h.0),
                    chs
                )
            }
            Self::ComputeShares(h, chs, num_t, th) => {
                write!(
                    f,
                    "ComputeShares{{ cfg hash={:?}, chs={:?}, #trustees={}, threshold={:?}",
                    dbg_hash(&h.0),
                    chs,
                    num_t,
                    th
                )
            }
            Self::ComputePublicKey(cfg_h, _sh_hs, _cm_hs, _self_pos, _num_t, _th) => {
                write!(f, "ComputePublicKey{{ cfg hash={:?} }}", dbg_hash(&cfg_h.0))
            }
            Self::SignPublicKey(cfg_h, pk_h, sh_hs, cm_hs, _self_pos, _num_t, _th) => {
                write!(
                    f,
                    "SignPublicKey{{ cfg hash={:?}, pk hash={:?}, shares_hs={:?}, commitments_hs={:?} }}",
                    dbg_hash(&cfg_h.0), dbg_hash(&pk_h.0), sh_hs.0.map(|h| dbg_hash(&h)), cm_hs.0.map(|h| dbg_hash(&h))
                )
            }
            Self::Mix(cfg_h, batch, ciphertexts_h, pk_h, signer_t, mix_no, trustees) => {
                write!(f, "Mix{{ cfg_h={:?} batch={:?} cipher_h={:?} pk_h={:?}, signer_t={:?}, mix_n={:?}, num_t={:?}}}", dbg_hash(&cfg_h.0), batch, dbg_hash(&ciphertexts_h.0), dbg_hash(&pk_h.0), signer_t, mix_no, trustees)
            }
            Self::SignMix(
                cfg_h,
                batch,
                source_h,
                signers_t,
                ciphertexts_h,
                signert_t,
                _pk_h,
                mix_n,
            ) => {
                write!(
                    f,
                    "SignMix{{ cfg_h={:?} batch={:?} source_h={:?}, cipher_h={:?} signers_t={:?}, signert_t={:?}, mix_n={:?} }}",
                    dbg_hash(&cfg_h.0), batch, dbg_hash(&source_h.0), dbg_hash(&ciphertexts_h.0), signers_t, signert_t, mix_n
                )
            }
            Self::ComputeDecryptionFactors(
                _cfg_h,
                _batch,
                _commitments_hs,
                _ciphertexts_h,
                _signer_t,
                _pk_h,
                _shares_hs,
                _self_p,
                _num_t,
                _threshold,
                _selected,
            ) => {
                write!(f, "ComputeDecryptionFactors {{ }}",)
            }
            Self::ComputePlaintexts(
                _cfg_h,
                _batch,
                _pk_h,
                _dfactor_hs,
                _ciphertexts_h,
                _mix_signer,
                _ts,
                _th,
            ) => {
                write!(f, "ComputePlaintexts {{ }}",)
            }
            Self::SignPlaintexts(
                _cfg_h,
                _batch,
                _pk_h,
                _plaintexts_h,
                _dfactor_hs,
                _ciphertexts_h,
                _mix_signer,
                _ts,
                _th,
            ) => {
                write!(f, "SignPlaintexts {{ }}",)
            }
        }
    }
}
