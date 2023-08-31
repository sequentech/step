use borsh::{BorshDeserialize, BorshSerialize};
use strum::Display;

type Timestamp = u64;
use crate::protocol2::Hash;
pub type THashes = [Hash; crate::protocol2::MAX_TRUSTEES];

use crate::protocol2::predicate::MixNumber;
use crate::protocol2::predicate::TrusteePosition;

///////////////////////////////////////////////////////////////////////////
// Statement
///////////////////////////////////////////////////////////////////////////

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum Statement {
    Configuration(Timestamp, ConfigurationH),
    ConfigurationSigned(Timestamp, ConfigurationH),
    Commitments(Timestamp, ConfigurationH, CommitmentsH),
    CommitmentsAllSigned(Timestamp, ConfigurationH, CommitmentsHs),
    Shares(Timestamp, ConfigurationH, SharesH),
    PublicKey(
        Timestamp,
        ConfigurationH,
        PublicKeyH,
        SharesHs,
        CommitmentsHs,
    ),
    PublicKeySigned(
        Timestamp,
        ConfigurationH,
        PublicKeyH,
        SharesHs,
        CommitmentsHs,
    ),

    Ballots(
        Timestamp,
        ConfigurationH,
        Batch,
        CiphertextsH,
        PublicKeyH,
        // the trustees to participate in mixing + decryption (ballots.trustees in Ballots artifact)
        [usize; crate::protocol2::MAX_TRUSTEES],
    ),
    Mix(
        Timestamp,
        ConfigurationH,
        Batch,
        CiphertextsH,
        CiphertextsH,
        // the mix number (mix.mix_number in Mix artifact)
        MixNumber,
        // the next mixing trustee (mix.target_trustee in Mix artifact)
        TrusteePosition,
    ),
    // See also local::StatementEntryIdentifier::mix_number
    MixSigned(
        Timestamp,
        ConfigurationH,
        Batch,
        MixNumber,
        CiphertextsH,
        CiphertextsH,
    ),
    DecryptionFactors(
        Timestamp,
        ConfigurationH,
        Batch,
        DecryptionFactorsH,
        CiphertextsH,
        SharesHs,
    ),
    Plaintexts(
        Timestamp,
        ConfigurationH,
        Batch,
        PlaintextsH,
        DecryptionFactorsHs,
        CiphertextsH,
    ),
    PlaintextsSigned(
        Timestamp,
        ConfigurationH,
        Batch,
        PlaintextsH,
        DecryptionFactorsHs,
        CiphertextsH,
    ),
}

impl Statement {
    ///////////////////////////////////////////////////////////////////////////
    // Statement creation functions
    ///////////////////////////////////////////////////////////////////////////

    pub(crate) fn configuration_stmt(cfg_hash: ConfigurationH) -> Statement {
        Statement::Configuration(Self::timestamp(), cfg_hash)
    }

    pub(crate) fn configuration_signed_stmt(cfg_hash: ConfigurationH) -> Statement {
        Statement::ConfigurationSigned(Self::timestamp(), cfg_hash)
    }

    pub(crate) fn commitments_stmt(
        cfg_hash: ConfigurationH,
        commitments_h: CommitmentsH,
    ) -> Statement {
        Statement::Commitments(Self::timestamp(), cfg_hash, commitments_h)
    }

    pub(crate) fn commitments_all_stmt(
        cfg_hash: ConfigurationH,
        commitments_hs: CommitmentsHs,
    ) -> Statement {
        Statement::CommitmentsAllSigned(Self::timestamp(), cfg_hash, commitments_hs)
    }

    pub(crate) fn shares_stmt(cfg_hash: ConfigurationH, shares_h: SharesH) -> Statement {
        Statement::Shares(Self::timestamp(), cfg_hash, shares_h)
    }

    pub(crate) fn pk_stmt(
        cfg_hash: ConfigurationH,
        pk_h: PublicKeyH,
        shares_hs: SharesHs,
        commitments_hs: CommitmentsHs,
    ) -> Statement {
        Statement::PublicKey(Self::timestamp(), cfg_hash, pk_h, shares_hs, commitments_hs)
    }

    pub(crate) fn pk_signed_stmt(
        cfg_hash: ConfigurationH,
        pk_h: PublicKeyH,
        shares_hs: SharesHs,
        commitments_hs: CommitmentsHs,
    ) -> Statement {
        Statement::PublicKeySigned(Self::timestamp(), cfg_hash, pk_h, shares_hs, commitments_hs)
    }

    pub(crate) fn ballots_stmt(
        cfg_hash: ConfigurationH,
        ballots_h: CiphertextsH,
        pk_h: PublicKeyH,
        batch: Batch,
        trustees: [usize; crate::protocol2::MAX_TRUSTEES],
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
        cfg_hash: ConfigurationH,
        // Points to either Ballots or Mix
        source_ciphertexts_h: CiphertextsH,
        mix_h: CiphertextsH,
        batch: Batch,
        mix_number: MixNumber,
        target_trustee: usize,
    ) -> Statement {
        Statement::Mix(
            Self::timestamp(),
            cfg_hash,
            batch,
            source_ciphertexts_h,
            mix_h,
            mix_number,
            target_trustee,
        )
    }

    pub(crate) fn mix_signed_stmt(
        cfg_hash: ConfigurationH,
        // Points to either Ballots or Mix
        source_ciphertexts_h: CiphertextsH,
        mix_h: CiphertextsH,
        batch: Batch,
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
        cfg_hash: ConfigurationH,
        batch: Batch,
        dfactors_h: DecryptionFactorsH,
        mix_h: CiphertextsH,
        shares_hs: SharesHs,
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
        cfg_hash: ConfigurationH,
        batch: Batch,
        plaintexts_h: PlaintextsH,
        dfactors_hs: DecryptionFactorsHs,
        cipher_h: CiphertextsH,
    ) -> Statement {
        Statement::Plaintexts(
            Self::timestamp(),
            cfg_hash,
            batch,
            plaintexts_h,
            dfactors_hs,
            cipher_h,
        )
    }

    pub(crate) fn plaintexts_signed_stmt(
        cfg_hash: ConfigurationH,
        batch: Batch,
        plaintexts_h: PlaintextsH,
        dfactors_hs: DecryptionFactorsHs,
        cipher_h: CiphertextsH,
    ) -> Statement {
        Statement::PlaintextsSigned(
            Self::timestamp(),
            cfg_hash,
            batch,
            plaintexts_h,
            dfactors_hs,
            cipher_h,
        )
    }

    fn timestamp() -> Timestamp {
        instant::now() as u64
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

    pub fn get_timestamp(&self) -> Timestamp {
        self.get_data().5
    }

    pub fn get_data(
        &self,
    ) -> (
        StatementType,
        Hash,
        usize,
        usize,
        Option<ArtifactType>,
        Timestamp,
    ) {
        let kind: StatementType;
        let ts: u64;
        let cfg: [u8; 64];
        let mut batch = 0usize;
        let mut mix_number = 0usize;
        let mut artifact_type = None;

        match self {
            Self::Configuration(ts_, cfg_h) => {
                ts = *ts_;
                kind = StatementType::Configuration;
                cfg = cfg_h.0;
                artifact_type = Some(ArtifactType::Configuration);
            }
            Self::ConfigurationSigned(ts_, cfg_h) => {
                ts = *ts_;
                kind = StatementType::ConfigurationSigned;
                cfg = cfg_h.0;
            }
            Self::Commitments(ts_, cfg_h, _) => {
                ts = *ts_;
                kind = StatementType::Commitments;
                cfg = cfg_h.0;
                artifact_type = Some(ArtifactType::Commitments);
            }
            Self::CommitmentsAllSigned(ts_, cfg_h, _) => {
                ts = *ts_;
                kind = StatementType::CommitmentsAllSigned;
                cfg = cfg_h.0;
            }
            Self::Shares(ts_, cfg_h, _) => {
                ts = *ts_;
                kind = StatementType::Shares;
                cfg = cfg_h.0;
                artifact_type = Some(ArtifactType::Shares);
            }
            Self::PublicKey(ts_, cfg_h, _, _, _) => {
                ts = *ts_;
                kind = StatementType::PublicKey;
                cfg = cfg_h.0;
                artifact_type = Some(ArtifactType::PublicKey);
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
                batch = bch.0;
                artifact_type = Some(ArtifactType::Ballots);
            }
            Self::Mix(ts_, cfg_h, bch, _, _, _, _) => {
                ts = *ts_;
                kind = StatementType::Mix;
                cfg = cfg_h.0;
                batch = bch.0;
                artifact_type = Some(ArtifactType::Mix);
            }
            Self::MixSigned(ts_, cfg_h, bch, mix_no, _, _) => {
                ts = *ts_;
                kind = StatementType::MixSigned;
                cfg = cfg_h.0;
                batch = bch.0;
                mix_number = *mix_no;
            }
            Self::DecryptionFactors(ts_, cfg_h, bch, _, _, _) => {
                ts = *ts_;
                kind = StatementType::DecryptionFactors;
                cfg = cfg_h.0;
                batch = bch.0;
                artifact_type = Some(ArtifactType::DecryptionFactors);
            }
            Self::Plaintexts(ts_, cfg_h, bch, _, _, _) => {
                ts = *ts_;
                kind = StatementType::Plaintexts;
                cfg = cfg_h.0;
                batch = bch.0;
                artifact_type = Some(ArtifactType::Plaintexts);
            }
            Self::PlaintextsSigned(ts_, cfg_h, bch, _, _, _) => {
                ts = *ts_;
                kind = StatementType::PlaintextsSigned;
                cfg = cfg_h.0;
                batch = bch.0;
            }
        }

        (kind, cfg, batch, mix_number, artifact_type, ts)
    }
}

///////////////////////////////////////////////////////////////////////////
// Newtypes
///////////////////////////////////////////////////////////////////////////

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct ConfigurationH(pub Hash);
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CommitmentsH(pub Hash);
#[derive(Clone)]
pub struct CommitmentsHs(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SharesH(pub Hash);
#[derive(Clone)]
pub struct SharesHs(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct PublicKeyH(pub Hash);
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CiphertextsH(pub Hash);
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct DecryptionFactorsH(pub Hash);
#[derive(Clone)]
pub struct DecryptionFactorsHs(pub THashes);
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct PlaintextsH(pub Hash);

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Batch(pub usize);

///////////////////////////////////////////////////////////////////////////
// Enums necessary to store statements and artifacts in LocalBoard
///////////////////////////////////////////////////////////////////////////

#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Display, Debug, Hash)]
#[repr(u8)]
pub enum StatementType {
    Configuration = 0,
    ConfigurationSigned = 1,
    Commitments = 2,
    CommitmentsAllSigned = 3,
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

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Display)]
pub enum ArtifactType {
    Configuration,
    Commitments,
    Shares,
    PublicKey,
    Ballots,
    Mix,
    DecryptionFactors,
    Plaintexts,
}

///////////////////////////////////////////////////////////////////////////
// Manual serialization necessary as [u8; 64] does not implement Default
///////////////////////////////////////////////////////////////////////////

impl BorshSerialize for CommitmentsHs {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.iter().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl BorshDeserialize for CommitmentsHs {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let inner: std::io::Result<Vec<[u8; 64]>> = vectors
            .iter()
            .map(|v| <[u8; 64]>::try_from_slice(v))
            .collect();

        let mut ret = [[0u8; 64]; crate::protocol2::MAX_TRUSTEES];
        ret.copy_from_slice(&inner?);

        Ok(CommitmentsHs(ret))
    }
}

impl BorshSerialize for SharesHs {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.iter().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl BorshDeserialize for SharesHs {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let inner: std::io::Result<Vec<[u8; 64]>> = vectors
            .iter()
            .map(|v| <[u8; 64]>::try_from_slice(v))
            .collect();

        let mut ret = [[0u8; 64]; crate::protocol2::MAX_TRUSTEES];
        ret.copy_from_slice(&inner?);

        Ok(SharesHs(ret))
    }
}

impl BorshSerialize for DecryptionFactorsHs {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.iter().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl BorshDeserialize for DecryptionFactorsHs {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let inner: std::io::Result<Vec<[u8; 64]>> = vectors
            .iter()
            .map(|v| <[u8; 64]>::try_from_slice(v))
            .collect();

        let mut ret = [[0u8; 64]; crate::protocol2::MAX_TRUSTEES];
        ret.copy_from_slice(&inner?);

        Ok(DecryptionFactorsHs(ret))
    }
}

impl std::fmt::Debug for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Statement{{ type={:?} }}", self.get_kind(),)
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use super::*;
    use strand::serialization::{StrandDeserialize, StrandSerialize};

    #[test]
    fn test_serialize_commitmentshs() {
        let hashes = [[0u8; 64]; crate::protocol2::MAX_TRUSTEES];
        let cs = CommitmentsHs(hashes);
        let bytes = cs.strand_serialize().unwrap();

        let d_cs: CommitmentsHs = CommitmentsHs::strand_deserialize(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }

    #[test]
    fn test_serialize_shareshs() {
        let hashes = [[0u8; 64]; crate::protocol2::MAX_TRUSTEES];
        let cs = SharesHs(hashes);
        let bytes = cs.strand_serialize().unwrap();

        let d_cs: SharesHs = SharesHs::strand_deserialize(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }

    #[test]
    fn test_serialize_decryptionfactorshs() {
        let hashes = [[0u8; 64]; crate::protocol2::MAX_TRUSTEES];
        let cs = DecryptionFactorsHs(hashes);
        let bytes = cs.strand_serialize().unwrap();

        let d_cs: DecryptionFactorsHs = DecryptionFactorsHs::strand_deserialize(&bytes).unwrap();

        assert_eq!(cs.0, d_cs.0);
    }
}
