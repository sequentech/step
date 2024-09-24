// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crepe::crepe;

///////////////////////////////////////////////////////////////////////////
// Logic
///////////////////////////////////////////////////////////////////////////
crepe! {

    @input
    pub struct InP(Predicate);

    // Input relations, used to convert from InP predicates to crepe relations

    struct ConfigurationSignedAll(ConfigurationHash, TrusteePosition, TrusteeCount, Threshold);
    struct PublicKeySignedAll(ConfigurationHash, PublicKeyHash, SharesHashes);
    struct Ballots(ConfigurationHash, BatchNumber, CiphertextsHash, PublicKeyHash, TrusteeSet);
    struct Mix(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, MixNumber, TrusteePosition);
    struct MixSigned(ConfigurationHash, BatchNumber, CiphertextsHash, CiphertextsHash, TrusteePosition);

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

    // Intermediate relations

    struct MixNumberSigned(ConfigurationHash, BatchNumber, MixNumber, TrusteePosition);
    struct MixNumberSignedUpTo(ConfigurationHash, BatchNumber, MixNumber, TrusteePosition);
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

    // First mix (ballots => mix)
    A(Action::Mix(cfg_h, batch, source_h, pk_h, PROTOCOL_MANAGER_INDEX, 1, trustees)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, _threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Ballots(cfg_h, batch, source_h, pk_h, trustees),
    !Mix(cfg_h, batch, source_h, _, 1, self_p),
    // Detects that we (self_p) are the trustee assigned to perform the first mix
    (trustees[0] - 1 == self_p);

    // After first mix (mix => mix)
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
    (trustees[mix_number] - 1 == self_p),
    // Sanity check, the previous signer must be the expected one
    (signer_t == trustees[mix_number - 1] - 1);

    // Sign first mix ( sign(ballots => ciphertexts) )
    A(Action::SignMix(cfg_h, batch, source_h, PROTOCOL_MANAGER_INDEX, ciphertexts_h, signert_t, pk_h, mix_number)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, _threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Mix(cfg_h, batch, source_h, ciphertexts_h, mix_number, signert_t),
    // The first mix starts from the ballots
    Ballots(cfg_h, batch, source_h, pk_h, trustees),
    !MixSigned(cfg_h, batch, source_h, ciphertexts_h, self_p),
    // Only selected trustees participate (using TrusteeSet parameters from Ballots predicate)
    // Also include a verifier trustee
    (trustees.iter().any(|t| *t - 1 == self_p) || self_p == VERIFIER_INDEX);

    // Sign after first mix ( sign(mix => ciphertexts) )
    A(Action::SignMix(cfg_h, batch, source_h, signers_t, ciphertexts_h, signert_t, pk_h, mix_number)) <-
    ConfigurationSignedAll(cfg_h, self_p, _num_t, _threshold),
    PublicKeySignedAll(cfg_h, pk_h, _shares_h),
    Ballots(cfg_h, batch, _source_h, pk_h, trustees),
    Mix(cfg_h, batch, source_h, ciphertexts_h, mix_number, signert_t),
    // Get the signer for the source, necessary to retrieve the source artifact
    // as it is part of the StatementEntryIdentifier.
    Mix(cfg_h, batch, _, source_h, _, signers_t),
    !MixSigned(cfg_h, batch, source_h, ciphertexts_h, self_p),
    // Only selected trustees participate (using TrusteeSet parameters from Ballots predicate)
    // Also include a verifier trustee
    (trustees.iter().any(|t| *t - 1 == self_p) || self_p == VERIFIER_INDEX);

    // The producer of a mix already counts as a signature
    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t) <-
    Mix(cfg_h, batch, source_h, ciphertexts_h, _mix_number, signer_t);

    // This predicate adds the mix number to the MixSigned predicate,
    // to detect when a mix number has reached all its threshold signatures.
    // This is used above mix action block (mix => mix)
    MixNumberSigned(cfg_h, batch, mix_number, signer_position) <-
    MixSigned(cfg_h, batch, source_h, ciphertexts_h, signer_t),
    Mix(cfg_h, batch, source_h, ciphertexts_h, mix_number, _),
    Ballots(cfg_h, batch, _, _pk_h, trustees),
    // Because threshold selected trustees are not contiguous (there are holes)
    // we need to arrange them into a continuous sequence so that the
    // MixNumbersignedUpTo predicate can detect when the threshold is reached
    // Get the position of trustee that signed the mix (MixSigned) (because trustees is 1-based n + 1)
    let p = trustees.iter().position(|t| *t == signer_t + 1),
    // NULL_TRUSTEE is a dummy value that will not contribute to reaching the threshold
    let signer_position = p.unwrap_or(NULL_TRUSTEE);

    MixNumberSignedUpTo(cfg_h, batch, mix_number, n + 1) <-
    MixNumberSignedUpTo(cfg_h, batch, mix_number, n),
    MixNumberSigned(cfg_h, batch, mix_number, n + 1);

    MixNumberSignedUpTo(cfg_h, batch, mix_number, 0) <-
    MixNumberSigned(cfg_h, batch, mix_number, 0);

    // Extra checks to ensure that all mixing trustees are unique
    MixRepeat(cfg_h, batch) <-
    ConfigurationSignedAll(cfg_h, _, _num_t, _threshold),
    Mix(cfg_h, batch, _, _, mix_number1, signer_t1),
    Mix(cfg_h, batch, _, _, mix_number2, signer_t2),
    (mix_number1 != mix_number2),
    (signer_t1 == signer_t2);

    OutP(Predicate::MixComplete(cfg_h, batch, threshold, ciphertexts_h, signer_t)) <-
    ConfigurationSignedAll(cfg_h, _self_p, _num_t, threshold),
    MixNumberSignedUpTo(cfg_h, batch, threshold, threshold - 1),
    !MixRepeat(cfg_h, batch),
    Mix(cfg_h, batch, _, ciphertexts_h, threshold, signer_t);

    // Fail if not all mixing trustees are unique
    DErr(DatalogError::MixRepeat(cfg_h, batch)) <-
    MixRepeat(cfg_h, batch);

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
