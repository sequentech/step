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
        let mut bytes = Vec::new();
        match self {
            Statement::Configuration(ts, cfg) => {
                bytes.extend(0u8.ser());
                bytes.extend((ts, cfg).ser());
            },
            Statement::ConfigurationSigned(ts, cfg) => {
                bytes.extend(1u8.ser());
                bytes.extend((ts, cfg).ser());
            },
            Statement::Channel(ts, cfg, ch) => {
                bytes.extend(2u8.ser());
                bytes.extend((ts, cfg, ch).ser());
            },
            Statement::ChannelsAllSigned(ts, cfg, chs) => {
                bytes.extend(3u8.ser());
                bytes.extend((ts, cfg, chs).ser());
            },
            Statement::Shares(ts, cfg, sh) => {
                bytes.extend(4u8.ser());
                bytes.extend((ts, cfg, sh).ser());
            },
            Statement::PublicKey(ts, cfg, pk, shs, chs) => {
                bytes.extend(5u8.ser());
                bytes.extend((ts, cfg, pk, shs, chs).ser());
            },
            Statement::PublicKeySigned(ts, cfg, pk, shs, chs) => {
                bytes.extend(6u8.ser());
                bytes.extend((ts, cfg, pk, shs, chs).ser());
            },
            Statement::Ballots(ts, cfg, bn, cth, pk, tset) => {
                bytes.extend(7u8.ser());
                bytes.extend((ts, cfg, bn, cth, pk, tset).ser());
            },
            Statement::Mix(ts, cfg, bn, cth1, cth2, mn) => {
                bytes.extend(8u8.ser());
                bytes.extend((ts, cfg, bn, cth1, cth2, mn).ser());
            },
            Statement::MixSigned(ts, cfg, bn, mn, cth1, cth2) => {
                bytes.extend(9u8.ser());
                bytes.extend((ts, cfg, bn, mn, cth1, cth2).ser());
            },
            Statement::DecryptionFactors(ts, cfg, bn, dfh, cth, shs) => {
                bytes.extend(10u8.ser());
                bytes.extend((ts, cfg, bn, dfh, cth, shs).ser());
            },
            Statement::Plaintexts(ts, cfg, bn, pth, dfhs, cth, pk) => {
                bytes.extend(11u8.ser());
                bytes.extend((ts, cfg, bn, pth, dfhs, cth, pk).ser());
            },
            Statement::PlaintextsSigned(ts, cfg, bn, pth, dfhs, cth, pk) => {
                bytes.extend(12u8.ser());
                bytes.extend((ts, cfg, bn, pth, dfhs, cth, pk).ser());
            },
        }
        bytes
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

    #[test]
    fn test_serialize_statement_configuration() {
        let cfg_hash = ConfigurationHash(crate::messages::newtypes::zero_hash());
        let stmt = Statement::Configuration(12345, cfg_hash);
        
        let bytes = stmt.ser();
        println!("Serialized bytes length: {}", bytes.len());
        println!("First 20 bytes: {:?}", &bytes[0..20.min(bytes.len())]);
        
        let deserialized = Statement::deser(&bytes).unwrap();
        
        match (stmt, deserialized) {
            (Statement::Configuration(ts1, cfg1), Statement::Configuration(ts2, cfg2)) => {
                assert_eq!(ts1, ts2);
                assert_eq!(cfg1.0, cfg2.0);
            }
            _ => panic!("Deserialized statement has wrong variant"),
        }
    }

    #[test]
    fn test_serialize_statement_publickey() {
        let cfg_hash = ConfigurationHash(crate::messages::newtypes::zero_hash());
        let pk_hash = PublicKeyHash(crate::messages::newtypes::zero_hash());
        let shares_hashes = SharesHashes([crate::messages::newtypes::zero_hash(); MAX_TRUSTEES]);
        let channels_hashes = ChannelsHashes([crate::messages::newtypes::zero_hash(); MAX_TRUSTEES]);
        
        let stmt = Statement::PublicKey(
            67890,
            cfg_hash,
            pk_hash,
            shares_hashes,
            channels_hashes,
        );
        
        let bytes = stmt.ser();
        let deserialized = Statement::deser(&bytes).unwrap();
        
        match (stmt, deserialized) {
            (
                Statement::PublicKey(ts1, cfg1, pk1, shs1, chs1),
                Statement::PublicKey(ts2, cfg2, pk2, shs2, chs2)
            ) => {
                assert_eq!(ts1, ts2);
                assert_eq!(cfg1.0, cfg2.0);
                assert_eq!(pk1.0, pk2.0);
                assert_eq!(shs1.0, shs2.0);
                assert_eq!(chs1.0, chs2.0);
            }
            _ => panic!("Deserialized statement has wrong variant"),
        }
    }

    #[test]
    fn test_serialize_statement_ballots() {
        let cfg_hash = ConfigurationHash(crate::messages::newtypes::zero_hash());
        let pk_hash = PublicKeyHash(crate::messages::newtypes::zero_hash());
        let cth = CiphertextsHash(crate::messages::newtypes::zero_hash());
        let trustee_set: TrusteeSet = [1usize; MAX_TRUSTEES];
        
        let stmt = Statement::Ballots(
            11111,
            cfg_hash,
            5,
            cth,
            pk_hash,
            trustee_set,
        );
        
        let bytes = stmt.ser();
        let deserialized = Statement::deser(&bytes).unwrap();
        
        match (stmt, deserialized) {
            (
                Statement::Ballots(ts1, cfg1, bn1, cth1, pk1, tset1),
                Statement::Ballots(ts2, cfg2, bn2, cth2, pk2, tset2)
            ) => {
                assert_eq!(ts1, ts2);
                assert_eq!(cfg1.0, cfg2.0);
                assert_eq!(bn1, bn2);
                assert_eq!(cth1.0, cth2.0);
                assert_eq!(pk1.0, pk2.0);
                assert_eq!(tset1, tset2);
            }
            _ => panic!("Deserialized statement has wrong variant"),
        }
    }

    #[test]
    fn test_serialize_statement_mix() {
        use sha3::digest::array::Array;
        
        let cfg_hash = ConfigurationHash(crate::messages::newtypes::zero_hash());
        let cth1 = CiphertextsHash(crate::messages::newtypes::zero_hash());
        let cth2 = CiphertextsHash(Array([1u8; 64]));
        
        let stmt = Statement::Mix(
            22222,
            cfg_hash,
            3,
            cth1,
            cth2,
            2,
        );
        
        let bytes = stmt.ser();
        let deserialized = Statement::deser(&bytes).unwrap();
        
        match (stmt, deserialized) {
            (
                Statement::Mix(ts1, cfg1, bn1, cth1a, cth2a, mn1),
                Statement::Mix(ts2, cfg2, bn2, cth1b, cth2b, mn2)
            ) => {
                assert_eq!(ts1, ts2);
                assert_eq!(cfg1.0, cfg2.0);
                assert_eq!(bn1, bn2);
                assert_eq!(cth1a.0, cth1b.0);
                assert_eq!(cth2a.0, cth2b.0);
                assert_eq!(mn1, mn2);
            }
            _ => panic!("Deserialized statement has wrong variant"),
        }
    }

    #[test]
    fn test_serialize_statement_plaintexts() {
        use sha3::digest::array::Array;
        
        let cfg_hash = ConfigurationHash(crate::messages::newtypes::zero_hash());
        let pk_hash = PublicKeyHash(crate::messages::newtypes::zero_hash());
        let pth = PlaintextsHash(Array([2u8; 64]));
        let dfhs = DecryptionFactorsHashes([crate::messages::newtypes::zero_hash(); MAX_TRUSTEES]);
        let cth = CiphertextsHash(Array([3u8; 64]));
        
        let stmt = Statement::Plaintexts(
            33333,
            cfg_hash,
            7,
            pth,
            dfhs,
            cth,
            pk_hash,
        );
        
        let bytes = stmt.ser();
        let deserialized = Statement::deser(&bytes).unwrap();
        
        match (stmt, deserialized) {
            (
                Statement::Plaintexts(ts1, cfg1, bn1, pth1, dfhs1, cth1, pk1),
                Statement::Plaintexts(ts2, cfg2, bn2, pth2, dfhs2, cth2, pk2)
            ) => {
                assert_eq!(ts1, ts2);
                assert_eq!(cfg1.0, cfg2.0);
                assert_eq!(bn1, bn2);
                assert_eq!(pth1.0, pth2.0);
                assert_eq!(dfhs1.0, dfhs2.0);
                assert_eq!(cth1.0, cth2.0);
                assert_eq!(pk1.0, pk2.0);
            }
            _ => panic!("Deserialized statement has wrong variant"),
        }
    }
}
