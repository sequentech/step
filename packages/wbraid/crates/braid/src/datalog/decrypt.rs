// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Decryption phase rules (§7).
//!
//! Ported from vs_lift `ascent_logic::decrypt`. The two input-mapping rules are
//! rewritten to destructure our named-field predicate structs
//! ([`PartialDecryptions`](crate::messages::predicate::PartialDecryptions),
//! [`Plaintexts`](crate::messages::predicate::Plaintexts)); the intermediate
//! and protocol rules are ported verbatim; the error rules add three over the
//! original, which hold the trustees to what the public verifier checks:
//! partial decryptions must be of the completed mix chain end, and partial
//! decryptions and plaintexts must be under the ballots public key. The
//! `#[cfg(test)] mod stateright` harness is dropped.

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

        // A partial decryptions input that does not match the completed mix
        // chain end. Combining it would give plaintexts the verifier rejects.
        error(format!("partial decryptions of {:?} by trustee {} are not of the completed mix {:?}", ciphertexts_hash, sender, end_ciphertexts_hash)) <--
            partial_decryptions(cfg_hash, pk_hash, ciphertexts_hash, _, sender),
            mix_complete(cfg_hash, pk_hash, _, end_ciphertexts_hash),
            if ciphertexts_hash != end_ciphertexts_hash;

        // Partial decryptions or plaintexts under a key other than the ballots
        // public key, which the datalog would otherwise ignore.
        error(format!("partial decryptions by trustee {} under public key {:?}, not the ballots key {:?}", sender, pk_hash, ballots_pk_hash)) <--
            partial_decryptions(cfg_hash, pk_hash, _, _, sender),
            ballots(cfg_hash, ballots_pk_hash, _, _),
            if pk_hash != ballots_pk_hash;

        error(format!("plaintexts by trustee {} under public key {:?}, not the ballots key {:?}", sender, pk_hash, ballots_pk_hash)) <--
            plaintexts(cfg_hash, pk_hash, _, _, sender),
            ballots(cfg_hash, ballots_pk_hash, _, _),
            if pk_hash != ballots_pk_hash;

        // Two plaintexts messages with differing plaintexts.
        error(format!("plaintexts mismatch {:?} != {:?} ({} {})", plaintexts_hash1, plaintexts_hash2, sender1, sender2)) <--
            plaintexts(cfg_hash, pk_hash, _, plaintexts_hash1, sender1),
            plaintexts(cfg_hash, pk_hash, _, plaintexts_hash2, sender2),
            if plaintexts_hash1 != plaintexts_hash2;

    }
}

#[cfg(test)]
mod tests {
    use crate::datalog::composed::run;
    use crate::datalog::Action;
    use crate::messages::newtypes::{
        hash_bytes, zero_hash, CiphertextsHash, ConfigurationHash, PartialDecryptionHash,
        PlaintextsHash, PublicKeyHash, TrusteeIndex,
    };
    use crate::messages::predicate::{
        Ballots, ConfigurationValid, Mix, MixSignature, PartialDecryptions, Plaintexts, Predicate,
        PublicKey,
    };

    fn cfg() -> ConfigurationHash {
        ConfigurationHash(zero_hash())
    }

    /// The public key the ballots name.
    fn ballots_key() -> PublicKeyHash {
        PublicKeyHash(hash_bytes(b"pk"))
    }

    fn other_key() -> PublicKeyHash {
        PublicKeyHash(hash_bytes(b"other pk"))
    }

    fn ciphertexts(label: &[u8]) -> CiphertextsHash {
        CiphertextsHash(hash_bytes(label))
    }

    /// Trustee 1 of 3 (threshold 2), with the public key every trustee
    /// posted, the ballots `M0` for trustees 1 then 2, and their completed mix
    /// chain M0 -> M1 -> M2, each mix signed by the other trustee.
    fn completed_mix() -> Vec<Predicate> {
        let mut predicates = vec![Predicate::ConfigurationValid(ConfigurationValid {
            configuration: cfg(),
            threshold: 2,
            trustee_count: 3,
            self_index: 1,
        })];
        for sender in 1..=3 {
            predicates.push(Predicate::PublicKey(PublicKey {
                configuration: cfg(),
                public_key: ballots_key(),
                sender,
            }));
        }
        predicates.push(Predicate::Ballots(Ballots {
            configuration: cfg(),
            public_key: ballots_key(),
            ciphertexts: ciphertexts(b"M0"),
            trustees: vec![1, 2],
            tally_id: 1,
        }));
        for (input, output, mixer, signer) in [(b"M0", b"M1", 1, 2), (b"M1", b"M2", 2, 1)] {
            predicates.push(Predicate::Mix(Mix {
                configuration: cfg(),
                public_key: ballots_key(),
                input: ciphertexts(input),
                output: ciphertexts(output),
                sender: mixer,
            }));
            predicates.push(Predicate::MixSignature(MixSignature {
                configuration: cfg(),
                public_key: ballots_key(),
                input: ciphertexts(input),
                output: ciphertexts(output),
                sender: signer,
            }));
        }
        predicates
    }

    /// Trustee `sender`'s partial decryptions of the ciphertexts `of`, under
    /// `key`.
    fn partial_decryptions(sender: TrusteeIndex, of: &[u8], key: PublicKeyHash) -> Predicate {
        Predicate::PartialDecryptions(PartialDecryptions {
            configuration: cfg(),
            public_key: key,
            ciphertexts: ciphertexts(of),
            decryptions: PartialDecryptionHash(hash_bytes(format!("d{sender}").as_bytes())),
            sender,
        })
    }

    /// Trustee `sender`'s plaintexts of the completed mix, under `key`.
    fn plaintexts(sender: TrusteeIndex, key: PublicKeyHash) -> Predicate {
        Predicate::Plaintexts(Plaintexts {
            configuration: cfg(),
            public_key: key,
            ciphertexts: ciphertexts(b"M2"),
            plaintexts: PlaintextsHash(hash_bytes(b"plaintexts")),
            sender,
        })
    }

    /// Partial decryptions of the completed mix under the ballots key are
    /// combined, next to another trustee's plaintexts of it.
    #[test]
    fn partial_decryptions_of_the_completed_mix_are_combined() {
        let mut predicates = completed_mix();
        predicates.push(partial_decryptions(1, b"M2", ballots_key()));
        predicates.push(partial_decryptions(2, b"M2", ballots_key()));
        predicates.push(plaintexts(2, ballots_key()));

        let actions = run(&predicates).expect("a consistent tally must not error");

        assert!(
            actions.iter().any(|a| matches!(
                a,
                Action::ComputePlaintexts(_, _, c, _, 1) if *c == ciphertexts(b"M2")
            )),
            "expected ComputePlaintexts of the completed mix, got {:?}",
            actions
        );
    }

    /// Partial decryptions that name other ciphertexts than the completed mix
    /// halt the tally; they used to be combined into plaintexts that the
    /// public verifier rejects.
    #[test]
    fn partial_decryptions_of_other_ciphertexts_halt() {
        let mut predicates = completed_mix();
        predicates.push(partial_decryptions(1, b"M2", ballots_key()));
        predicates.push(partial_decryptions(2, b"M1", ballots_key()));

        let err = run(&predicates).expect_err("must error");

        assert!(
            err.contains("by trustee 2 are not of the completed mix"),
            "{err}"
        );
    }

    /// Partial decryptions or plaintexts under another public key halt the
    /// tally; the datalog used to ignore them, and the verifier rejects them.
    #[test]
    fn statements_under_another_key_halt() {
        for (statement, expected) in [
            (
                partial_decryptions(2, b"M2", other_key()),
                "partial decryptions by trustee 2 under public key",
            ),
            (
                plaintexts(2, other_key()),
                "plaintexts by trustee 2 under public key",
            ),
        ] {
            let mut predicates = completed_mix();
            predicates.push(partial_decryptions(1, b"M2", ballots_key()));
            predicates.push(statement);

            let err = run(&predicates).expect_err("must error");

            assert!(err.contains(expected), "{err}");
        }
    }
}
