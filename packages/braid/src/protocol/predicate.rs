// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use log::trace;
use std::collections::HashSet;
use strum::Display;

use strand::context::Ctx;
use strand::signature::StrandSignaturePk;

use board_messages::braid::artifact::Configuration;
use board_messages::braid::newtypes::*;
use board_messages::braid::statement::Statement;

use crate::util::ProtocolError;

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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Display, Debug)]
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
    ) -> Result<Predicate, ProtocolError> {
        let ret = match statement {
            // Only called for configuration signatures, configuration
            // bootstrap is done through Predicate::get_bootstrap_predicate
            // Configuration(Timestamp, ConfigurationH)
            Statement::Configuration(_, _) => panic!("impossible"),
            Statement::ConfigurationSigned(_ts, cfg_h) => Ok(Self::ConfigurationSigned(
                ConfigurationHash(cfg_h.0),
                signer_position,
            )),
            // Commitments(Timestamp, ConfigurationH, CommitmentsH)
            Statement::Channel(_ts, cfg_h, cm_h) => Ok(Self::Channel(
                ConfigurationHash(cfg_h.0),
                ChannelHash(cm_h.0),
                signer_position,
            )),
            // CommitmentsAllSigned(Timestamp, ConfigurationH, CommitmentsHs)
            Statement::ChannelsAllSigned(_ts, cfg_h, cm_hs) => Ok(Self::ChannelsSigned(
                ConfigurationHash(cfg_h.0),
                ChannelsHashes(cm_hs.0),
                signer_position,
            )),
            // Shares(Timestamp, ConfigurationH, SharesH)
            Statement::Shares(_ts, cfg_h, sh_h) => Ok(Self::Shares(
                ConfigurationHash(cfg_h.0),
                SharesHash(sh_h.0),
                signer_position,
            )),
            // PublicKey(Timestamp, ConfigurationH, PublicKeyH, SharesHs, CommitmentsHs)
            Statement::PublicKey(_ts, cfg_h, pk_h, sh_hs, cm_hs) => Ok(Self::PublicKey(
                ConfigurationHash(cfg_h.0),
                PublicKeyHash(pk_h.0),
                SharesHashes(sh_hs.0),
                ChannelsHashes(cm_hs.0),
                signer_position,
            )),
            // PublicKeySigned(Timestamp, ConfigurationH, PublicKeyH, SharesHs, CommitmentsHs)
            Statement::PublicKeySigned(_ts, cfg_h, pk_h, sh_hs, cm_hs) => {
                Ok(Self::PublicKeySigned(
                    ConfigurationHash(cfg_h.0),
                    PublicKeyHash(pk_h.0),
                    SharesHashes(sh_hs.0),
                    ChannelsHashes(cm_hs.0),
                    signer_position,
                ))
            }
            // Ballots(Timestamp, ConfigurationH, usize, CiphertextsH, PublicKeyH, TrusteeSet)
            Statement::Ballots(_ts, cfg_h, batch, ballots_h, pk_h, trustees) => {
                // Verify that all selected trustees are valid
                let mut selected = vec![];
                trustees.iter().for_each(|s| {
                    if *s != NULL_TRUSTEE {
                        assert!(*s > 0 && *s <= cfg.trustees.len());
                        selected.push(*s);
                    }
                });

                // Verify that all selected trustees are unique
                let unique: HashSet<usize> = selected.into_iter().collect();
                if unique.len() != cfg.threshold {
                    return Err(ProtocolError::InvalidTrusteeSelection(format!(
                        "Selected trustees should be equal to the threshold. Selected {} but required {}",
                        unique.len(),
                        cfg.threshold
                    )));
                }

                Ok(Self::Ballots(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    CiphertextsHash(ballots_h.0),
                    PublicKeyHash(pk_h.0),
                    *trustees,
                ))
            }
            // Mix(Timestamp, ConfigurationH, usize, CiphertextsH, CiphertextsH)
            Statement::Mix(_ts, cfg_h, batch, source_h, mix_h, mix_number) => Ok(Self::Mix(
                ConfigurationHash(cfg_h.0),
                *batch,
                CiphertextsHash(source_h.0),
                CiphertextsHash(mix_h.0),
                *mix_number,
                signer_position,
            )),
            // MixSigned(Timestamp, ConfigurationH, usize, usize, CiphertextsH, CiphertextsH)
            Statement::MixSigned(_ts, cfg_h, batch, _mix_no, source_h, mix_h) => {
                Ok(Self::MixSigned(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    CiphertextsHash(source_h.0),
                    CiphertextsHash(mix_h.0),
                    signer_position,
                ))
            }
            // DecryptionFactors(Timestamp, ConfigurationH, usize, DecryptionFactorsH, CiphertextsH, SharesHs)
            Statement::DecryptionFactors(_ts, cfg_h, batch, df_h, mix_h, sh_hs) => {
                Ok(Self::DecryptionFactors(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    DecryptionFactorsHash(df_h.0),
                    CiphertextsHash(mix_h.0),
                    SharesHashes(sh_hs.0),
                    signer_position,
                ))
            }
            // Plaintexts(Timestamp, ConfigurationH, usize, PlaintextsH, DecryptionFactorsHs, PublicKeyH)
            Statement::Plaintexts(_ts, cfg_h, batch, pl_h, df_hs, c_h, pk_h) => {
                Ok(Self::Plaintexts(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    PlaintextsHash(pl_h.0),
                    DecryptionFactorsHashes(df_hs.0),
                    CiphertextsHash(c_h.0),
                    PublicKeyHash(pk_h.0),
                    signer_position,
                ))
            }
            // PlaintextsSigned(Timestamp, ConfigurationH, usize, PlaintextsH, DecryptionFactorsHs, PublicKeyH)
            Statement::PlaintextsSigned(_ts, cfg_h, batch, pl_h, df_hs, c_h, pk_h) => {
                Ok(Self::PlaintextsSigned(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    PlaintextsHash(pl_h.0),
                    DecryptionFactorsHashes(df_hs.0),
                    CiphertextsHash(c_h.0),
                    PublicKeyHash(pk_h.0),
                    signer_position,
                ))
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
            VERIFIER_INDEX,
            configuration.trustees.len(),
            configuration.threshold,
        );

        Some(p)
    }
}
