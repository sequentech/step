// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Decryption phase rules (§7).
//!
//! Ported from vs_lift `ascent_logic::decrypt`. The two input-mapping rules are
//! rewritten to destructure our named-field predicate structs
//! ([`PartialDecryptions`](crate::messages::predicate::PartialDecryptions),
//! [`Plaintexts`](crate::messages::predicate::Plaintexts)); the intermediate,
//! protocol, and error rules are ported verbatim. The `#[cfg(test)] mod
//! stateright` harness is dropped.

/// Ascent inference rules for the decryption phase, as a reusable source template.
pub mod infer {

    ascent::ascent_source! { decrypt_infer:

        // Input relations /////////////////////////////////////////

        // The given trustee computed the given partial decryptions of the input
        // ciphertexts, for the configuration and public key context.
        relation partial_decryptions(CfgHash, PublicKeyHash, CiphertextsHash, PartialDecryptionsHash, Sender);
        // The given trustee verified all partial decryptions and combined them
        // into the given plaintexts, for the configuration and public key context.
        relation plaintexts(CfgHash, PublicKeyHash, CiphertextsHash, PlaintextsHash, Sender);

        // Predicate -> input relation mappings.

        partial_decryptions(pd.configuration, pd.public_key, pd.ciphertexts, pd.decryptions, pd.sender) <--
            predicate(p),
            if let Predicate::PartialDecryptions(pd) = p;

        plaintexts(pt.configuration, pt.public_key, pt.ciphertexts, pt.plaintexts, pt.sender) <--
            predicate(p),
            if let Predicate::Plaintexts(pt) = p;

        // Intermediate relations //////////////////////////////////

        // Partial decryptions accumulated up to the given trustee index.
        // Correspondence is by AccumulatorSet index; validity is not asserted.
        relation partial_decryptions_acc(CfgHash, PartialDecryptionsHashesAcc, Sender);
        // The accumulated partial decryptions constitute all necessary values.
        relation partial_decryptions_all(CfgHash, PartialDecryptionsHashesAcc);

        // Protocol rules //////////////////////////////////////////

        // Compute partial decryptions if the trustee accepts the configuration,
        // is a participant, the mix chain is complete from the ballots to the
        // given ciphertexts, all DKG shares are in, and it has not yet posted its
        // partial decryptions. The accumulated shares hashes are carried in the
        // action so it explicitly names every input the trustee decrypts its own
        // share from (as `ComputePublicKey` does).
        action(Action::ComputePartialDecryptions(*cfg_hash, *pk_hash, *out_ciphertexts_hash, shares.extract(), *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            ballots(cfg_hash, pk_hash, ciphertexts_hash, mixing_trustees),
            mix_complete(cfg_hash, pk_hash, ciphertexts_hash, out_ciphertexts_hash),
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, _, self_index),
            shares_all(cfg_hash, shares),
            !partial_decryptions(cfg_hash, pk_hash, _, _, self_index);

        // Partial decryptions up to trustee 1 if we have position 1's value.
        partial_decryptions_acc(cfg_hash, AccumulatorSet::new(*partial_decryptions), 1) <--
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, 1, trustee),
            partial_decryptions(cfg_hash, pk_hash, _, partial_decryptions, trustee);

        // Partial decryptions up to trustee n if we have them up to n - 1 and
        // trustee n's value.
        partial_decryptions_acc(cfg_hash, partial_decryptions_hashes.add(*partial_decryptions, *position), position) <--
            mixing_position(cfg_hash, pk_hash, ciphertexts_hash, position, trustee),
            partial_decryptions_acc(cfg_hash, partial_decryptions_hashes, position - 1),
            partial_decryptions(cfg_hash, pk_hash, _, partial_decryptions, trustee);

        // All partial decryptions received once we have them up to the threshold.
        partial_decryptions_all(cfg_hash, partial_decryptions) <--
            partial_decryptions_acc(cfg_hash, partial_decryptions, threshold),
            configuration_valid(cfg_hash, threshold, _, self_index);

        // Compute plaintexts if the trustee accepts the configuration, is a
        // participant, all partial decryptions are in, the mix chain ends at the
        // given ciphertexts, and it has not yet posted its plaintexts.
        action(Action::ComputePlaintexts(*cfg_hash, *pk_hash, *ciphertexts_hash, partial_decryptions.extract(), *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            // comment this line out to have all trustees, not just the mixing
            // participants, compute the plaintexts
            mixing_position(cfg_hash, pk_hash, _, _, self_index),
            partial_decryptions_all(cfg_hash, partial_decryptions),
            mix_complete(cfg_hash, pk_hash, _, ciphertexts_hash),
            !plaintexts(cfg_hash, pk_hash, ciphertexts_hash, _, self_index);

        // Errors //////////////////////////////////////////////////

        // Two plaintexts messages with differing input ciphertexts.
        error(format!("ciphertexts mismatch {:?} != {:?} ({} {})", ciphertexts_hash1, ciphertexts_hash2, sender1, sender2)) <--
            plaintexts(cfg_hash, pk_hash, ciphertexts_hash1, _, sender1),
            plaintexts(cfg_hash, pk_hash, ciphertexts_hash2, _, sender2),
            if ciphertexts_hash1 != ciphertexts_hash2;

        // A plaintexts input that does not match the completed mix chain end.
        error(format!("unexpected input ciphertexts {:?} != {:?}", ciphertexts_hash1, end_ciphertexts_hash)) <--
            plaintexts(cfg_hash, pk_hash, ciphertexts_hash1, _, sender1),
            mix_complete(cfg_hash, pk_hash, _, end_ciphertexts_hash),
            if ciphertexts_hash1 != end_ciphertexts_hash;

        // Two plaintexts messages with differing plaintexts.
        error(format!("plaintexts mismatch {:?} != {:?} ({} {})", plaintexts_hash1, plaintexts_hash2, sender1, sender2)) <--
            plaintexts(cfg_hash, pk_hash, _, plaintexts_hash1, sender1),
            plaintexts(cfg_hash, pk_hash, _, plaintexts_hash2, sender2),
            if plaintexts_hash1 != plaintexts_hash2;

    }
}
