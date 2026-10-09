// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::protocol::datalog::{hashes_add, hashes_init};
use crate::protocol::predicate::Predicate;
use b3::messages::newtypes::*;
use crepe::crepe;
use std::collections::HashSet;

///////////////////////////////////////////////////////////////////////////
// Logic
///////////////////////////////////////////////////////////////////////////
crepe! {

    @input
    pub struct InP(Predicate);

    // Input relations, used to convert from InP predicates to crepe relations
    struct Configuration(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct ConfigurationSigned(ConfigurationHash, TrusteePosition);
    struct ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct PublicKeySignedAll(ConfigurationHash, PublicKeyHash, SharesHashes);
    struct PublicKey(ConfigurationHash, PublicKeyHash, SharesHashes, ChannelsHashes, TrusteePosition);
    struct PublicKeySigned(ConfigurationHash, PublicKeyHash, SharesHashes, ChannelsHashes, TrusteePosition);
    struct Ballots(ConfigurationHash, BatchNumber, CiphertextsHash, PublicKeyHash, TrusteeSet);
    struct MixComplete(ConfigurationHash, BatchNumber, MixNumber, CiphertextsHash, TrusteePosition);
    struct DecryptionFactors(ConfigurationHash, BatchNumber, DecryptionFactorsHash, CiphertextsHash, SharesHashes, TrusteePosition);
    struct Plaintexts(ConfigurationHash, BatchNumber,PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, PublicKeyHash, TrusteePosition);
    struct PlaintextsSigned(ConfigurationHash, BatchNumber, PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, PublicKeyHash, TrusteePosition);
    struct Mix(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, MixNumber, TrusteePosition);
    struct MixSigned(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, TrusteePosition);

    ConfigurationSignedAll(config_hash, self_position, num_t, threshold) <- InP(p),
    let Predicate::ConfigurationSignedAll(config_hash, self_position, num_t, threshold) = p;

    PublicKeySignedAll(cfg_h, pk_h, shares_hs) <- InP(p),
    let Predicate::PublicKeySignedAll(cfg_h, pk_h, shares_hs) = p;

    Ballots(cfg_h, batch, ballots_h, pk_h, selected) <- InP(p),
    let Predicate::Ballots(cfg_h, batch, ballots_h, pk_h, selected) = p;

    MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) = p;

    Mix(cfg_h, batch, source_h, mix_h, mix_number, signer_t) <- InP(p),
    let Predicate::Mix(cfg_h, batch, source_h, mix_h, mix_number, signer_t) = p;

    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) = p;

    DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) <- InP(p),
    let Predicate::DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) = p;

    Plaintexts(cfg_h, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, signer_t) <- InP(p),
    let Predicate::Plaintexts(cfg_h, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, signer_t) = p;

    PlaintextsSigned(cfg_h, batch, plaintexts_h, df_hs, cipher_h, pk_h, signer_t) <- InP(p),
    let Predicate::PlaintextsSigned(cfg_h, batch, plaintexts_h, df_hs, cipher_h, pk_h, signer_t) = p;

    ConfigurationSigned(cfg_h, signer_t) <- InP(p),
    let Predicate::ConfigurationSigned(cfg_h, signer_t) = p;

    PublicKey(config_hash, pk_hash, shares_hs, channels_hs, signer_t) <- InP(p),
    let Predicate::PublicKey(config_hash, pk_hash, shares_hs, channels_hs, signer_t) = p;

    PublicKeySigned(config_hash, pk_hash, shares_hs, channels_hs, signer_t) <- InP(p),
    let Predicate::PublicKeySigned(config_hash, pk_hash, shares_hs, channels_hs, signer_t) = p;

    Configuration(cfg_h, self_position, num_t, threshold) <- InP(p),
    let Predicate::Configuration(cfg_h, self_position, num_t, threshold) = p;

    // Intermediate relations

    struct ConfigurationSignedUpTo(ConfigurationHash, TrusteePosition);
    struct PublicKeySignedUpTo(ConfigurationHash, PublicKeyHash, SharesHashes, TrusteePosition);
    struct MixVerifiedUpto(ConfigurationHash, BatchNumber, CiphertextsHash, MixingHashes, TrusteeCount);
    struct MixRepeat(ConfigurationHash, BatchNumber);

    @output
    pub struct RootVerified(
        pub(crate) ConfigurationHash,
        pub(crate) PublicKeyHash,
    );

    @output
    pub struct Target(
        pub(crate) ConfigurationHash,
        pub(crate) BatchNumber,
        pub(crate) PublicKeyHash,
        pub(crate) CiphertextsHash,
        pub(crate) PlaintextsHash,
    );

    @output
    pub struct Verified(
        pub(crate) ConfigurationHash,
        pub(crate) BatchNumber,
        pub(crate) CiphertextsHash,
        pub(crate) CiphertextsHash,
        pub(crate) PublicKeyHash,
        pub(crate) PlaintextsHash,
        pub(crate) MixingHashes,
    );

    RootVerified(cfg_h, pk_h) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    PublicKeySigned(cfg_h, pk_h, _, _, VERIFIER_INDEX);

    Target(cfg_h, batch, pk_h, ballots_h, plaintexts_h, ) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    Ballots(cfg_h, batch, ballots_h, pk_h, _),
    Plaintexts(cfg_h, batch, plaintexts_h, _, _, _, _);

    ConfigurationSignedUpTo(cfg_h, n + 1) <-
    ConfigurationSignedUpTo(cfg_h, n),
    ConfigurationSigned(cfg_h, n + 1);

    ConfigurationSignedUpTo(cfg_h, 0) <-
    ConfigurationSigned(cfg_h, 0);

    PublicKeySigned(cfg_h, pk_h, shares_hs, channels_hs, 0) <-
    PublicKey(cfg_h, pk_h, shares_hs, channels_hs, 0);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n + 1) <-
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, n),
    PublicKeySigned(cfg_h, pk_h, shares_hs, _channels_hs, n + 1);

    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, 0) <-
    PublicKeySigned(cfg_h, pk_h, shares_hs, _channels_hs, 0);

    PublicKeySignedAll(cfg_h, pk_h, shares_hs) <-
    ConfigurationSignedAll(cfg_h, _self_p, num_t, _threshold),
    PublicKeySignedUpTo(cfg_h, pk_h, shares_hs, num_t - 1);

    ConfigurationSignedAll(cfg_h, self_position, num_t, threshold) <-
    Configuration(cfg_h, self_position, num_t, threshold),
    // We subtract 1 since trustees positions are 0 based
    ConfigurationSignedUpTo(cfg_h, num_t - 1);

    MixVerifiedUpto(cfg_h, batch, target_h, mixing_hs, 1) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    Ballots(cfg_h, batch, ballots_h, pk_h, _),
    !Plaintexts(cfg_h, batch, _, _, ballots_h, _, _),
    MixSigned(cfg_h, batch, ballots_h, target_h, VERIFIER_INDEX),
    let mixing_hs = MixingHashes(hashes_init(ballots_h.0));

    MixVerifiedUpto(cfg_h, batch, ciphertexts_h, new_mixing_hs, n + 1) <-
    MixSigned(cfg_h, batch, source_h, ciphertexts_h, VERIFIER_INDEX),
    MixVerifiedUpto(cfg_h, batch, source_h, mixing_hs, n),
    !Plaintexts(cfg_h, batch, _, _, source_h, _, _),
    let new_mixing_hs = MixingHashes(hashes_add(mixing_hs.0, source_h.0));

    // Extra checks to ensure that all mixing trustees are unique
    MixRepeat(cfg_h, batch) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _threshold),
    Mix(cfg_h, batch, _, _, mix_number1, signer_t1),
    Mix(cfg_h, batch, _, _, mix_number2, signer_t2),
    (mix_number1 != mix_number2),
    (signer_t1 == signer_t2);

    Verified(cfg_h, batch, ballots_h, last_ciphertexts_h, decryption_pk_h, plaintexts_h, new_mixing_hs) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, threshold),
    MixVerifiedUpto(cfg_h, batch, last_ciphertexts_h, mixing_hs, threshold),
    MixVerifiedUpto(cfg_h, batch, _, _, 1),
    MixSigned(cfg_h, batch, ballots_h, _target_h, VERIFIER_INDEX),
    Ballots(cfg_h, batch, ballots_h, _, selected),
    Plaintexts(cfg_h, batch, plaintexts_h, dfactors_hs, last_ciphertexts_h, decryption_pk_h, selected[0] - 1),
    // The plaintexts must have been recomputed and signed by the verifier itself
    PlaintextsSigned(cfg_h, batch, plaintexts_h, dfactors_hs, last_ciphertexts_h, decryption_pk_h, VERIFIER_INDEX),
    !MixRepeat(cfg_h, batch),
    let new_mixing_hs = MixingHashes(hashes_add(mixing_hs.0, last_ciphertexts_h.0));

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct S;

impl S {
    pub(crate) fn run(
        &self,
        predicates: &Vec<Predicate>,
    ) -> (HashSet<RootVerified>, HashSet<Target>, HashSet<Verified>) {
        let mut runtime = Crepe::new();
        let inputs: Vec<InP> = predicates.iter().map(|p| InP(*p)).collect();
        runtime.extend(&inputs);

        runtime.run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::datalog::{trustees_add, trustees_init};

    const BATCH: BatchNumber = 1;
    const NUM_TRUSTEES: TrusteeCount = 3;
    const THRESHOLD: Threshold = 2;

    struct Hashes {
        cfg: ConfigurationHash,
        pk: PublicKeyHash,
        shares: SharesHashes,
        ballots: CiphertextsHash,
        mix1: CiphertextsHash,
        mix2: CiphertextsHash,
        plaintexts: PlaintextsHash,
        dfactors: DecryptionFactorsHashes,
    }

    fn hashes() -> Hashes {
        Hashes {
            cfg: ConfigurationHash([1u8; 64]),
            pk: PublicKeyHash([2u8; 64]),
            shares: SharesHashes(hashes_init([3u8; 64])),
            ballots: CiphertextsHash([4u8; 64]),
            mix1: CiphertextsHash([5u8; 64]),
            mix2: CiphertextsHash([6u8; 64]),
            plaintexts: PlaintextsHash([7u8; 64]),
            dfactors: DecryptionFactorsHashes(hashes_add(hashes_init([8u8; 64]), [9u8; 64])),
        }
    }

    // Trustees 1 and 2 (1-based) are selected; trustee 0 (0-based) mixes first
    // and posts the plaintexts, trustee 1 produces the last mix.
    fn verified_mix_chain(h: &Hashes) -> Vec<Predicate> {
        let selected = trustees_add(trustees_init(1), 2);
        vec![
            Predicate::ConfigurationSignedAll(h.cfg, VERIFIER_INDEX, NUM_TRUSTEES, THRESHOLD),
            Predicate::PublicKeySignedAll(h.cfg, h.pk, h.shares),
            Predicate::Ballots(h.cfg, BATCH, h.ballots, h.pk, selected),
            Predicate::Mix(h.cfg, BATCH, h.ballots, h.mix1, 1, 0),
            Predicate::Mix(h.cfg, BATCH, h.mix1, h.mix2, 2, 1),
            Predicate::MixSigned(h.cfg, BATCH, h.ballots, h.mix1, VERIFIER_INDEX),
            Predicate::MixSigned(h.cfg, BATCH, h.mix1, h.mix2, VERIFIER_INDEX),
        ]
    }

    fn plaintexts(h: &Hashes, cipher_h: CiphertextsHash, signer_t: TrusteePosition) -> Predicate {
        Predicate::Plaintexts(
            h.cfg,
            BATCH,
            h.plaintexts,
            h.dfactors,
            cipher_h,
            h.pk,
            signer_t,
        )
    }

    fn plaintexts_signed(
        h: &Hashes,
        cipher_h: CiphertextsHash,
        signer_t: TrusteePosition,
    ) -> Predicate {
        Predicate::PlaintextsSigned(
            h.cfg,
            BATCH,
            h.plaintexts,
            h.dfactors,
            cipher_h,
            h.pk,
            signer_t,
        )
    }

    #[test]
    fn verified_requires_verifier_plaintexts_signature() {
        let h = hashes();
        let mut predicates = verified_mix_chain(&h);
        predicates.push(plaintexts(&h, h.mix2, 0));
        predicates.push(plaintexts_signed(&h, h.mix2, 1));

        let (_, targets, verified) = S.run(&predicates);

        assert_eq!(targets.len(), 1);
        assert!(verified.is_empty());
    }

    #[test]
    fn verified_with_verifier_plaintexts_signature() {
        let h = hashes();
        let mut predicates = verified_mix_chain(&h);
        predicates.push(plaintexts(&h, h.mix2, 0));
        predicates.push(plaintexts_signed(&h, h.mix2, VERIFIER_INDEX));

        let (_, _, verified) = S.run(&predicates);

        assert_eq!(verified.len(), 1);
        let v = verified.iter().next().expect("one verified batch");
        assert_eq!(v.1, BATCH);
        assert_eq!(v.2, h.ballots);
        assert_eq!(v.3, h.mix2);
        assert_eq!(v.4, h.pk);
        assert_eq!(v.5, h.plaintexts);
    }

    #[test]
    fn verified_rejects_plaintexts_not_over_last_mix() {
        let h = hashes();
        let other = CiphertextsHash([10u8; 64]);
        let mut predicates = verified_mix_chain(&h);
        predicates.push(plaintexts(&h, other, 0));
        predicates.push(plaintexts_signed(&h, other, VERIFIER_INDEX));

        let (_, _, verified) = S.run(&predicates);

        assert!(verified.is_empty());
    }
}
