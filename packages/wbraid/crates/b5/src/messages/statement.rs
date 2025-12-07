// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{CryptographicHash, messages::newtypes::*};
use cryptography::utils::serialization::{VSerializable, VDeserializable};
use strum::Display;

///////////////////////////////////////////////////////////////////////////
// Statement
///////////////////////////////////////////////////////////////////////////

#[derive(Clone, Display, Debug)]
pub enum Statement {
    Configuration(Timestamp, ConfigurationHash),
    ConfigurationSigned(Timestamp, ConfigurationHash),
    Channel(Timestamp, ConfigurationHash, ChannelHash),
    ChannelsAllSigned(Timestamp, ConfigurationHash, ChannelsHashes),
    Shares(Timestamp, ConfigurationHash, SharesHash),
    PublicKey(
        Timestamp,
        ConfigurationHash,
        PublicKeyHash,
        SharesHashes,
        ChannelsHashes,
    ),
    PublicKeySigned(
        Timestamp,
        ConfigurationHash,
        PublicKeyHash,
        SharesHashes,
        ChannelsHashes,
    ),

    Ballots(
        Timestamp,
        ConfigurationHash,
        BatchNumber,
        CiphertextsHash,
        PublicKeyHash,
        // the trustees (1-based positions) to participate in mixing + decryption
        TrusteeSet,
    ),
    Mix(
        Timestamp,
        ConfigurationHash,
        BatchNumber,
        CiphertextsHash,
        CiphertextsHash,
        // the mix number (mix.mix_number in Mix artifact)
        MixNumber,
    ),
    // See also local::StatementEntryIdentifier::mix_number
    MixSigned(
        Timestamp,
        ConfigurationHash,
        BatchNumber,
        MixNumber,
        CiphertextsHash,
        CiphertextsHash,
    ),
    DecryptionFactors(
        Timestamp,
        ConfigurationHash,
        BatchNumber,
        DecryptionFactorsHash,
        CiphertextsHash,
        SharesHashes,
    ),
    Plaintexts(
        Timestamp,
        ConfigurationHash,
        BatchNumber,
        PlaintextsHash,
        DecryptionFactorsHashes,
        CiphertextsHash,
        PublicKeyHash,
    ),
    PlaintextsSigned(
        Timestamp,
        ConfigurationHash,
        BatchNumber,
        PlaintextsHash,
        DecryptionFactorsHashes,
        CiphertextsHash,
        PublicKeyHash,
    ),
}

impl Statement {
    ///////////////////////////////////////////////////////////////////////////
    // Statement creation functions
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn configuration_stmt(cfg_hash: ConfigurationHash) -> Statement {
        Statement::Configuration(Self::timestamp(), cfg_hash)
    }

    pub(crate) fn configuration_signed_stmt(cfg_hash: ConfigurationHash) -> Statement {
        Statement::ConfigurationSigned(Self::timestamp(), cfg_hash)
    }

    pub(crate) fn channel_stmt(cfg_hash: ConfigurationHash, channel_h: ChannelHash) -> Statement {
        Statement::Channel(Self::timestamp(), cfg_hash, channel_h)
    }

    pub(crate) fn channels_all_stmt(
        cfg_hash: ConfigurationHash,
        channels_hs: ChannelsHashes,
    ) -> Statement {
        Statement::ChannelsAllSigned(Self::timestamp(), cfg_hash, channels_hs)
    }

    pub(crate) fn shares_stmt(cfg_hash: ConfigurationHash, shares_h: SharesHash) -> Statement {
        Statement::Shares(Self::timestamp(), cfg_hash, shares_h)
    }

    pub(crate) fn pk_stmt(
        cfg_hash: ConfigurationHash,
        pk_h: PublicKeyHash,
        shares_hs: SharesHashes,
        commitments_hs: ChannelsHashes,
    ) -> Statement {
        Statement::PublicKey(Self::timestamp(), cfg_hash, pk_h, shares_hs, commitments_hs)
    }

    pub(crate) fn pk_signed_stmt(
        cfg_hash: ConfigurationHash,
        pk_h: PublicKeyHash,
        shares_hs: SharesHashes,
        commitments_hs: ChannelsHashes,
    ) -> Statement {
        Statement::PublicKeySigned(Self::timestamp(), cfg_hash, pk_h, shares_hs, commitments_hs)
    }

    // The trustees field indicates which trustees will participate in the mix and decryption.
    // There must be threshold # of them. Each trustee is a number starting at 1 up to the the number of eligible
    // trustees as per the configuration. 0 is not a valid trustee. Remaining
    // slots of this fixed size array must be padded with newtypes::NULL_TRUSTEE
    pub(crate) fn ballots_stmt(
        cfg_hash: ConfigurationHash,
        ballots_h: CiphertextsHash,
        pk_h: PublicKeyHash,
        batch: BatchNumber,
        trustees: [usize; crate::messages::newtypes::MAX_TRUSTEES],
    ) -> Statement {
        Statement::Ballots(
            Self::timestamp(),
            cfg_hash,
            batch,
            ballots_h,
            pk_h,
            trustees,
        )
    }

    pub(crate) fn mix_stmt(
        cfg_hash: ConfigurationHash,
        // Points to either Ballots or Mix
        source_ciphertexts_h: CiphertextsHash,
        mix_h: CiphertextsHash,
        batch: BatchNumber,
        mix_number: MixNumber,
    ) -> Statement {
        Statement::Mix(
            Self::timestamp(),
            cfg_hash,
            batch,
            source_ciphertexts_h,
            mix_h,
            mix_number,
        )
    }

    pub(crate) fn mix_signed_stmt(
        cfg_hash: ConfigurationHash,
        // Points to either Ballots or Mix
        source_ciphertexts_h: CiphertextsHash,
        mix_h: CiphertextsHash,
        batch: BatchNumber,
        mix_number: MixNumber,
    ) -> Statement {
        Statement::MixSigned(
            Self::timestamp(),
            cfg_hash,
            batch,
            mix_number,
            source_ciphertexts_h,
            mix_h,
        )
    }

    pub(crate) fn decryption_factors_stmt(
        cfg_hash: ConfigurationHash,
        batch: BatchNumber,
        dfactors_h: DecryptionFactorsHash,
        mix_h: CiphertextsHash,
        shares_hs: SharesHashes,
    ) -> Statement {
        Statement::DecryptionFactors(
            Self::timestamp(),
            cfg_hash,
            batch,
            dfactors_h,
            mix_h,
            shares_hs,
        )
    }

    pub(crate) fn plaintexts_stmt(
        cfg_hash: ConfigurationHash,
        batch: BatchNumber,
        plaintexts_h: PlaintextsHash,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
    ) -> Statement {
        Statement::Plaintexts(
            Self::timestamp(),
            cfg_hash,
            batch,
            plaintexts_h,
            dfactors_hs,
            cipher_h,
            pk_h,
        )
    }

    pub(crate) fn plaintexts_signed_stmt(
        cfg_hash: ConfigurationHash,
        batch: BatchNumber,
        plaintexts_h: PlaintextsHash,
        dfactors_hs: DecryptionFactorsHashes,
        cipher_h: CiphertextsHash,
        pk_h: PublicKeyHash,
    ) -> Statement {
        Statement::PlaintextsSigned(
            Self::timestamp(),
            cfg_hash,
            batch,
            plaintexts_h,
            dfactors_hs,
            cipher_h,
            pk_h,
        )
    }

    fn timestamp() -> Timestamp {
        crate::timestamp()
    }

    ///////////////////////////////////////////////////////////////////////////
    // Data accessors
    ///////////////////////////////////////////////////////////////////////////

    pub fn get_kind(&self) -> StatementType {
        self.get_data().0
    }

    pub fn get_cfg_h(&self) -> Hash {
        self.get_data().1
    }

    pub fn get_batch_number(&self) -> BatchNumber {
        self.get_data().2
    }

    pub fn get_mix_number(&self) -> MixNumber {
        self.get_data().3
    }

    pub fn get_timestamp(&self) -> Timestamp {
        self.get_data().4
    }

    pub fn get_data(&self) -> (StatementType, Hash, BatchNumber, MixNumber, Timestamp) {
        let kind: StatementType;
        let ts: u64;
        let cfg: CryptographicHash;
        let mut batch = 0;
        let mut mix_number = 0;

        match self {
            Self::Configuration(ts_, cfg_h) => {
                ts = *ts_;
                kind = StatementType::Configuration;
                cfg = cfg_h.0;
            }
            Self::ConfigurationSigned(ts_, cfg_h) => {
                ts = *ts_;
                kind = StatementType::ConfigurationSigned;
                cfg = cfg_h.0;
            }
            Self::Channel(ts_, cfg_h, _) => {
                ts = *ts_;
                kind = StatementType::Channel;
                cfg = cfg_h.0;
            }
            Self::ChannelsAllSigned(ts_, cfg_h, _) => {
                ts = *ts_;
                kind = StatementType::ChannelsAllSigned;
                cfg = cfg_h.0;
            }
            Self::Shares(ts_, cfg_h, _) => {
                ts = *ts_;
                kind = StatementType::Shares;
                cfg = cfg_h.0;
            }
            Self::PublicKey(ts_, cfg_h, _, _, _) => {
                ts = *ts_;
                kind = StatementType::PublicKey;
                cfg = cfg_h.0;
            }
            Self::PublicKeySigned(ts_, cfg_h, _, _, _) => {
                ts = *ts_;
                kind = StatementType::PublicKeySigned;
                cfg = cfg_h.0;
            }
            Self::Ballots(ts_, cfg_h, bch, _, _, _) => {
                ts = *ts_;
                kind = StatementType::Ballots;
                cfg = cfg_h.0;
                batch = bch.clone();
            }
            Self::Mix(ts_, cfg_h, bch, _, _, _) => {
                ts = *ts_;
                kind = StatementType::Mix;
                cfg = cfg_h.0;
                batch = bch.clone();
            }
            Self::MixSigned(ts_, cfg_h, bch, mix_no, _, _) => {
                ts = *ts_;
                kind = StatementType::MixSigned;
                cfg = cfg_h.0;
                batch = bch.clone();
                mix_number = mix_no.clone();
            }
            Self::DecryptionFactors(ts_, cfg_h, bch, _, _, _) => {
                ts = *ts_;
                kind = StatementType::DecryptionFactors;
                cfg = cfg_h.0;
                batch = bch.clone();
            }
            Self::Plaintexts(ts_, cfg_h, bch, _, _, _, _) => {
                ts = *ts_;
                kind = StatementType::Plaintexts;
                cfg = cfg_h.0;
                batch = bch.clone();
            }
            Self::PlaintextsSigned(ts_, cfg_h, bch, _, _, _, _) => {
                ts = *ts_;
                kind = StatementType::PlaintextsSigned;
                cfg = cfg_h.0;
                batch = bch.clone();
            }
        }

        (kind, cfg, batch, mix_number, ts)
    }
}

impl VSerializable for Statement {
    fn ser(&self) -> Vec<u8> {
        match self {
            Statement::Configuration(ts, cfg) => (0u8, ts, cfg).ser(),
            Statement::ConfigurationSigned(ts, cfg) => (1u8, ts, cfg).ser(),
            Statement::Channel(ts, cfg, ch) => (2u8, ts, cfg, ch).ser(),
            Statement::ChannelsAllSigned(ts, cfg, chs) => (3u8, ts, cfg, chs).ser(),
            Statement::Shares(ts, cfg, sh) => (4u8, ts, cfg, sh).ser(),
            Statement::PublicKey(ts, cfg, pk, shs, chs) => (5u8, ts, cfg, pk, shs, chs).ser(),
            Statement::PublicKeySigned(ts, cfg, pk, shs, chs) => (6u8, ts, cfg, pk, shs, chs).ser(),
            Statement::Ballots(ts, cfg, bn, cth, pk, tset) => (7u8, ts, cfg, bn, cth, pk, tset).ser(),
            Statement::Mix(ts, cfg, bn, cth1, cth2, mn) => (8u8, ts, cfg, bn, cth1, cth2, mn).ser(),
            Statement::MixSigned(ts, cfg, bn, mn, cth1, cth2) => (9u8, ts, cfg, bn, mn, cth1, cth2).ser(),
            Statement::DecryptionFactors(ts, cfg, bn, dfh, cth, shs) => (10u8, ts, cfg, bn, dfh, cth, shs).ser(),
            Statement::Plaintexts(ts, cfg, bn, pth, dfhs, cth, pk) => (11u8, ts, cfg, bn, pth, dfhs, cth, pk).ser(),
            Statement::PlaintextsSigned(ts, cfg, bn, pth, dfhs, cth, pk) => (12u8, ts, cfg, bn, pth, dfhs, cth, pk).ser(),
        }
    }
}

impl VDeserializable for Statement {
    fn deser(buffer: &[u8]) -> Result<Self, cryptography::utils::error::Error> {
        let discriminant = u8::deser(&buffer[0..1])?;
        let rest = &buffer[1..];
        
        Ok(match discriminant {
            0 => {
                let (ts, cfg) = VDeserializable::deser(rest)?;
                Statement::Configuration(ts, cfg)
            },
            1 => {
                let (ts, cfg) = VDeserializable::deser(rest)?;
                Statement::ConfigurationSigned(ts, cfg)
            },
            2 => {
                let (ts, cfg, ch) = VDeserializable::deser(rest)?;
                Statement::Channel(ts, cfg, ch)
            },
            3 => {
                let (ts, cfg, chs) = VDeserializable::deser(rest)?;
                Statement::ChannelsAllSigned(ts, cfg, chs)
            },
            4 => {
                let (ts, cfg, sh) = VDeserializable::deser(rest)?;
                Statement::Shares(ts, cfg, sh)
            },
            5 => {
                let (ts, cfg, pk, shs, chs) = VDeserializable::deser(rest)?;
                Statement::PublicKey(ts, cfg, pk, shs, chs)
            },
            6 => {
                let (ts, cfg, pk, shs, chs) = VDeserializable::deser(rest)?;
                Statement::PublicKeySigned(ts, cfg, pk, shs, chs)
            },
            7 => {
                let (ts, cfg, bn, cth, pk, tset) = VDeserializable::deser(rest)?;
                Statement::Ballots(ts, cfg, bn, cth, pk, tset)
            },
            8 => {
                let (ts, cfg, bn, cth1, cth2, mn) = VDeserializable::deser(rest)?;
                Statement::Mix(ts, cfg, bn, cth1, cth2, mn)
            },
            9 => {
                let (ts, cfg, bn, mn, cth1, cth2) = VDeserializable::deser(rest)?;
                Statement::MixSigned(ts, cfg, bn, mn, cth1, cth2)
            },
            10 => {
                let (ts, cfg, bn, dfh, cth, shs) = VDeserializable::deser(rest)?;
                Statement::DecryptionFactors(ts, cfg, bn, dfh, cth, shs)
            },
            11 => {
                let (ts, cfg, bn, pth, dfhs, cth, pk) = VDeserializable::deser(rest)?;
                Statement::Plaintexts(ts, cfg, bn, pth, dfhs, cth, pk)
            },
            12 => {
                let (ts, cfg, bn, pth, dfhs, cth, pk) = VDeserializable::deser(rest)?;
                Statement::PlaintextsSigned(ts, cfg, bn, pth, dfhs, cth, pk)
            },
            _ => return Err(cryptography::utils::error::Error::DeserializationError(format!("Invalid Statement discriminant: {}", discriminant))),
        })
    }
}


///////////////////////////////////////////////////////////////////////////
// Enums necessary to store statements and artifacts in LocalBoard
///////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq, Display, Debug, core::hash::Hash)]
#[repr(u8)]
pub enum StatementType {
    Configuration = 0,
    ConfigurationSigned = 1,
    Channel = 2,
    ChannelsAllSigned = 3,
    Shares = 4,
    PublicKey = 5,
    PublicKeySigned = 6,
    Ballots = 7,
    Mix = 8,
    MixSigned = 9,
    DecryptionFactors = 10,
    Plaintexts = 11,
    PlaintextsSigned = 12,
}

impl VSerializable for StatementType {
    fn ser(&self) -> Vec<u8> {
        (*self as u8).ser()
    }
}

impl VDeserializable for StatementType {
    fn deser(buffer: &[u8]) -> Result<Self, cryptography::utils::error::Error> {
        let disc = u8::deser(buffer)?;
        match disc {
            0 => Ok(StatementType::Configuration),
            1 => Ok(StatementType::ConfigurationSigned),
            2 => Ok(StatementType::Channel),
            3 => Ok(StatementType::ChannelsAllSigned),
            4 => Ok(StatementType::Shares),
            5 => Ok(StatementType::PublicKey),
            6 => Ok(StatementType::PublicKeySigned),
            7 => Ok(StatementType::Ballots),
            8 => Ok(StatementType::Mix),
            9 => Ok(StatementType::MixSigned),
            10 => Ok(StatementType::DecryptionFactors),
            11 => Ok(StatementType::Plaintexts),
            12 => Ok(StatementType::PlaintextsSigned),
            _ => Err(cryptography::utils::error::Error::DeserializationError(format!("Invalid StatementType discriminant: {}", disc))),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use super::*;
    use cryptography::utils::serialization::{VDeserializable, VSerializable};

    #[test]
    fn test_serialize_channelshashes() {
        let hashes = [crate::messages::newtypes::zero_hash(); crate::messages::newtypes::MAX_TRUSTEES];
        let cs = ChannelsHashes(hashes);
        let bytes = cs.ser();

        let d_cs: ChannelsHashes = ChannelsHashes::deser(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }

    #[test]
    fn test_serialize_shareshashes() {
        let hashes = [crate::messages::newtypes::zero_hash(); crate::messages::newtypes::MAX_TRUSTEES];
        let cs = SharesHashes(hashes);
        let bytes = cs.ser();

        let d_cs: SharesHashes = SharesHashes::deser(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }

    #[test]
    fn test_serialize_decryptionfactorshs() {
        let hashes = [crate::messages::newtypes::zero_hash(); crate::messages::newtypes::MAX_TRUSTEES];
        let cs = DecryptionFactorsHashes(hashes);
        let bytes = cs.ser();

        let d_cs: DecryptionFactorsHashes =
            DecryptionFactorsHashes::deser(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }
}
