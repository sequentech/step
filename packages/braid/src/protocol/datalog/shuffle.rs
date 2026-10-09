// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crepe::crepe;

// Mixing.
//
// The mixing process is started when the Ballots
// Message is posted to the bulletin board. This
// message includes the set of trustees requested
// to participate in mixing and decryption. The
// trustee selection set is a set of 1-based
// indices that point to the trustees present in
// the configuration.
//
// The mixing chain begins with the first trustee
// in the the trustee selection set. This trustee
// will mix the ciphertexts in the Ballots message,
// and post the resulting Mix message.
//
// Each subsequent trustee in the trustee selection
// set will produce a mix using the previous mix's
// output ciphertexts as their input. This
// continues until the number of mixes reaches
// the threshold.
//
// (Note: it may be desirable to have _all_ trustees
// verify mixes, not just the selected ones)
//
// For each mix in the chain, all selected non-mixing
// trustees will verify the mix. The next mix in
// the chain does not begin until the previous mix
// has been signed by all selected trustees.
//
// When the last mix in the chain has been signed by
// all selected trustees, the mixing chain is complete.
//
// Actions:                 Mix
//                          SignMix
//
// Output predicates:       MixComplete
crepe! {

    ///////////////////////////////////////////////////////////////////////////
    // Inference.
    ///////////////////////////////////////////////////////////////////////////

    // First mix (ballots => mix)

    // Mix the ciphertexts source_h, at mix_number = 1 if:
    //      the configuration has been signed by all trustees and we are at position self_p,
    //      the public key has been signed by all,
    //      the ballots contain ciphertexts source_h,
    //      self_p has not mixed,
    //      self_p is the first trustee in the selected set.
    A(Action::Mix(cfg_h, batch, source_h, pk_h, PROTOCOL_MANAGER_INDEX, 1, trustees)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, _threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Ballots(cfg_h, batch, source_h, pk_h, trustees),
    !Mix(cfg_h, batch, source_h, _, 1, self_p),
    // Detects that we (self_p) are the trustee assigned to perform the first mix
    (trustees[0] - 1 == self_p);

    // After first mix (mix => mix)

    // Mix the ciphertexts source_h, at mix_number + 1 if:
    //      the configuration has been signed by all trustees and we are at position self_p,
    //      the public key has been signed by all,
    //      the ballots have been posted with selected set trustees,
    //      the mix prev_source_h => ciphertexts_h at mix_number is signed by signer_t,
    //      the mix prev_source_h => ciphertexts_h was signed by threshold # trustees,
    //      we have not mixed,
    //      the mix number is smaller than the threshold,
    //      we are assigned to perform the next mix,
    //      signer_t was assigned to the mix at mix_number in the selected set.
    A(Action::Mix(cfg_h, batch, ciphertexts_h, pk_h, signer_t, mix_number + 1, trustees)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Ballots(cfg_h, batch, _ciphertext_h, pk_h, trustees),
    Mix(cfg_h, batch, prev_source_h, ciphertexts_h, mix_number, signer_t),
    // Previous mix must have been signed by all selected trustees before next mix
    MixNumberSignedUpTo(cfg_h, batch, prev_source_h, ciphertexts_h, mix_number, threshold - 1),
    !Mix(cfg_h, batch, ciphertexts_h, _, mix_number + 1, self_p),
    // If mix_number == threshold, there is no next mix.
    // (This check must be performed first to avoid index out of bounds below)
    (mix_number < threshold),
    // Detects that we (self_p) are the trustee assigned to perform the next mix
    // because mix_number is 0 based, trustees[mix_number] mixes at mix_number + 1
    (trustees[mix_number] - 1 == self_p),
    // Sanity check, the previous signer must be the expected one
    (signer_t == trustees[mix_number - 1] - 1);

    // Sign first mix ( sign(ballots => ciphertexts) )

    // Sign the mix source_h => ciphertexts_h at mix_number 1 if:
    //      the configuration has been signed by all trustees and we are at position self_p,
    //      the public key has been signed by all,
    //      the mix source_h => ciphertexts_h is at mix_number 1 and was signed by signer_t,
    //      the ballots with ciphertexts source_h have been posted with selected set trustees,
    //      signert_t is the first trustee in the selected set,
    //      we have not signed the mix,
    //      we are part of the selected set, trustees.
    A(Action::SignMix(cfg_h, batch, source_h, PROTOCOL_MANAGER_INDEX, ciphertexts_h, signert_t, pk_h, 1)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, _threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Mix(cfg_h, batch, source_h, ciphertexts_h, 1, signert_t),
    // The first mix starts from the ballots
    Ballots(cfg_h, batch, source_h, pk_h, trustees),
    (signert_t == trustees[0] - 1),
    !MixSigned(cfg_h, batch, source_h, ciphertexts_h, self_p),
    // Only selected trustees participate (using TrusteeSet parameters from Ballots predicate)
    // Also include a verifier trustee
    (trustees.iter().any(|t| *t - 1 == self_p) || self_p == VERIFIER_INDEX);

    // Sign after first mix ( sign(mix => ciphertexts) )

    // Sign the mix source_h => ciphertexts_h at mix_number if:
    //      the configuration has been signed by all trustees and we are at position self_p,
    //      the public key has been signed by all,
    //      the ballots have been posted with selected set trustees,
    //      the mix source_h => ciphertexts_h is at mix_number and was signed by signert_t,
    //      signert_t was assigned to the mix at mix_number in the selected set,
    //      a mix chain ends at source_h at mix_number - 1 with its last mix signed by signers_t,
    //      we have not signed the mix,
    //      we are part of trustees, the selected set.
    A(Action::SignMix(cfg_h, batch, source_h, signers_t, ciphertexts_h, signert_t, pk_h, mix_number)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Ballots(cfg_h, batch, _source_h, pk_h, trustees),
    Mix(cfg_h, batch, source_h, ciphertexts_h, mix_number, signert_t),
    // These checks must be performed first to avoid out of bounds below
    (mix_number > 1),
    (mix_number <= threshold),
    (signert_t == trustees[mix_number - 1] - 1),
    // Get the signer for the source, necessary to retrieve the source artifact
    // as it is part of the StatementEntryIdentifier.
    MixChain(cfg_h, batch, _, source_h, mix_number - 1, signers_t),
    !MixSigned(cfg_h, batch, source_h, ciphertexts_h, self_p),
    // Only selected trustees participate (using TrusteeSet parameters from Ballots predicate)
    // Also include a verifier trustee
    (trustees.iter().any(|t| *t - 1 == self_p) || self_p == VERIFIER_INDEX);

    // The producer of a mix already counts as a signature.

    // A mix is signed by signer_t if:
    //      That mix was computed and signed by signer_t.
    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) <-
    Mix(cfg_h, batch, source_h, ciphertexts_h, _mix_number, signer_t);

    // This predicate adds the mix number and the signer position
    // to the the MixSigned predicate to produce the MixNumberSigned
    // predicate. The latter is used to detect when a mix has been signed
    // by all selected trustees. Only when all selected trustees
    // have signed a mix can the next mix in the chain begin.
    // This condition is checked with:
    // MixNumberSignedUpTo(cfg_h, batch, prev_source_h, ciphertexts_h, mix_number, threshold - 1),
    // in the mix => mix block above.

    // The mix source_h => ciphertexts_h at position mix_number is signed by a
    // trustee at signer_position in the selected set if:
    //      The mix has been signed by signer_t,
    //      The mix was at position mix_number,
    //      the ballots have been posted with selected set trustees,
    //      The position of signer_t in trustees is signer_position.
    MixNumberSigned(cfg_h, batch, source_h, ciphertexts_h, mix_number, signer_position) <-
    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t),
    Mix(cfg_h, batch, source_h, ciphertexts_h, mix_number, _),
    Ballots(cfg_h, batch, _, _pk_h, trustees),
    // Get the position of trustee that signed the mix (MixSigned).
    // Because trustees is 1-based and signer_t is 0 based we add 1.
    //
    // For example, if the selected trustees are [0, 2, 4] and
    // the current mix is signed by signer_t = 3, p will be set to 2,
    // the position of the value 4 (3 + 1) above.
    let p = trustees.iter().position(|t| *t == signer_t + 1),
    // NULL_TRUSTEE is a dummy value that will not contribute to
    // reaching the threshold
    let signer_position = p.unwrap_or(NULL_TRUSTEE);

    MixNumberSignedUpTo(cfg_h, batch, source_h, ciphertexts_h, mix_number, n + 1) <-
    MixNumberSignedUpTo(cfg_h, batch, source_h, ciphertexts_h, mix_number, n),
    MixNumberSigned(cfg_h, batch, source_h, ciphertexts_h, mix_number, n + 1);

    MixNumberSignedUpTo(cfg_h, batch, source_h, ciphertexts_h, mix_number, 0) <-
    MixNumberSigned(cfg_h, batch, source_h, ciphertexts_h, mix_number, 0);

    // Detect if there is a repeated mixing trustee

    // A repeated trustee exists in the chain if:
    //      the configuration with hash cfg_g was signed by all trustees,
    //      a mix signed by signer_t1 is at position mix_number1 with context cfg_h, batch
    //      a mix signed by signer_t1 is at mix_number2 with context cfg_h, batch
    //      the mixes are at different positions
    //      the signers for the two mixes are the same.
    MixRepeat(cfg_h, batch) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _threshold),
    Mix(cfg_h, batch, _, _, mix_number1, signer_t1),
    Mix(cfg_h, batch, _, _, mix_number2, signer_t2),
    (mix_number1 != mix_number2),
    (signer_t1 == signer_t2);

    // A mix chain spanning source_h and target_h ending at 1 exists if:
    //      the configuration with given threshold has been signed by all trustees,
    //      ballots were posted with ciphertexts source_h and selected set trustees,
    //      a mix with source_h => target_h exists at mix_number 1,
    //      the mix at mix_number 1 was signed by threshold # trustees,
    //      signer_t was assigned to the mix at mix_number 1 in the selected set.
    MixChain(cfg_h, batch, source_h, target_h, 1, signer_t) <-
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    Ballots(cfg_h, batch, source_h, _pk_h, trustees),
    Mix(cfg_h, batch, source_h, target_h, 1, signer_t),
    MixNumberSignedUpTo(cfg_h, batch, source_h, target_h, 1, threshold - 1),
    // The first mixer is at 0
    (signer_t == trustees[0] - 1);

    // A mix chain spanning source_h and target_h ending at mix_number_target exists if:
    //      the configuration with given threshold has been signed by all trustees,
    //      ballots were posted with selected set trustees,
    //      a mix chain exists spanning source_h => middle_h, ending at mix_number_source,
    //      a mix with middle_h => target_h exists at mix_number_target signed by signer_target_t,
    //      the mix at mix_number_target was signed by threshold # trustees,
    //      mix_number_target is the next mix after mix_number_source,
    //      signer_target_t was assigned to the mix at mix_number_target in the selected set.
    MixChain(cfg_h, batch, source_h, target_h, mix_number_target, signer_target_t) <-
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    Ballots(cfg_h, batch, _, _pk_h, trustees),
    Mix(cfg_h, batch, middle_h, target_h, mix_number_target, signer_target_t),
    MixNumberSignedUpTo(cfg_h, batch, middle_h, target_h, mix_number_target, threshold - 1),
    MixChain(cfg_h, batch, source_h, middle_h, mix_number_source, _),
    (mix_number_target == mix_number_source + 1),
    (signer_target_t == trustees[mix_number_target - 1] - 1);

    ///////////////////////////////////////////////////////////////////////////
    // Output predicates
    ///////////////////////////////////////////////////////////////////////////

    // The mixing process is complete and ending at ciphertexts_h if:
    //      the configuration with given threshold was signed by all trustees,
    //      ballots were posted with ciphertexts source_h,
    //      there is a mixchain spanning source_h and ciphertexts_h ending at threshold.
    OutP(Predicate::MixComplete(cfg_h, batch, threshold, ciphertexts_h, signer_t)) <-
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    Ballots(cfg_h, batch, source_h, _pk_h, _),
    MixChain(cfg_h, batch, source_h, ciphertexts_h, threshold, signer_t);

    // The mixing chain is complete if:
    //      the configuration with given threshold was signed by all trustees,
    //      ballots were posted with selected set trustees,
    //      a last mix with mix_number = threshold exists,
    //      the last mix was signed by threshold # trustees,
    //      the last mix was signed by the last member of trustees, the selected set.
    /*OutP(Predicate::MixComplete(cfg_h, batch, threshold, ciphertexts_h, signer_t)) <-
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    Ballots(cfg_h, batch, _, _pk_h, trustees),
    MixNumberSignedUpTo(cfg_h, batch, threshold, threshold - 1),
    !MixRepeat(cfg_h, batch),
    Mix(cfg_h, batch, _, ciphertexts_h, threshold, signer_t),
    // Sanity check, the signer of last mix should be last in trustee set.
    (signer_t == trustees[threshold - 1] - 1);*/

    // Fail if not all mixing trustees are unique
    DErr(DatalogError::MixRepeat(cfg_h, batch)) <-
    MixRepeat(cfg_h, batch);

    @input
    pub struct InP(Predicate);

    ///////////////////////////////////////////////////////////////////////////
    // Input relations.
    ///////////////////////////////////////////////////////////////////////////

    struct ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct PublicKeySignedAll(ConfigurationHash, PublicKeyHash, SharesHashes);
    struct Ballots(ConfigurationHash, BatchNumber, CiphertextsHash, PublicKeyHash, TrusteeSet);
    struct Mix(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, MixNumber, TrusteePosition);
    struct MixSigned(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, TrusteePosition);

    ///////////////////////////////////////////////////////////////////////////
    // Convert from InP predicates to crepe relations.
    ///////////////////////////////////////////////////////////////////////////

    ConfigurationSignedAll(cfg_h, self_position, num_t, threshold) <- InP(p),
    let Predicate::ConfigurationSignedAll(cfg_h, self_position, num_t, threshold) = p;

    PublicKeySignedAll(cfg_h, pk_h, shares_hs) <- InP(p),
    let Predicate::PublicKeySignedAll(cfg_h, pk_h, shares_hs) = p;

    Ballots(cfg_h, batch, ballots_h, pk_h, trustees) <- InP(p),
    let Predicate::Ballots(cfg_h, batch, ballots_h, pk_h, trustees) = p;

    Mix(cfg_h, batch, source_h, mix_h, mix_number, signer_t) <- InP(p),
    let Predicate::Mix(cfg_h, batch, source_h, mix_h, mix_number, signer_t) = p;

    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) <- InP(p),
    let Predicate::MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) = p;

    ///////////////////////////////////////////////////////////////////////////
    // Intermediate relations.
    ///////////////////////////////////////////////////////////////////////////

    struct MixNumberSigned(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, MixNumber, TrusteePosition);
    struct MixNumberSignedUpTo(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, MixNumber, TrusteePosition);
    struct MixChain(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, MixNumber, TrusteePosition);
    struct MixRepeat(ConfigurationHash, BatchNumber);

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
            "Datalog: state shuffle running with {} predicates, {:?}",
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
    const NUM_TRUSTEES: TrusteeCount = 4;
    const THRESHOLD: Threshold = 3;
    // Configuration positions 0, 1 and 2 are selected, in that mixing order.
    const SELECTED: [TrusteePosition; 3] = [0, 1, 2];
    const UNSELECTED: TrusteePosition = 3;

    const BALLOTS: u8 = 10;
    const MIX_1: u8 = 11;
    const MIX_2: u8 = 12;
    const MIX_3: u8 = 13;
    const OTHER_MIX_1: u8 = 21;
    const OTHER_MIX_2: u8 = 22;

    fn cfg_h() -> ConfigurationHash {
        ConfigurationHash([1u8; 64])
    }

    fn pk_h() -> PublicKeyHash {
        PublicKeyHash([2u8; 64])
    }

    fn ciphertexts(id: u8) -> CiphertextsHash {
        CiphertextsHash([id; 64])
    }

    fn trustee_set() -> TrusteeSet {
        let mut trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        for (i, position) in SELECTED.iter().enumerate() {
            trustees[i] = position + 1;
        }
        trustees
    }

    fn ballots_posted(self_p: TrusteePosition) -> Vec<Predicate> {
        vec![
            Predicate::ConfigurationSignedAll(cfg_h(), self_p, NUM_TRUSTEES, THRESHOLD),
            Predicate::PublicKeySignedAll(cfg_h(), pk_h(), SharesHashes([NULL_HASH; MAX_TRUSTEES])),
            Predicate::Ballots(cfg_h(), BATCH, ciphertexts(BALLOTS), pk_h(), trustee_set()),
        ]
    }

    fn mix(source: u8, target: u8, mix_number: MixNumber, signer: TrusteePosition) -> Predicate {
        Predicate::Mix(
            cfg_h(),
            BATCH,
            ciphertexts(source),
            ciphertexts(target),
            mix_number,
            signer,
        )
    }

    fn mix_signed(source: u8, target: u8, signer: TrusteePosition) -> Predicate {
        Predicate::MixSigned(
            cfg_h(),
            BATCH,
            ciphertexts(source),
            ciphertexts(target),
            signer,
        )
    }

    // The first mix, by the first selected trustee, signed by every selected trustee.
    fn first_mix_signed_by_all() -> Vec<Predicate> {
        vec![
            mix(BALLOTS, MIX_1, 1, SELECTED[0]),
            mix_signed(BALLOTS, MIX_1, SELECTED[1]),
            mix_signed(BALLOTS, MIX_1, SELECTED[2]),
        ]
    }

    fn sign_mix_targets(actions: &HashSet<Action>) -> Vec<CiphertextsHash> {
        actions
            .iter()
            .filter_map(|a| match a {
                Action::SignMix(_, _, _, _, target_h, _, _, _) => Some(*target_h),
                _ => None,
            })
            .collect()
    }

    fn mix_numbers(actions: &HashSet<Action>) -> Vec<MixNumber> {
        actions
            .iter()
            .filter_map(|a| match a {
                Action::Mix(_, _, _, _, _, mix_number, _) => Some(*mix_number),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn sign_mix_signs_mixes_from_expected_trustees() {
        let mut predicates = ballots_posted(SELECTED[2]);
        predicates.extend(first_mix_signed_by_all());
        predicates.push(mix(MIX_1, MIX_2, 2, SELECTED[1]));

        let (_, actions, errors) = D.run(&predicates);

        assert!(errors.is_empty());
        assert!(actions.contains(&Action::SignMix(
            cfg_h(),
            BATCH,
            ciphertexts(MIX_1),
            SELECTED[0],
            ciphertexts(MIX_2),
            SELECTED[1],
            pk_h(),
            2,
        )));

        let mut predicates = ballots_posted(SELECTED[1]);
        predicates.push(mix(BALLOTS, MIX_1, 1, SELECTED[0]));

        let (_, actions, _) = D.run(&predicates);

        assert!(actions.contains(&Action::SignMix(
            cfg_h(),
            BATCH,
            ciphertexts(BALLOTS),
            PROTOCOL_MANAGER_INDEX,
            ciphertexts(MIX_1),
            SELECTED[0],
            pk_h(),
            1,
        )));
    }

    #[test]
    fn sign_mix_ignores_first_mix_from_unexpected_trustee() {
        let mut predicates = ballots_posted(SELECTED[1]);
        predicates.push(mix(BALLOTS, OTHER_MIX_1, 1, UNSELECTED));
        predicates.push(mix(BALLOTS, MIX_1, 1, SELECTED[2]));

        let (_, actions, _) = D.run(&predicates);

        assert!(sign_mix_targets(&actions).is_empty());
    }

    #[test]
    fn sign_mix_ignores_later_mix_from_unexpected_trustee() {
        let mut predicates = ballots_posted(SELECTED[0]);
        predicates.extend(first_mix_signed_by_all());
        predicates.push(mix(MIX_1, OTHER_MIX_2, 2, UNSELECTED));
        predicates.push(mix(MIX_1, MIX_2, 2, SELECTED[2]));

        let (_, actions, _) = D.run(&predicates);

        assert!(sign_mix_targets(&actions).is_empty());
    }

    #[test]
    fn sign_mix_requires_source_on_mix_chain() {
        let mut predicates = ballots_posted(SELECTED[2]);
        predicates.extend(first_mix_signed_by_all());
        // A mix at position 1 that is not the first selected trustee's
        predicates.push(mix(BALLOTS, OTHER_MIX_1, 1, UNSELECTED));
        predicates.push(mix(OTHER_MIX_1, MIX_2, 2, SELECTED[1]));

        let (_, actions, _) = D.run(&predicates);

        assert!(!sign_mix_targets(&actions).contains(&ciphertexts(MIX_2)));
    }

    #[test]
    fn mix_signatures_are_counted_per_mix() {
        let mut predicates = ballots_posted(SELECTED[2]);
        predicates.extend(first_mix_signed_by_all());
        // Signatures by positions 0 and 2 on a different mix at position 2
        predicates.push(mix(MIX_1, OTHER_MIX_2, 2, UNSELECTED));
        predicates.push(mix_signed(MIX_1, OTHER_MIX_2, SELECTED[0]));
        predicates.push(mix_signed(MIX_1, OTHER_MIX_2, SELECTED[2]));
        // The expected mix at position 2, signed only by its producer
        predicates.push(mix(MIX_1, MIX_2, 2, SELECTED[1]));

        let (_, actions, _) = D.run(&predicates);

        assert!(!mix_numbers(&actions).contains(&3));
    }

    #[test]
    fn honest_mix_chain_completes() {
        let mut predicates = ballots_posted(SELECTED[0]);
        predicates.extend(first_mix_signed_by_all());
        predicates.push(mix(MIX_1, MIX_2, 2, SELECTED[1]));
        predicates.push(mix_signed(MIX_1, MIX_2, SELECTED[0]));
        predicates.push(mix_signed(MIX_1, MIX_2, SELECTED[2]));
        predicates.push(mix(MIX_2, MIX_3, 3, SELECTED[2]));
        predicates.push(mix_signed(MIX_2, MIX_3, SELECTED[0]));
        predicates.push(mix_signed(MIX_2, MIX_3, SELECTED[1]));

        let (outputs, actions, errors) = D.run(&predicates);

        assert!(errors.is_empty());
        assert!(actions.is_empty());
        assert!(outputs.contains(&Predicate::MixComplete(
            cfg_h(),
            BATCH,
            THRESHOLD,
            ciphertexts(MIX_3),
            SELECTED[2],
        )));
    }
}
