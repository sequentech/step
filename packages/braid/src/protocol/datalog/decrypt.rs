// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crepe::crepe;

// Distributed decryption.
//
// Actions:                ComputeDecryptionFactors
//                         ComputePlaintexts
//                         SignPlaintexts
crepe! {

    ///////////////////////////////////////////////////////////////////////////
    // Inference.
    ///////////////////////////////////////////////////////////////////////////

    A(Action::ComputeDecryptionFactors(cfg_h, batch, channels_hs, ciphertexts_h, signer_t, pk_h, shares_hs, self_p, num_t, threshold, selected)) <-
    PublicKeySignedAll(cfg_h, pk_h, shares_hs),
    ConfigurationSignedAll(cfg_h, self_p, num_t, threshold),
    ChannelsAllSignedAll(cfg_h, channels_hs),
    MixComplete(cfg_h, batch, _mix_n, ciphertexts_h, signer_t),
    Ballots(cfg_h, batch, _ballots_h, pk_h, selected),
    !DecryptionFactors(cfg_h, batch, _, ciphertexts_h, shares_hs, self_p),
    // Only selected trustees participate (using TrusteeSet parameters from Ballots predicate)
    (selected.iter().any(|t| *t - 1 == self_p));

    DecryptionFactorsAcc(cfg_h, batch, hs, ts, 0) <-
    PublicKeySignedAll(cfg_h, pk_h, shares_hs),
    Ballots(cfg_h, batch, _, pk_h, selected),
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, _threshold),
    // Trustees are 1 based, so n - 1
    DecryptionFactors(cfg_h, batch, dfactors_h, _ciphertexts_h, shares_hs, selected[0] - 1),
    let ts = super::trustees_init(selected[0]),
    let hs = DecryptionFactorsHashes(super::hashes_init(dfactors_h.0));

    DecryptionFactorsAcc(cfg_h, batch, new_dfactor_hs, new_ts, n + 1) <-
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    DecryptionFactorsAcc(cfg_h, batch, dfactor_hs, ts, n),
    // n accumulator is 0-based, threshold is 1-based, so the last value of n + 1 is threshold - 1
    (n + 1 <= threshold - 1),
    PublicKeySignedAll(cfg_h, pk_h, shares_hs),
    Ballots(cfg_h, batch, _, pk_h, selected),
    // Trustees are 1 based, so n - 1
    DecryptionFactors(cfg_h, batch, dfactors_h, _ciphertexts_h, shares_hs, selected[n + 1] - 1),
    let new_ts = super::trustees_add(ts, selected[n + 1]),
    let new_dfactor_hs = DecryptionFactorsHashes(super::hashes_add(dfactor_hs.0, dfactors_h.0));

    DecryptionFactorsAll(cfg_h, batch, dfactor_hs, ciphertexts_h, mix_signer, ts, threshold) <-
    MixComplete(cfg_h, batch, _mix_n, ciphertexts_h, mix_signer),
    DecryptionFactorsAcc(cfg_h, batch, dfactor_hs, ts, _),
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    (trustees_count(ts) == threshold);

    A(Action::ComputePlaintexts(cfg_h, batch, pk_h, dfactors_hs, ciphertexts_h, mix_signer, selected, threshold)) <-
    Ballots(cfg_h, batch, _, pk_h, selected),
    ConfigurationSignedAll(cfg_h, selected[0] - 1, _num_t, threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    DecryptionFactorsAll(cfg_h, batch, dfactors_hs, ciphertexts_h, mix_signer, _, threshold),
    !Plaintexts(cfg_h, batch, _, dfactors_hs, _, _, selected[0] - 1);

    PlaintextsSigned(cfg_h, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, selected[0] - 1) <-
    Ballots(cfg_h, batch, _, _, selected),
    Plaintexts(cfg_h, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, selected[0] - 1);

    A(Action::SignPlaintexts(cfg_h, batch, pk_h, plaintexts_h, dfactors_hs, ciphertexts_h, mix_signer, selected, threshold)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_hs),
    Ballots(cfg_h, batch, _, pk_h, selected),
    MixComplete(cfg_h, batch, _mix_n, ciphertexts_h, mix_signer),
    Plaintexts(cfg_h, batch, plaintexts_h, dfactors_hs, ciphertexts_h, pk_h, selected[0] - 1),
    !PlaintextsSigned(cfg_h, batch, plaintexts_h, dfactors_hs, ciphertexts_h, pk_h, self_p);

    ///////////////////////////////////////////////////////////////////////////
    // Input relations.
    ///////////////////////////////////////////////////////////////////////////

    struct ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct PublicKeySignedAll(ConfigurationHash, PublicKeyHash, SharesHashes);
    struct ChannelsAllSignedAll(ConfigurationHash, ChannelsHashes);
    struct Ballots(ConfigurationHash, BatchNumber, CiphertextsHash, PublicKeyHash, TrusteeSet);
    struct MixComplete(ConfigurationHash, BatchNumber, MixNumber, CiphertextsHash, TrusteePosition);
    struct DecryptionFactors(ConfigurationHash, BatchNumber, DecryptionFactorsHash, CiphertextsHash, SharesHashes, TrusteePosition);
    struct Plaintexts(ConfigurationHash, BatchNumber,PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, PublicKeyHash, TrusteePosition);
    struct PlaintextsSigned(ConfigurationHash, BatchNumber, PlaintextsHash, DecryptionFactorsHashes, CiphertextsHash, PublicKeyHash, TrusteePosition);

    ///////////////////////////////////////////////////////////////////////////
    // Convert from InP predicates to crepe relations.
    ///////////////////////////////////////////////////////////////////////////

    ConfigurationSignedAll(config_hash, self_position, num_t, threshold) <- InP(p),
    let Predicate::ConfigurationSignedAll(config_hash, self_position, num_t, threshold) = p;

    PublicKeySignedAll(cfg_h, pk_h, shares_hs) <- InP(p),
    let Predicate::PublicKeySignedAll(cfg_h, pk_h, shares_hs) = p;

    ChannelsAllSignedAll(cfg_h, channels_hs) <- InP(p),
    let Predicate::ChannelsAllSignedAll(cfg_h, channels_hs) = p;

    Ballots(cfg_h, batch, ballots_h, pk_h, selected) <- InP(p),
    let Predicate::Ballots(cfg_h, batch, ballots_h, pk_h, selected) = p;

    MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixComplete(cfg_h, batch, mix_number, ciphertexts_h, signer_t) = p;

    DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) <- InP(p),
    let Predicate::DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, signer_t) = p;

    Plaintexts(ch, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, signer_t) <- InP(p),
    let Predicate::Plaintexts(ch, batch, plaintexts_h, dfactors_hs, cipher_h, pk_h, signer_t) = p;

    PlaintextsSigned(ch, batch, plaintexts_h, df_hs, cipher_h, pk_h, signer_t) <- InP(p),
    let Predicate::PlaintextsSigned(ch, batch, plaintexts_h, df_hs, cipher_h, pk_h, signer_t) = p;

    ///////////////////////////////////////////////////////////////////////////
    // Intermediate relations.
    ///////////////////////////////////////////////////////////////////////////

    struct DecryptionFactorsAcc(ConfigurationHash, BatchNumber, DecryptionFactorsHashes, TrusteeSet, TrusteePosition);
    struct DecryptionFactorsAll(ConfigurationHash, BatchNumber, DecryptionFactorsHashes, CiphertextsHash, TrusteePosition, TrusteeSet, TrusteeCount);

    @input
    pub struct InP(Predicate);

    @output
    #[derive(Debug)]
    pub struct OutP(Predicate);

    @output
    #[derive(Debug)]
    pub struct A(pub(crate) Action);

    @output
    #[derive(Debug)]
    pub struct DErr(DatalogError);

}

///////////////////////////////////////////////////////////////////////////
// Running (see datalog::get_phases())
///////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct D;

impl D {
    pub(crate) fn run(
        &self,
        predicates: &Vec<Predicate>,
    ) -> (HashSet<Predicate>, HashSet<Action>, HashSet<DatalogError>) {
        trace!(
            "Datalog: state cfg running with {} predicates, {:?}",
            predicates.len(),
            predicates
        );

        let mut runtime = Crepe::new();
        let inputs: Vec<InP> = predicates.iter().map(|p| InP(*p)).collect();
        runtime.extend(&inputs);

        let result: (HashSet<OutP>, HashSet<A>, HashSet<DErr>) = runtime.run();

        (
            result.0.iter().map(|a| a.0).collect::<HashSet<Predicate>>(),
            result.1.iter().map(|i| i.0).collect::<HashSet<Action>>(),
            result
                .2
                .iter()
                .map(|i| i.0)
                .collect::<HashSet<DatalogError>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BATCH: BatchNumber = 1;
    const NUM_TRUSTEES: TrusteeCount = 3;
    const THRESHOLD: Threshold = 2;
    const SELF_POSITION: TrusteePosition = 1;
    const LAST_MIX_SIGNER: TrusteePosition = 1;

    const CFG_H: ConfigurationHash = ConfigurationHash([1u8; 64]);
    const PK_H: PublicKeyHash = PublicKeyHash([2u8; 64]);
    const BALLOTS_H: CiphertextsHash = CiphertextsHash([4u8; 64]);
    const LAST_MIX_H: CiphertextsHash = CiphertextsHash([6u8; 64]);
    const PLAINTEXTS_H: PlaintextsHash = PlaintextsHash([7u8; 64]);

    /// Predicates under which trustee `SELF_POSITION` may sign the plaintexts
    /// of `BATCH`: configuration and public key signed by all, the batch's
    /// ballots, and `LAST_MIX_H` as the completed mix.
    fn mix_complete() -> Vec<Predicate> {
        let selected = trustees_add(trustees_init(1), 2);
        vec![
            Predicate::ConfigurationSignedAll(CFG_H, SELF_POSITION, NUM_TRUSTEES, THRESHOLD),
            Predicate::PublicKeySignedAll(CFG_H, PK_H, SharesHashes(hashes_init([3u8; 64]))),
            Predicate::Ballots(CFG_H, BATCH, BALLOTS_H, PK_H, selected),
            Predicate::MixComplete(CFG_H, BATCH, THRESHOLD, LAST_MIX_H, LAST_MIX_SIGNER),
        ]
    }

    /// A `Plaintexts` statement for `BATCH` from the first selected trustee,
    /// decrypted from `cipher_h` under `pk_h`.
    fn plaintexts(cipher_h: CiphertextsHash, pk_h: PublicKeyHash) -> Predicate {
        let dfactors_hs = DecryptionFactorsHashes(hashes_add(hashes_init([8u8; 64]), [9u8; 64]));
        Predicate::Plaintexts(CFG_H, BATCH, PLAINTEXTS_H, dfactors_hs, cipher_h, pk_h, 0)
    }

    /// Runs the decryption datalog and keeps only its `SignPlaintexts` actions.
    fn sign_plaintexts_actions(predicates: &Vec<Predicate>) -> Vec<Action> {
        let (_, actions, _) = D.run(predicates);
        actions
            .into_iter()
            .filter(|a| matches!(a, Action::SignPlaintexts(..)))
            .collect()
    }

    /// Plaintexts decrypted from the completed mix under the ballots key are
    /// signed, and the action names that mix and its producer.
    #[test]
    fn sign_plaintexts_over_completed_mix() {
        let mut predicates = mix_complete();
        predicates.push(plaintexts(LAST_MIX_H, PK_H));

        let actions = sign_plaintexts_actions(&predicates);

        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0],
            Action::SignPlaintexts(
                _,
                BATCH,
                PK_H,
                PLAINTEXTS_H,
                _,
                LAST_MIX_H,
                LAST_MIX_SIGNER,
                _,
                THRESHOLD
            )
        ));
    }

    /// Plaintexts that name ciphertexts other than the completed mix are not
    /// signed.
    #[test]
    fn sign_plaintexts_requires_matching_ciphertexts() {
        let mut predicates = mix_complete();
        predicates.push(plaintexts(CiphertextsHash([10u8; 64]), PK_H));

        assert!(sign_plaintexts_actions(&predicates).is_empty());
    }

    /// Plaintexts that name a key other than the ballots public key are not
    /// signed.
    #[test]
    fn sign_plaintexts_requires_matching_public_key() {
        let mut predicates = mix_complete();
        predicates.push(plaintexts(LAST_MIX_H, PublicKeyHash([11u8; 64])));

        assert!(sign_plaintexts_actions(&predicates).is_empty());
    }
}
