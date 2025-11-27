// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use log::trace;
use std::collections::HashSet;
use strum::Display;

use strand::context::Ctx;
use strand::signature::StrandSignaturePk;

use b3::messages::artifact::Configuration;
use b3::messages::newtypes::*;
use b3::messages::statement::Statement;

use crate::util::ProtocolError;

///////////////////////////////////////////////////////////////////////////
// Predicate
///////////////////////////////////////////////////////////////////////////

/// A datalog predicate that represents an assertion about the protocol state.
///
/// Predicates are enum variants passed as inputs to datalog
/// Phases for inference, and outputted by datalog to forward to
/// subsequent Phases.
/// They contain newtype tuple members, except for members
/// that require doing arithmetic which are type aliases.
/// Predicates are derived from Statements, using the general
/// Predicate::from_statement method.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Display, Debug)]
pub(crate) enum Predicate {
    // Input predicates
    // Bootstrap //////////////////////////////////////////////////////////////////
    Configuration(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold),
    ConfigurationSigned(ConfigurationHash, TrusteePosition),

    // Dkg ////////////////////////////////////////////////////////////////////////
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

    // Shuffle ////////////////////////////////////////////////////////////////////
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
    /// Returns the predicate asserted by the given statement.
    pub(crate) fn from_statement<C: Ctx>(
        statement: &Statement,
        signer_position: TrusteePosition,
        cfg: &Configuration<C>,
    ) -> Result<Predicate, ProtocolError> {
        let ret = match statement {
            // Only called for configuration signatures, configuration
            // bootstrap is done through Predicate::get_bootstrap_predicate
            Statement::Configuration(_, _) => panic!("impossible"),
            // variant: ConfigurationSigned(Timestamp, ConfigurationH)
            Statement::ConfigurationSigned(_ts, cfg_h) => Ok(Self::ConfigurationSigned(
                ConfigurationHash(cfg_h.0),
                signer_position,
            )),
            // variant: Commitments(Timestamp, ConfigurationH, CommitmentsH)
            Statement::Channel(_ts, cfg_h, cm_h) => Ok(Self::Channel(
                ConfigurationHash(cfg_h.0),
                ChannelHash(cm_h.0),
                signer_position,
            )),
            // variant: CommitmentsAllSigned(Timestamp, ConfigurationH, CommitmentsHs)
            Statement::ChannelsAllSigned(_ts, cfg_h, cm_hs) => Ok(Self::ChannelsSigned(
                ConfigurationHash(cfg_h.0),
                ChannelsHashes(cm_hs.0),
                signer_position,
            )),
            // variant: Shares(Timestamp, ConfigurationH, SharesH)
            Statement::Shares(_ts, cfg_h, sh_h) => Ok(Self::Shares(
                ConfigurationHash(cfg_h.0),
                SharesHash(sh_h.0),
                signer_position,
            )),
            // variant: PublicKey(Timestamp, ConfigurationH, PublicKeyH, SharesHs, CommitmentsHs)
            Statement::PublicKey(_ts, cfg_h, pk_h, sh_hs, cm_hs) => Ok(Self::PublicKey(
                ConfigurationHash(cfg_h.0),
                PublicKeyHash(pk_h.0),
                SharesHashes(sh_hs.0),
                ChannelsHashes(cm_hs.0),
                signer_position,
            )),
            // variant: PublicKeySigned(Timestamp, ConfigurationH, PublicKeyH, SharesHs, CommitmentsHs)
            Statement::PublicKeySigned(_ts, cfg_h, pk_h, sh_hs, cm_hs) => {
                Ok(Self::PublicKeySigned(
                    ConfigurationHash(cfg_h.0),
                    PublicKeyHash(pk_h.0),
                    SharesHashes(sh_hs.0),
                    ChannelsHashes(cm_hs.0),
                    signer_position,
                ))
            }
            // variant: Ballots(Timestamp, ConfigurationH, usize, CiphertextsH, PublicKeyH, TrusteeSet)
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
            // variant: Mix(Timestamp, ConfigurationH, usize, CiphertextsH, CiphertextsH, usize)
            Statement::Mix(_ts, cfg_h, batch, source_h, mix_h, mix_number) => Ok(Self::Mix(
                ConfigurationHash(cfg_h.0),
                *batch,
                CiphertextsHash(source_h.0),
                CiphertextsHash(mix_h.0),
                *mix_number,
                signer_position,
            )),
            // variant: MixSigned(Timestamp, ConfigurationH, usize, usize, CiphertextsH, CiphertextsH)
            Statement::MixSigned(_ts, cfg_h, batch, _mix_no, source_h, mix_h) => {
                Ok(Self::MixSigned(
                    ConfigurationHash(cfg_h.0),
                    *batch,
                    CiphertextsHash(source_h.0),
                    CiphertextsHash(mix_h.0),
                    signer_position,
                ))
            }
            // variant: DecryptionFactors(Timestamp, ConfigurationH, usize, DecryptionFactorsH, CiphertextsH, SharesHs)
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
            // variant: Plaintexts(Timestamp, ConfigurationH, usize, PlaintextsH, DecryptionFactorsHs, PublicKeyH)
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
            // variant: PlaintextsSigned(Timestamp, ConfigurationH, usize, PlaintextsH, DecryptionFactorsHs, PublicKeyH)
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

    /// Returns the predicate corresponding to a configuration artifact.
    ///
    /// Special case for bootstrapping. Other statements are obtained
    /// with Predicate::from_statement.
    pub(crate) fn get_bootstrap_predicate<C: Ctx>(
        configuration: &Configuration<C>,
        trustee_pk: &StrandSignaturePk,
    ) -> Result<Predicate, ProtocolError> {
        let index = configuration.get_trustee_position(trustee_pk).ok_or(
            ProtocolError::InvalidConfiguration(
                "Could not find trustee position in configuration".to_string(),
            ),
        )?;

        assert!(index != PROTOCOL_MANAGER_INDEX);

        let cfg = ConfigurationHash::from_configuration(configuration).map_err(|e| {
            ProtocolError::InvalidConfiguration(format!("Could not read configuration {}", e))
        })?;

        Ok(Predicate::Configuration(
            cfg,
            index,
            configuration.trustees.len(),
            configuration.threshold,
        ))
    }

    /// Returns the predicate corresponding to a configuration artifact.
    ///
    /// Special case for bootstrapping in verifier mode. In verifying mode
    /// this predicate will identify this trustee as a strict observer, only
    /// deriving verification Actions.
    pub(crate) fn get_verifier_bootstrap_predicate<C: Ctx>(
        configuration: &Configuration<C>,
    ) -> Result<Predicate, ProtocolError> {
        let cfg = ConfigurationHash::from_configuration(configuration).map_err(|e| {
            ProtocolError::InvalidConfiguration(format!("Could not read configuration {}", e))
        })?;

        Ok(Predicate::Configuration(
            cfg,
            VERIFIER_INDEX,
            configuration.trustees.len(),
            configuration.threshold,
        ))
    }
}
