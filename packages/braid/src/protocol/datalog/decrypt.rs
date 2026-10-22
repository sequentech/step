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
    MixComplete(cfg_h, batch, _mix_n, ciphertexts_h, _mix_signer),
    // Trustees are 1 based, so n - 1
    DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, selected[0] - 1),
    let ts = super::trustees_init(selected[0]),
    let hs = DecryptionFactorsHashes(super::hashes_init(dfactors_h.0));

    DecryptionFactorsAcc(cfg_h, batch, new_dfactor_hs, new_ts, n + 1) <-
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    DecryptionFactorsAcc(cfg_h, batch, dfactor_hs, ts, n),
    // n accumulator is 0-based, threshold is 1-based, so the last value of n + 1 is threshold - 1
    (n + 1 <= threshold - 1),
    PublicKeySignedAll(cfg_h, pk_h, shares_hs),
    Ballots(cfg_h, batch, _, pk_h, selected),
    MixComplete(cfg_h, batch, _mix_n, ciphertexts_h, _mix_signer),
    // Trustees are 1 based, so n - 1
    DecryptionFactors(cfg_h, batch, dfactors_h, ciphertexts_h, shares_hs, selected[n + 1] - 1),
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
    // The board keeps one PlaintextsSigned per trustee and batch
    !PlaintextsSigned(cfg_h, batch, _, _, _, _, self_p);

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
    const SIGNER_POSITION: TrusteePosition = 1;
    const LAST_MIX_SIGNER: TrusteePosition = 1;

    const CFG_H: ConfigurationHash = ConfigurationHash([1u8; 64]);
    const PK_H: PublicKeyHash = PublicKeyHash([2u8; 64]);
    const SHARES_H: Hash = [3u8; 64];
    const BALLOTS_H: CiphertextsHash = CiphertextsHash([4u8; 64]);
    const LAST_MIX_H: CiphertextsHash = CiphertextsHash([6u8; 64]);
    const PLAINTEXTS_H: PlaintextsHash = PlaintextsHash([7u8; 64]);
    const FIRST_DFACTORS_H: Hash = [8u8; 64];
    const SECOND_DFACTORS_H: Hash = [9u8; 64];
    const OTHER_CIPHERTEXTS_H: CiphertextsHash = CiphertextsHash([10u8; 64]);

    /// Predicates under which trustee `self_p` takes part in decrypting
    /// `BATCH`: configuration and public key signed by all, the batch's
    /// ballots with trustees 1 and 2 (1-based) selected, and `LAST_MIX_H` as
    /// the completed mix.
    fn mix_complete(self_p: TrusteePosition) -> Vec<Predicate> {
        let selected = trustees_add(trustees_init(1), 2);
        vec![
            Predicate::ConfigurationSignedAll(CFG_H, self_p, NUM_TRUSTEES, THRESHOLD),
            Predicate::PublicKeySignedAll(CFG_H, PK_H, SharesHashes(hashes_init(SHARES_H))),
            Predicate::Ballots(CFG_H, BATCH, BALLOTS_H, PK_H, selected),
            Predicate::MixComplete(CFG_H, BATCH, THRESHOLD, LAST_MIX_H, LAST_MIX_SIGNER),
        ]
    }

    /// The decryption factors hashes of both selected trustees, in selection
    /// order.
    fn dfactors_hs() -> DecryptionFactorsHashes {
        DecryptionFactorsHashes(hashes_add(hashes_init(FIRST_DFACTORS_H), SECOND_DFACTORS_H))
    }

    /// A `DecryptionFactors` statement for `BATCH` from trustee `signer_t`,
    /// computed over `cipher_h`.
    fn dfactors(
        dfactors_h: Hash,
        cipher_h: CiphertextsHash,
        signer_t: TrusteePosition,
    ) -> Predicate {
        Predicate::DecryptionFactors(
            CFG_H,
            BATCH,
            DecryptionFactorsHash(dfactors_h),
            cipher_h,
            SharesHashes(hashes_init(SHARES_H)),
            signer_t,
        )
    }

    /// A `Plaintexts` statement for `BATCH` from the first selected trustee,
    /// decrypted from `cipher_h` under `pk_h`.
    fn plaintexts(cipher_h: CiphertextsHash, pk_h: PublicKeyHash) -> Predicate {
        Predicate::Plaintexts(CFG_H, BATCH, PLAINTEXTS_H, dfactors_hs(), cipher_h, pk_h, 0)
    }

    /// A `PlaintextsSigned` statement for `BATCH` from `SIGNER_POSITION`.
    fn plaintexts_signed(plaintexts_h: PlaintextsHash, cipher_h: CiphertextsHash) -> Predicate {
        Predicate::PlaintextsSigned(
            CFG_H,
            BATCH,
            plaintexts_h,
            dfactors_hs(),
            cipher_h,
            PK_H,
            SIGNER_POSITION,
        )
    }

    /// Runs the decryption datalog and keeps only the actions that `keep`
    /// selects.
    fn actions(predicates: &Vec<Predicate>, keep: fn(&Action) -> bool) -> Vec<Action> {
        let (_, actions, _) = D.run(predicates);
        actions.into_iter().filter(keep).collect()
    }

    fn is_sign_plaintexts(action: &Action) -> bool {
        matches!(action, Action::SignPlaintexts(..))
    }

    fn is_compute_plaintexts(action: &Action) -> bool {
        matches!(action, Action::ComputePlaintexts(..))
    }

    /// Plaintexts decrypted from the completed mix under the ballots key are
    /// signed, and the action names that mix and its producer.
    #[test]
    fn sign_plaintexts_over_completed_mix() {
        let mut predicates = mix_complete(SIGNER_POSITION);
        predicates.push(plaintexts(LAST_MIX_H, PK_H));

        let actions = actions(&predicates, is_sign_plaintexts);

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
        let mut predicates = mix_complete(SIGNER_POSITION);
        predicates.push(plaintexts(OTHER_CIPHERTEXTS_H, PK_H));

        assert!(actions(&predicates, is_sign_plaintexts).is_empty());
    }

    /// Plaintexts that name a key other than the ballots public key are not
    /// signed.
    #[test]
    fn sign_plaintexts_requires_matching_public_key() {
        let mut predicates = mix_complete(SIGNER_POSITION);
        predicates.push(plaintexts(LAST_MIX_H, PublicKeyHash([11u8; 64])));

        assert!(actions(&predicates, is_sign_plaintexts).is_empty());
    }

    /// Once the trustee has signed the plaintexts over the completed mix, it
    /// does not sign them again.
    #[test]
    fn sign_plaintexts_not_repeated_after_signing() {
        let mut predicates = mix_complete(SIGNER_POSITION);
        predicates.push(plaintexts(LAST_MIX_H, PK_H));
        predicates.push(plaintexts_signed(PLAINTEXTS_H, LAST_MIX_H));

        assert!(actions(&predicates, is_sign_plaintexts).is_empty());
    }

    /// A trustee that has signed the plaintexts of the completed mix does not
    /// sign again when the plaintexts statement names other ciphertexts.
    #[test]
    fn sign_plaintexts_not_repeated_when_statement_names_other_ciphertexts() {
        let mut predicates = mix_complete(SIGNER_POSITION);
        predicates.push(plaintexts(OTHER_CIPHERTEXTS_H, PK_H));
        predicates.push(plaintexts_signed(PLAINTEXTS_H, LAST_MIX_H));

        assert!(actions(&predicates, is_sign_plaintexts).is_empty());
    }

    /// A trustee signs the plaintexts of a batch at most once, even when its
    /// signature covers other plaintexts than the current statement.
    #[test]
    fn sign_plaintexts_at_most_once_per_batch() {
        let mut predicates = mix_complete(SIGNER_POSITION);
        predicates.push(plaintexts(LAST_MIX_H, PK_H));
        predicates.push(plaintexts_signed(PlaintextsHash([12u8; 64]), LAST_MIX_H));

        assert!(actions(&predicates, is_sign_plaintexts).is_empty());
    }

    /// The first selected trustee computes the plaintexts from the selected
    /// trustees' decryption factors over the completed mix.
    #[test]
    fn compute_plaintexts_from_factors_over_completed_mix() {
        let mut predicates = mix_complete(0);
        predicates.push(dfactors(FIRST_DFACTORS_H, LAST_MIX_H, 0));
        predicates.push(dfactors(SECOND_DFACTORS_H, LAST_MIX_H, 1));

        let actions = actions(&predicates, is_compute_plaintexts);

        assert_eq!(actions.len(), 1);
        let Action::ComputePlaintexts(_, batch, pk_h, dfactors_hs, cipher_h, mix_signer, _, _) =
            actions[0]
        else {
            panic!("expected ComputePlaintexts, got {:?}", actions[0]);
        };
        assert_eq!(batch, BATCH);
        assert_eq!(pk_h, PK_H);
        assert_eq!(dfactors_hs, self::dfactors_hs());
        assert_eq!(cipher_h, LAST_MIX_H);
        assert_eq!(mix_signer, LAST_MIX_SIGNER);
    }

    /// Decryption factors computed over ciphertexts other than the completed
    /// mix are not combined into plaintexts.
    #[test]
    fn compute_plaintexts_requires_factors_over_completed_mix() {
        let mut predicates = mix_complete(0);
        predicates.push(dfactors(FIRST_DFACTORS_H, LAST_MIX_H, 0));
        predicates.push(dfactors(SECOND_DFACTORS_H, OTHER_CIPHERTEXTS_H, 1));

        assert!(actions(&predicates, is_compute_plaintexts).is_empty());
    }
}
