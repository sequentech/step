// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Mixing phase rules (§7).
//!
//! Ported from vs_lift `ascent_logic::mix`. The three input-mapping rules are
//! rewritten to destructure our named-field predicate structs
//! ([`Ballots`](crate::messages::predicate::Ballots),
//! [`Mix`](crate::messages::predicate::Mix),
//! [`MixSignature`](crate::messages::predicate::MixSignature)); the
//! intermediate rules are ported verbatim; the mix/sign protocol rules
//! additionally tag their actions with a [`MixSource`](super::MixSource),
//! splitting `SignMix` by input provenance (see below); the error rules add
//! two rules over the original: the ballots mixing-trustee list must be
//! exactly threshold-sized, and its entries must name existing trustees
//! (1..=trustee_count). The mixing-set is a `Vec<TrusteeIndex>` (not a fixed
//! array) so the rules can recursively unpack it into per-position facts. The
//! `#[cfg(test)] mod stateright` harness is dropped.

/// Ascent inference rules for the mixing phase, as a reusable source template.
pub mod infer {

    ascent::ascent_source! { mix_infer:

        // Input relations /////////////////////////////////////////

        // The given ciphertexts are valid input ballots, mixed by the given
        // ordered set of trustees, for the configuration and public key context.
        relation ballots(CfgHash, PublicKeyHash, CiphertextsHash, Vec<TrusteeIndex>);
        // The given trustee mixed the input ciphertexts into the output
        // ciphertexts, for the configuration and public key context.
        relation mix(CfgHash, PublicKeyHash, /* input hash */ CiphertextsHash, /* output hash */ CiphertextsHash, Sender);
        // The given trustee verified the mix (input -> output ciphertexts), for
        // the configuration and public key context.
        relation mix_signature(CfgHash, PublicKeyHash, /* input hash */ CiphertextsHash, /* output hash */ CiphertextsHash, Sender);

        // Predicate -> input relation mappings.

        ballots(b.configuration, b.public_key, b.ciphertexts, b.trustees.clone()) <--
            predicate(p),
            if let Predicate::Ballots(b) = p;

        mix(mx.configuration, mx.public_key, mx.input, mx.output, mx.sender) <--
            predicate(p),
            if let Predicate::Mix(mx) = p;

        mix_signature(ms.configuration, ms.public_key, ms.input, ms.output, ms.sender) <--
            predicate(p),
            if let Predicate::MixSignature(ms) = p;

        // Intermediate relations //////////////////////////////////

        // The mixing position of the trustee at the given index is the head of
        // the given trustee list, as defined by the ballots message.
        relation mixing_position_acc(CfgHash, PublicKeyHash, CiphertextsHash, TrusteeIndex, Vec<TrusteeIndex>);
        // The given trustee is assigned to the given mixing position.
        relation mixing_position(CfgHash, PublicKeyHash, CiphertextsHash, /* mixing position */ TrusteeIndex, /* trustee */ TrusteeIndex);
        // The mix (input -> output) has been verified by all participating
        // trustees up to the given position (indices per the ballots list).
        relation mix_signatures_acc(CfgHash, PublicKeyHash, CiphertextsHash, CiphertextsHash, Sender);
        // The mix (input -> output) has been verified by all participating trustees.
        relation mix_signatures_all(CfgHash, PublicKeyHash, CiphertextsHash, CiphertextsHash);
        // The mix chain with the given endpoints extends up to the given
        // position: it starts at the input ciphertexts, ends at the output
        // ciphertexts, each intermediate mix is by the assigned trustee, and
        // each mix is verified by all participating trustees.
        relation mix_chain(CfgHash, PublicKeyHash, CiphertextsHash, CiphertextsHash, Sender);
        // The mix chain with the given input ciphertexts is complete: it extends
        // up to the last participant position (threshold).
        relation mix_complete(CfgHash, PublicKeyHash, CiphertextsHash, CiphertextsHash);

        // Protocol rules //////////////////////////////////////////

        // Initialize the mixing positions from the ballots.
        mixing_position_acc(cfg_hash, pk_hash, ciphertexts_hash, 0, mixing_trustees) <--
            ballots(cfg_hash, pk_hash, ciphertexts_hash, mixing_trustees)
            if mixing_trustees.len() > 0;

        // Unpack the mixing positions from the ballots one by one.
        mixing_position_acc(cfg_hash, pk_hash, ciphertexts_hash, index + 1, mixing_trustees[1..].to_vec()) <--
            mixing_position_acc(cfg_hash, pk_hash, ciphertexts_hash, index, mixing_trustees),
            if mixing_trustees.len() > 1;

        // The nth trustee in the ballots message is assigned to position n.
        mixing_position(cfg_hash, pk_hash, ciphertexts_hash, index + 1, mixing_trustees[0]) <--
            mixing_position_acc(cfg_hash, pk_hash, ciphertexts_hash, index, mixing_trustees);

        // Compute a mix of the ballot ciphertexts if the trustee accepts the
        // configuration, its position is 1, the ciphertexts are the ballots, and
        // it has not yet posted its mix.
        action(Action::ComputeMix(*cfg_hash, *pk_hash, MixSource::Ballots, *ciphertexts_hash, *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, 1, self_index),
            public_keys_all(cfg_hash, pk_hash),
            ballots(cfg_hash, pk_hash, ciphertexts_hash, _),
            !mix(cfg_hash, pk_hash, ciphertexts_hash, _, self_index);

        // Compute a mix of the previous output if the trustee accepts the
        // configuration, is assigned the given position, the given ciphertexts
        // are the previous position's output, all participants signed the
        // previous mix, and it has not yet posted its mix.
        action(Action::ComputeMix(*cfg_hash, *pk_hash, MixSource::PriorMix, *out_ciphertexts_hash, *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            public_keys_all(cfg_hash, pk_hash),
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, position, self_index),
            mix(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, previous),
            // only selected trustees compute mixes
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, position - 1, previous),
            mix_signatures_all(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash),
            !mix(cfg_hash, pk_hash, out_ciphertexts_hash, _, self_index);

        // Mix signatures up to trustee 1 if we have position 1's signature.
        mix_signatures_acc(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, 1) <--
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, 1, trustee),
            mix_signature(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, trustee);

        // Mix signatures up to position n if we have them up to n - 1 and
        // position n's signature.
        mix_signatures_acc(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, position) <--
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, position, trustee),
            mix_signature(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, trustee),
            mix_signatures_acc(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, position - 1);

        // All mix signatures received once we have them up to the threshold.
        mix_signatures_all(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash) <--
            configuration_valid(cfg_hash, threshold, _, _),
            mix_signatures_acc(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, threshold);

        // A mix computed by a trustee is already a signature for that mix.
        mix_signature(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, self_index) <--
             mix(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, self_index);

        // Verify a mix if the trustee accepts the configuration, is a
        // participant, the mix has been computed, and it has not yet signed it.
        // Split by the mix's input provenance so the action names the source
        // (`Ballots` for the first mix, `PriorMix` otherwise) the signer must
        // fetch to re-derive the shuffle context; the two rules are mutually
        // exclusive (a mix input equal to both the ballots and a prior output
        // would be a hash collision, already caught by the chain-endpoint errors).

        // First mix: its input is the ballots ciphertexts.
        action(Action::SignMix(*cfg_hash, *pk_hash, MixSource::Ballots, *in_ciphertexts_hash, *out_ciphertexts_hash, *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            public_keys_all(cfg_hash, pk_hash),
            // only selected trustees sign mixes
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, _, self_index),
            mix(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, _),
            ballots(cfg_hash, pk_hash, in_ciphertexts_hash, _),
            !mix_signature(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, self_index);

        // Later mix: its input is a previous mixer's output.
        action(Action::SignMix(*cfg_hash, *pk_hash, MixSource::PriorMix, *in_ciphertexts_hash, *out_ciphertexts_hash, *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            public_keys_all(cfg_hash, pk_hash),
            // only selected trustees sign mixes
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, _, self_index),
            mix(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, _),
            mix(cfg_hash, pk_hash, _, in_ciphertexts_hash, _),
            !mix_signature(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, self_index);

        // The mix chain extends to position 1 if the position-1 trustee mixed
        // the ballot ciphertexts and all participants signed that mix.
        mix_chain(cfg_hash, pk_hash, ciphertexts_hash, out_ciphertexts_hash, 1) <--
            public_keys_all(cfg_hash, pk_hash),
            ballots(cfg_hash, pk_hash, ciphertexts_hash, mixing_trustees),
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, 1, trustee),
            mix(cfg_hash, pk_hash, ciphertexts_hash, out_ciphertexts_hash, trustee),
            mix_signatures_all(cfg_hash, pk_hash, ciphertexts_hash, out_ciphertexts_hash);

        // The mix chain extends to position2 if it extends to position1, the
        // position2 trustee mixed position1's output, all participants signed
        // it, and position2 follows position1.
        mix_chain(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash2, position2) <--
            mix_chain(cfg_hash, pk_hash, in_ciphertexts_hash, out_ciphertexts_hash, position1),
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, position2, trustee),
            mix(cfg_hash, pk_hash, out_ciphertexts_hash, out_ciphertexts_hash2, trustee),
            mix_signatures_all(cfg_hash, pk_hash, out_ciphertexts_hash, out_ciphertexts_hash2),
            if *position2 == position1 + 1;

        // The mix is complete if the chain extends to the threshold, has the
        // given endpoints, and starts at the ballot ciphertexts.
        mix_complete(cfg_hash, pk_hash, ciphertexts_hash, out_ciphertexts_hash) <--
            configuration_valid(cfg_hash, threshold, _, _),
            public_keys_all(cfg_hash, pk_hash),
            ballots(cfg_hash, pk_hash, ciphertexts_hash, mixing_trustees),
            mix_chain(cfg_hash, pk_hash, ciphertexts_hash, out_ciphertexts_hash, threshold);

        // Errors //////////////////////////////////////////////////

        // A mixing-trustee list whose size is not exactly the threshold. The
        // mixing set is the decryption quorum: the chain completes at position
        // threshold (mix_complete), so a shorter list can never complete and a
        // longer one would strand mixes beyond the completed chain.
        error(format!("mixing set size {} does not match threshold {}", mixing_trustees.len(), threshold)) <--
            configuration_valid(cfg_hash, threshold, _, _),
            ballots(cfg_hash, _, _, mixing_trustees),
            if mixing_trustees.len() != *threshold;

        // A mixing-trustee index that names no existing trustee (indices are
        // 1-based, §4.3). Without this check a nonexistent index would stall
        // the mix chain forever instead of halting.
        error(format!("mixing trustee index {} out of range 1..={}", trustee, trustee_count)) <--
            configuration_valid(cfg_hash, _, trustee_count, _),
            mixing_position(cfg_hash, _, _, _, trustee),
            if *trustee == 0 || *trustee > *trustee_count;

        // A trustee assigned to two different mixing positions.
        error(format!("Multiple mixing positions for trustee {:?}: {:?}, {:?}", trustee, position1, position2)) <--
            mixing_position(cfg_hash, pk_hash, _, position1, trustee),
            mixing_position(cfg_hash, pk_hash, _, position2, trustee),
            if position1 != position2;

        // A mix chain end reached at two different positions.
        error(format!("Repeated mix chain end for lengths {:?}, {:?}", position1, position2)) <--
            mix_chain(cfg_hash, pk_hash, _, out_ciphertexts_hash, position1),
            mix_chain(cfg_hash, pk_hash, _, out_ciphertexts_hash, position2),
            if position1 != position2;

        // A mix chain that does not start at the ballot ciphertexts.
        error(format!("Non-ballots mix chain start {:?}", in_ciphertexts_hash)) <--
            mix_chain(cfg_hash, pk_hash, in_ciphertexts_hash, _, _),
            ballots(cfg_hash, pk_hash, ciphertexts_hash, mixing_trustees),
            if in_ciphertexts_hash != ciphertexts_hash;

        // A mix chain whose end matches the ballot ciphertexts.
        error(format!("Unexpected ballots ciphertexts in chain end")) <--
            mix_chain(cfg_hash, pk_hash, _, out_ciphertexts_hash, _),
            ballots(cfg_hash, pk_hash, ciphertexts_hash, mixing_trustees),
            if out_ciphertexts_hash == ciphertexts_hash;

        // Two trustees computed a mix with the same input.
        error(format!("Multiple trustees with mix input, {:?}, {:?}", sender1, sender2)) <--
            mix(cfg_hash, pk_hash, in_ciphertexts_hash, _, sender1),
            mix(cfg_hash, pk_hash, in_ciphertexts_hash, _, sender2),
            if sender1 != sender2;

        // Two trustees computed a mix with the same output.
        error(format!("Multiple trustees with mix output, {:?}, {:?}", sender1, sender2)) <--
            mix(cfg_hash, pk_hash, _, out_ciphertexts_hash, sender1),
            mix(cfg_hash, pk_hash, _, out_ciphertexts_hash, sender2),
            if sender1 != sender2;

        // The same trustee computed two different mixes.
        error(format!("Repeated trustee in mix chain {:?}", sender1)) <--
            mix(cfg_hash, pk_hash, in_ciphertexts_hash1, out_ciphertexts_hash1, sender1),
            mix(cfg_hash, pk_hash, in_ciphertexts_hash2, out_ciphertexts_hash2, sender2),
            if sender1 == sender2,
            if in_ciphertexts_hash1 != in_ciphertexts_hash2 || out_ciphertexts_hash1 != out_ciphertexts_hash2;

        // Two chained mixes whose trustees are not at consecutive positions.
        error(format!("Non-consecutive mix chain participants {:?}, {:?}", sender1, sender2)) <--
            mix(cfg_hash, pk_hash, _, out_ciphertexts_hash, sender1),
            mix(cfg_hash, pk_hash, out_ciphertexts_hash, _, sender2),
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, index1, sender1),
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, index2, sender2),
            if index1 + 1 != *index2;
    }
}
