use std::collections::HashSet;
use strum::Display;

use anyhow::Result;
use log::trace;
use borsh::{BorshDeserialize, BorshSerialize};


use strand::signature::StrandSignaturePk;
use strand::{context::Ctx, serialization::StrandSerialize};

use crate::protocol2::artifact::Configuration;
use crate::protocol2::datalog::NULL_TRUSTEE;
use crate::protocol2::statement::Statement;
use crate::protocol2::statement::THashes;
use crate::protocol2::PROTOCOL_MANAGER_INDEX;

///////////////////////////////////////////////////////////////////////////
// Predicate
//
// Predicates are enum variants passed as inputs to MachineState Datalog
// relations for inference, and outputted by MachineStates to forward to
// subsequent MachineStates.
// They contain newtype tuple members, except for members
// that require doing arithmetic.
// Predicates are derived from Statements, using the general
// Predicate::from_statement method.
///////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq, Hash, Display)]
pub(crate) enum Predicate {
    // Input predicates
    /// Bootstrap //////////////////////////////////////////////////////////////////
    Configuration(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold),
    ConfigurationSigned(ConfigurationHash, TrusteePosition),

    /// Dkg ////////////////////////////////////////////////////////////////////////
    Channel(ConfigurationHash, ChannelHash, TrusteePosition),
    ChannelsSigned(ConfigurationHash, ChannelsHashes, TrusteePosition),
    ChannelsAllSignedAll(ConfigurationHash, ChannelsHashes),
    Shares(ConfigurationHash, SharesHash, TrusteePosition),
    PublicKey(
        ConfigurationHash,
        PublicKeyHash,
        SharesHashes,
        ChannelsHashes,
        TrusteePosition,
    ),
    PublicKeySigned(
        ConfigurationHash,
        PublicKeyHash,
        SharesHashes,
        ChannelsHashes,
        TrusteePosition,
    ),

    /// Shuffle ////////////////////////////////////////////////////////////////////
    Ballots(
        ConfigurationHash,
        BatchNumber,
        CiphertextsHash,
        PublicKeyHash,
        TrusteeSet,
    ),
    // A mix predicate describes the mix itself but also specifies its position (starting at 1)
    Mix(
        ConfigurationHash,
        BatchNumber,
        CiphertextsHash,
        CiphertextsHash,
        MixNumber,
        TrusteePosition,
    ),
    MixSigned(
        ConfigurationHash,
        BatchNumber,
        CiphertextsHash,
        CiphertextsHash,
        TrusteePosition,
    ),

    /// Decrypt ////////////////////////////////////////////////////////////////////
    DecryptionFactors(
        ConfigurationHash,
        BatchNumber,
        DecryptionFactorsHash,
        CiphertextsHash,
        SharesHashes,
        TrusteePosition,
    ),
    Plaintexts(
        ConfigurationHash,
        BatchNumber,
        PlaintextsHash,
        DecryptionFactorsHashes,
        CiphertextsHash,
        PublicKeyHash,
        TrusteePosition,
    ),
    PlaintextsSigned(
        ConfigurationHash,
        BatchNumber,
        PlaintextsHash,
        DecryptionFactorsHashes,
        CiphertextsHash,
        PublicKeyHash,
        TrusteePosition,
    ),

    // Output predicates
    ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold),
    PublicKeySignedAll(ConfigurationHash, PublicKeyHash, SharesHashes),
    MixComplete(
        ConfigurationHash,
        BatchNumber,
        MixNumber,
        CiphertextsHash,
        TrusteePosition,
    ),
}
impl Predicate {
    pub(crate) fn from_statement<C: Ctx>(
        statement: &Statement,
        signer_position: TrusteePosition,
        cfg: &Configuration<C>,
    ) -> Predicate {
        let ret = match statement {
            // Only called for configuration signatures, configuration
            // bootstrap is done through Predicate::get_bootstrap_predicate
            // Configuration(Timestamp, ConfigurationH)
            Statement::Configuration(_, _) => panic!("impossible"),
            Statement::ConfigurationSigned(_ts, cfg_h) => {
                Self::ConfigurationSigned(ConfigurationHash(cfg_h.0), signer_position)
            }
            // Commitments(Timestamp, ConfigurationH, CommitmentsH)
            Statement::Channel(_ts, cfg_h, cm_h) => Self::Channel(
                ConfigurationHash(cfg_h.0),
                ChannelHash(cm_h.0),
                signer_position,
            ),
            // CommitmentsAllSigned(Timestamp, ConfigurationH, CommitmentsHs)
            Statement::ChannelsAllSigned(_ts, cfg_h, cm_hs) => Self::ChannelsSigned(
                ConfigurationHash(cfg_h.0),
                ChannelsHashes(cm_hs.0),
                signer_position,
            ),
            // Shares(Timestamp, ConfigurationH, SharesH)
            Statement::Shares(_ts, cfg_h, sh_h) => Self::Shares(
                ConfigurationHash(cfg_h.0),
                SharesHash(sh_h.0),
                signer_position,
            ),
            // PublicKey(Timestamp, ConfigurationH, PublicKeyH, SharesHs, CommitmentsHs)
            Statement::PublicKey(_ts, cfg_h, pk_h, sh_hs, cm_hs) => Self::PublicKey(
                ConfigurationHash(cfg_h.0),
                PublicKeyHash(pk_h.0),
                SharesHashes(sh_hs.0),
                ChannelsHashes(cm_hs.0),
                signer_position,
            ),
            // PublicKeySigned(Timestamp, ConfigurationH, PublicKeyH, SharesHs, CommitmentsHs)
            Statement::PublicKeySigned(_ts, cfg_h, pk_h, sh_hs, cm_hs) => Self::PublicKeySigned(
                ConfigurationHash(cfg_h.0),
                PublicKeyHash(pk_h.0),
                SharesHashes(sh_hs.0),
                ChannelsHashes(cm_hs.0),
                signer_position,
            ),
            // Ballots(Timestamp, ConfigurationH, usize, CiphertextsH, PublicKeyH, TrusteeSet)
            Statement::Ballots(_ts, cfg_h, batch, ballots_h, pk_h, trustees) => {
                // Verify that all selected trustees are unique
                let mut selected = vec![];
                trustees.iter().for_each(|s| {
                    if *s != NULL_TRUSTEE {
                        assert!(*s > 0 && *s <= cfg.trustees.len());
                        selected.push(*s);
                    }
                });

                let unique: HashSet<usize> = selected.into_iter().collect();
                assert!(unique.len() == cfg.threshold);

                Self::Ballots(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    CiphertextsHash(ballots_h.0),
                    PublicKeyHash(pk_h.0),
                    *trustees,
                )
            }
            // Mix(Timestamp, ConfigurationH, usize, CiphertextsH, CiphertextsH)
            Statement::Mix(_ts, cfg_h, batch, source_h, mix_h, mix_number) => Self::Mix (
                ConfigurationHash(cfg_h.0),
                *batch,
                CiphertextsHash(source_h.0),
                CiphertextsHash(mix_h.0),
                *mix_number,
                signer_position,
            ),
            // MixSigned(Timestamp, ConfigurationH, usize, usize, CiphertextsH, CiphertextsH)
            Statement::MixSigned(_ts, cfg_h, batch, _mix_no, source_h, mix_h) => Self::MixSigned(
                ConfigurationHash(cfg_h.0),
                *batch,
                CiphertextsHash(source_h.0),
                CiphertextsHash(mix_h.0),
                signer_position,
            ),
            // DecryptionFactors(Timestamp, ConfigurationH, usize, DecryptionFactorsH, CiphertextsH, SharesHs)
            Statement::DecryptionFactors(_ts, cfg_h, batch, df_h, mix_h, sh_hs) => {
                Self::DecryptionFactors(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    DecryptionFactorsHash(df_h.0),
                    CiphertextsHash(mix_h.0),
                    SharesHashes(sh_hs.0),
                    signer_position,
                )
            }
            // Plaintexts(Timestamp, ConfigurationH, usize, PlaintextsH, DecryptionFactorsHs, PublicKeyH)
            Statement::Plaintexts(_ts, cfg_h, batch, pl_h, df_hs, c_h, pk_h) => Self::Plaintexts(
                ConfigurationHash(cfg_h.0),
                *batch,
                PlaintextsHash(pl_h.0),
                DecryptionFactorsHashes(df_hs.0),
                CiphertextsHash(c_h.0),
                PublicKeyHash(pk_h.0),
                signer_position,
            ),
            // PlaintextsSigned(Timestamp, ConfigurationH, usize, PlaintextsH, DecryptionFactorsHs, PublicKeyH)
            Statement::PlaintextsSigned(_ts, cfg_h, batch, pl_h, df_hs, c_h, pk_h) => {
                Self::PlaintextsSigned(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    PlaintextsHash(pl_h.0),
                    DecryptionFactorsHashes(df_hs.0),
                    CiphertextsHash(c_h.0),
                    PublicKeyHash(pk_h.0),
                    signer_position,
                )
            }
        };

        trace!("Predicate {:?} derived from statement {:?}", ret, statement);

        ret
    }

    // Special case for bootstrap configuration
    pub(crate) fn get_bootstrap_predicate<C: Ctx>(
        configuration: &Configuration<C>,
        trustee_pk: &StrandSignaturePk,
    ) -> Option<Predicate> {
        let index = configuration.get_trustee_position(trustee_pk)?;
        assert!(index != PROTOCOL_MANAGER_INDEX);

        let p = Predicate::Configuration(
            ConfigurationHash::from_configuration(configuration).ok()?,
            index,
            configuration.trustees.len(),
            configuration.threshold,
        );

        Some(p)
    }

    // Used when a trustee runs in verifier mode
    pub(crate) fn get_verifier_bootstrap_predicate<C: Ctx>(
        configuration: &Configuration<C>,
    ) -> Option<Predicate> {
        let p = Predicate::Configuration(
            ConfigurationHash::from_configuration(configuration).ok()?,
            crate::protocol2::VERIFIER_INDEX,
            configuration.trustees.len(),
            configuration.threshold,
        );

        Some(p)
    }
}

///////////////////////////////////////////////////////////////////////////
// Newtypes
///////////////////////////////////////////////////////////////////////////
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfigurationHash(pub crate::protocol2::Hash);
impl ConfigurationHash {
    pub(crate) fn from_configuration<C: Ctx>(
        configuration: &Configuration<C>,
    ) -> Result<ConfigurationHash> {
        let bytes = configuration.strand_serialize()?;
        let hash = strand::hash::hash(&bytes)?;
        Ok(ConfigurationHash(crate::util::hash_from_vec(&hash)?))
    }
}
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ChannelHash(pub crate::protocol2::Hash);
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ChannelsHashes(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SharesHash(pub crate::protocol2::Hash);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SharesHashes(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PublicKeyHash(pub crate::protocol2::Hash);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
// The ciphertexts hash is used to refer to ballots and mix artifacts.
// This allows accessing either one when pointing to a source of
// ciphertexts (ballots or mix). The same typed hash is propagated
// all the way from Ballots to DecryptionFactors predicates.
pub struct CiphertextsHash(pub crate::protocol2::Hash);

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct DecryptionFactorsHash(pub crate::protocol2::Hash);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct DecryptionFactorsHashes(pub THashes);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct MixingHashes(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PlaintextsHash(pub crate::protocol2::Hash);

// 0-based
pub(crate) type TrusteePosition = usize;
// 1-based
pub(crate) type Threshold = usize;
pub(crate) type TrusteeCount = usize;
// 1-based _elements_
pub(crate) type TrusteeSet = [usize; crate::protocol2::MAX_TRUSTEES];
// 1-based, the position in the mixing chain (note this is not the same as the
// position of the mixing trustee, since active trustees are set by the ballots artifact)
pub(crate) type MixNumber = usize;

pub(crate) type BatchNumber = usize;

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

use crate::util::dbg_hash;

impl std::fmt::Debug for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Configuration(h, t, c, th) => write!(
                f,
                "Configuration(Bootstrap){{ hash={:?}, self={:?}, #trustees={:?}, th={:?} }}",
                dbg_hash(&h.0), t, c, th
            ),
            Predicate::ConfigurationSigned(h, t) => write!(
                f,
                "ConfigurationSigned{{ cfg hash={:?}, signer={:?} }}",
                dbg_hash(&h.0), t
            ),
            Predicate::ConfigurationSignedAll(h, t, c, th) => write!(
                f,
                "ConfigurationSignedAll{{ cfg hash={:?}, signer={:?}, #trustees={:?}, th={:?} }}",
                dbg_hash(&h.0), t, c, th
            ),
            Predicate::Channel(ch, h, t) => write!(
                f,
                "Commitments{{ cfg hash={:?}, hash={:?}, signer={:?} }}",
                dbg_hash(&ch.0), h.0, t
            ),
            Predicate::ChannelsSigned(ch, h, t) => write!(
                f,
                "CommitmentsSigned{{ cfg hash={:?}, hash={:?}, signer={:?} }}",
                dbg_hash(&ch.0), h, t
            ),
            Predicate::ChannelsAllSignedAll(ch, h) => write!(
                f,
                "CommitmentsAllSignedAll{{ cfg hash={:?}, hash={:?} }}",
                dbg_hash(&ch.0), h
            ),
            Predicate::Shares(ch, h, t) => write!(
                f,
                "Shares{{ cfg hash={:?}, hash={:?}, signer={:?} }}",
                dbg_hash(&ch.0), h.0, t
            ),
            Predicate::PublicKey(cfg_h, pk_h, _shares_hs, _commitments_hs, t) => write!(
                f,
                "PublicKey{{ cfg hash={:?}, pk_h={:?}, signer={:?} }}",
                dbg_hash(&cfg_h.0), dbg_hash(&pk_h.0), t
            ),
            Predicate::PublicKeySigned(cfg_h, pk_h, _shares_hs, _commitments_hs, t) => write!(
                f,
                "PublicKeySigned{{ cfg hash={:?}, pk_h={:?}, signer={:?} }}",
                dbg_hash(&cfg_h.0), dbg_hash(&pk_h.0), t
            ),
            Predicate::PublicKeySignedAll(cfg_h, pk_h, _shares_hs) => write!(
                f,
                "PublicKeySignedAll{{ cfg hash={:?}, pk_hash={:?} }}",
                dbg_hash(&cfg_h.0), dbg_hash(&pk_h.0)
            ),
            Predicate::Ballots(cfg_h, batch, cipher_h, pk_h, _ts) => write!(
                f,
                "Ballots{{ cfg hash={:?}, batch={:?}, pk_h={:?} cipher_h={:?} }}",
                dbg_hash(&cfg_h.0), batch, dbg_hash(&pk_h.0), dbg_hash(&cipher_h.0),
            ),
            Predicate::Mix(_cfg_h, _batch, source_h, cipher_h, mix_n, signer_t) => write!(
                f,
                "Mix{{ source_h={:?} cipher_h={:?} mix_n={:?} signer_t={:?} }}",
                dbg_hash(&source_h.0), dbg_hash(&cipher_h.0), mix_n, signer_t
            ),
            Predicate::MixSigned(_cfg_h, _batchh, source_h, cipher_h, signer_t) => write!(
                f,
                "MixSigned{{ source_h={:?} cipher_h={:?} signer_t={:?} }}",
                source_h, dbg_hash(&cipher_h.0), signer_t,
            ),
            Predicate::MixComplete(cfg_h, batch, mix_n, ciphertexts_h, t) => write!(
                f,
                "MixComplete{{ cfg hash={:?}, batch={:?}, mix_n={:?}, ciphertexts={:?} signer_t={:?} }}",
                dbg_hash(&cfg_h.0), batch, mix_n, dbg_hash(&ciphertexts_h.0), t
            ),
            Predicate::DecryptionFactors(_cfg_h, _batch, dfactors_h, _mix_h, _shares_hs, signer_t) => write!(
                f,
                "DecryptionFactors{{ dfactors_h={dfactors_h:?} signer_t={signer_t:?} }}"
            ),
            Predicate::Plaintexts(cfg_h, batch, plaintexts_h, _dfactors_hs, _ciph_h, _pk_h, signer_t) => write!(
                f,
                "Plaintexts{{ cfg hash={:?}, batch={:?}, plaintexts_h={:?}, signer_t={:?} }}",
                dbg_hash(&cfg_h.0), batch, plaintexts_h, signer_t
            ),
            Predicate::PlaintextsSigned(cfg_h, batch, plaintexts_h, _df_hs, _ciph_h, _pk_h, signer_t) => write!(
                f,
                "PlaintextsSigned{{ cfg hash={:?}, batch={:?}, plaintexts_h={:?}, signer_t={:?} }}",
                dbg_hash(&cfg_h.0), batch, plaintexts_h, signer_t
            ),
        }
    }
}
use crate::util::dbg_hashes;
impl std::fmt::Debug for ChannelsHashes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hashes={:?}", dbg_hashes(&self.0))
    }
}
