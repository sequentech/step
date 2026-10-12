// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Distributed key generation (DKG) phase rules (§7).
//!
//! Ported from vs_lift `ascent_logic::dkg`. The two input-mapping rules are
//! rewritten to destructure our named-field predicate structs
//! ([`Shares`](crate::messages::predicate::Shares),
//! [`PublicKey`](crate::messages::predicate::PublicKey)); the intermediate,
//! protocol, and error rules are ported verbatim (they reference relations, not
//! the predicate enum). The `#[cfg(test)] mod stateright` harness is dropped.

/// Ascent inference rules for the DKG phase, as a reusable source template.
pub mod infer {

    ascent::ascent_source! { dkg_infer:

        // Input relations /////////////////////////////////////////

        // The given trustee has computed the given shares for the configuration context.
        relation shares(CfgHash, TrusteeSharesHash, Sender);
        // The given trustee has computed the given public key (from validated
        // shares) for the configuration context.
        relation public_key(CfgHash, PublicKeyHash, Sender);

        // Predicate -> input relation mappings.

        shares(s.configuration, s.shares, s.sender) <--
            predicate(p),
            if let Predicate::Shares(s) = p;

        public_key(pk.configuration, pk.public_key, pk.sender) <--
            predicate(p),
            if let Predicate::PublicKey(pk) = p;

        // Intermediate relations //////////////////////////////////

        // Shares for the given trustees have been accumulated up to the given
        // trustee index. Share-trustee correspondence is by AccumulatorSet index.
        // This does not assert share validity.
        relation shares_acc(CfgHash, SharesHashesAcc, Sender);
        // The accumulated shares constitute all required shares for the context.
        relation shares_all(CfgHash, SharesHashesAcc);
        // Public key values have been accumulated, from verified shares, up to
        // the given trustee index. In a valid DKG all trustees compute the same
        // public key. This does not assert public key validity.
        relation public_keys_acc(CfgHash, PublicKeyHash, Sender);
        // The accumulated public key values constitute all necessary values.
        relation public_keys_all(CfgHash, PublicKeyHash);

        // Protocol rules //////////////////////////////////////////

        // Compute shares if the trustee accepts the configuration and has not
        // yet posted its shares.
        action(Action::ComputeShares(*cfg_hash, *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            !shares(cfg_hash, _, self_index);

        // We have shares up to trustee 1 if we have the shares from trustee 1.
        shares_acc(cfg_hash, AccumulatorSet::new(*shares), 1) <--
            shares(cfg_hash, shares, 1);

        // We have shares up to trustee n if we have shares up to trustee n - 1
        // and the shares from trustee n.
        shares_acc(cfg_hash, shares_hashes.add(*shares, *sender), sender) <--
            shares(cfg_hash, shares, sender),
            shares_acc(cfg_hash, shares_hashes, sender - 1);

        // All shares received once we have shares up to trustee_count.
        shares_all(cfg_hash, shares) <--
            configuration_valid(cfg_hash, _, trustee_count, self_index),
            shares_acc(cfg_hash, shares, trustee_count);

        // Compute the public key if the trustee accepts the configuration, all
        // shares are in, and it has not yet posted its public key.
        action(Action::ComputePublicKey(*cfg_hash, shares.extract(), *self_index)) <--
            configuration_valid(cfg_hash, _, _, self_index),
            shares_all(cfg_hash, shares),
            !public_key(cfg_hash, _, self_index);

        // We have public key values up to trustee 1 if we have trustee 1's value.
        public_keys_acc(cfg_hash, pk_hash, 1) <--
            public_key(cfg_hash, pk_hash, 1);

        // We have public key values up to trustee n if we have values up to
        // trustee n - 1 and the value from trustee n.
        public_keys_acc(cfg_hash, pk_hash, sender) <--
            public_key(cfg_hash, pk_hash, sender),
            public_keys_acc(cfg_hash, pk_hash, sender - 1);

        // All public key values received once we have values up to trustee_count.
        public_keys_all(cfg_hash, pk_hash) <--
            public_keys_acc(cfg_hash, pk_hash, trustee_count),
            configuration_valid(cfg_hash, _, trustee_count, self_index);

        // Errors //////////////////////////////////////////////////

        // A public-key mismatch: two trustees computed differing public keys.
        error(format!("pk mismatch {:?} != {:?} ({} {})", pk_hash1, pk_hash2, sender1, sender2)) <--
            public_key(cfg_hash, pk_hash1, sender1),
            public_key(cfg_hash, pk_hash2, sender2),
            if pk_hash1 != pk_hash2;

    }
}
