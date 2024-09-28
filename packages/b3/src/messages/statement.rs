// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::messages::newtypes::*;
use borsh::{BorshDeserialize, BorshSerialize};
use strand::hash::Hash;
use strum::Display;

///////////////////////////////////////////////////////////////////////////
// Statement
///////////////////////////////////////////////////////////////////////////

#[derive(BorshSerialize, BorshDeserialize, Clone, Display, Debug)]
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
        let cfg: [u8; 64];
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

///////////////////////////////////////////////////////////////////////////
// Enums necessary to store statements and artifacts in LocalBoard
///////////////////////////////////////////////////////////////////////////

#[derive(
    BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Display, Debug, core::hash::Hash,
)]
#[repr(u8)]
#[borsh(use_discriminant = true)]
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

///////////////////////////////////////////////////////////////////////////
// Manual serialization necessary as [u8; 64] does not implement Default
///////////////////////////////////////////////////////////////////////////

impl BorshSerialize for ChannelsHashes {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.iter().map(|t| borsh::to_vec(t)).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl BorshDeserialize for ChannelsHashes {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let vectors = <Vec<Vec<u8>>>::deserialize_reader(reader)?;

        let inner: std::io::Result<Vec<[u8; 64]>> = vectors
            .iter()
            .map(|v| <[u8; 64]>::try_from_slice(v))
            .collect();

        let mut ret = [[0u8; 64]; crate::messages::newtypes::MAX_TRUSTEES];
        ret.copy_from_slice(&inner?);

        Ok(ChannelsHashes(ret))
    }
}

impl BorshSerialize for SharesHashes {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.iter().map(|t| borsh::to_vec(t)).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl BorshDeserialize for SharesHashes {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let vectors = <Vec<Vec<u8>>>::deserialize_reader(reader)?;

        let inner: std::io::Result<Vec<[u8; 64]>> = vectors
            .iter()
            .map(|v| <[u8; 64]>::try_from_slice(v))
            .collect();

        let mut ret = [[0u8; 64]; crate::messages::newtypes::MAX_TRUSTEES];
        ret.copy_from_slice(&inner?);

        Ok(SharesHashes(ret))
    }
}

impl BorshSerialize for DecryptionFactorsHashes {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.iter().map(|t| borsh::to_vec(t)).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl BorshDeserialize for DecryptionFactorsHashes {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let vectors = <Vec<Vec<u8>>>::deserialize_reader(reader)?;

        let inner: std::io::Result<Vec<[u8; 64]>> = vectors
            .iter()
            .map(|v| <[u8; 64]>::try_from_slice(v))
            .collect();

        let mut ret = [[0u8; 64]; crate::messages::newtypes::MAX_TRUSTEES];
        ret.copy_from_slice(&inner?);

        Ok(DecryptionFactorsHashes(ret))
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use super::*;
    use strand::serialization::{StrandDeserialize, StrandSerialize};

    #[test]
    fn test_serialize_channelshashes() {
        let hashes = [[0u8; 64]; crate::messages::newtypes::MAX_TRUSTEES];
        let cs = ChannelsHashes(hashes);
        let bytes = cs.strand_serialize().unwrap();

        let d_cs: ChannelsHashes = ChannelsHashes::strand_deserialize(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }

    #[test]
    fn test_serialize_shareshashes() {
        let hashes = [[0u8; 64]; crate::messages::newtypes::MAX_TRUSTEES];
        let cs = SharesHashes(hashes);
        let bytes = cs.strand_serialize().unwrap();

        let d_cs: SharesHashes = SharesHashes::strand_deserialize(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }

    #[test]
    fn test_serialize_decryptionfactorshs() {
        let hashes = [[0u8; 64]; crate::messages::newtypes::MAX_TRUSTEES];
        let cs = DecryptionFactorsHashes(hashes);
        let bytes = cs.strand_serialize().unwrap();

        let d_cs: DecryptionFactorsHashes =
            DecryptionFactorsHashes::strand_deserialize(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }
}
