// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
    //      the mix at mix_number has output ciphertexts source_h and is signed by signer_t,
    //      the mix at mix_number was signed by threshold # trustees,
    //      we have not mixed,
    //      the mix number is smaller than the threshold,
    //      we are assigned to perform the next mix,
    //      signer_t was assigned to the mix at mix_number in the selected set.
    A(Action::Mix(cfg_h, batch, ciphertexts_h, pk_h, signer_t, mix_number + 1, trustees)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Ballots(cfg_h, batch, _ciphertext_h, pk_h, trustees),
    Mix(cfg_h, batch, _, ciphertexts_h, mix_number, signer_t),
    // Previous mix must have been signed by all selected trustees before next mix
    MixNumberSignedUpTo(cfg_h, batch, mix_number, threshold - 1),
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
    //      we have not signed the mix,
    //      we are part of the selected set, trustees.
    A(Action::SignMix(cfg_h, batch, source_h, PROTOCOL_MANAGER_INDEX, ciphertexts_h, signert_t, pk_h, 1)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, _threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Mix(cfg_h, batch, source_h, ciphertexts_h, 1, signert_t),
    // The first mix starts from the ballots
    Ballots(cfg_h, batch, source_h, pk_h, trustees),
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
    //      the mix _ => source_h is at mix_number - 1 and was signed by signers_t,
    //      the mix at mix_number - 1 was signed by threshold # trustees,
    //      we have not signed the mix,
    //      we are part of trustees, the selected set.
    A(Action::SignMix(cfg_h, batch, source_h, signers_t, ciphertexts_h, signert_t, pk_h, mix_number)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Ballots(cfg_h, batch, _source_h, pk_h, trustees),
    Mix(cfg_h, batch, source_h, ciphertexts_h, mix_number, signert_t),
    // Get the signer for the source, necessary to retrieve the source artifact
    // as it is part of the StatementEntryIdentifier.
    Mix(cfg_h, batch, _, source_h, mix_number - 1, signers_t),
    MixNumberSignedUpTo(cfg_h, batch, mix_number - 1, threshold - 1),
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
    // MixNumberSignedUpTo(cfg_h, batch, mix_number, threshold - 1),
    // in the mix => mix block above.

    // The mix at position mix_number is signed by a trustee at
    // signer_position in the selected set if:
    //      The mix has been signed by signer_t,
    //      The mix was at position mix_number,
    //      the ballots have been posted with selected set trustees,
    //      The position of signer_t in trustees is signer_position.
    MixNumberSigned(cfg_h, batch, mix_number, signer_position) <-
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

    MixNumberSignedUpTo(cfg_h, batch, mix_number, n + 1) <-
    MixNumberSignedUpTo(cfg_h, batch, mix_number, n),
    MixNumberSigned(cfg_h, batch, mix_number, n + 1);

    MixNumberSignedUpTo(cfg_h, batch, mix_number, 0) <-
    MixNumberSigned(cfg_h, batch, mix_number, 0);

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
    MixNumberSignedUpTo(cfg_h, batch, 1, threshold - 1),
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
    MixNumberSignedUpTo(cfg_h, batch, mix_number_target, threshold - 1),
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

    struct MixNumberSigned(ConfigurationHash, BatchNumber, MixNumber, TrusteePosition);
    struct MixNumberSignedUpTo(ConfigurationHash, BatchNumber, MixNumber, TrusteePosition);
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
