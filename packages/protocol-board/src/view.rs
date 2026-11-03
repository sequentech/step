// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a DKG board says, read as the platform needs it: how far each trustee
//! has got, and the joint public key once every trustee has published the same
//! one.
//!
//! braid has verified every message by the time this runs. Judging whether
//! the protocol halted is the trustees' job; this reading only refuses to take
//! a key the trustees do not agree on.

use anyhow::anyhow;
use cryptography::utils::serialization::Deserializable;
use sequent_core::types::ceremonies::{
    KeysCeremonyFailureReason, TrusteeStatus,
};
use wbraid::board::store::MessageStore;
use wbraid::messages::artifact::DkgPublicKey;
use wbraid::messages::newtypes::{
    ConfigurationHash, PublicKeyHash, TrusteeIndex,
};
use wbraid::messages::predicate::Predicate;

use crate::encoding::{encode_joint_public_key, HashHex};
use crate::Ctx;

/// Where the key generation stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DkgStatus {
    InProgress,
    /// Every trustee published the same joint public key.
    Completed {
        /// The key in the form the ceremony status and the ballot styles carry.
        joint_public_key: String,
        /// The hash of the `PublicKey` message that carried it, which every
        /// tally of this key names.
        public_key_hash: HashHex,
    },
    /// No joint public key will ever be taken from this board: it serves
    /// another Configuration, braid refused what it carries, or the trustees
    /// did not publish one single readable key.
    Unusable {
        reason: KeysCeremonyFailureReason,
        detail: String,
    },
}

/// One reading of a DKG board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DkgView {
    /// Each trustee's phase in `Configuration` order: the trustee with
    /// protocol index `i` is at position `i - 1`. Empty when the board could
    /// not be read at all.
    pub trustee_statuses: Vec<TrusteeStatus>,
    pub status: DkgStatus,
    _private: (),
}

impl DkgView {
    /// Read a board that braid has already verified, then derive the current status.
    ///
    /// **Pre-condition**: the configuration hash must be checked with the store to
    /// match the expectations. This makes no further checks.
    pub(crate) fn from_verified(
        configuration: &ConfigurationHash,
        store: &MessageStore<Ctx>,
    ) -> DkgView {
        let mut dealers: Vec<TrusteeIndex> = Vec::new();
        let mut public_keys: Vec<(TrusteeIndex, PublicKeyHash)> = Vec::new();

        // Collect per-trustee status by checking the indicative messages.
        for predicate in store.get_predicates() {
            // Messages naming another `Configuration` are not this run's and are left out.
            // It is likely a bug, because the store (per-board) should already be scoped
            // into a specific config.
            if predicate.get_configuration() != *configuration {
                tracing::warn!(
                    predicate=?predicate,
                    "Skipping predicate with mismatching configuration"
                );
                continue;
            }
            match predicate {
                Predicate::Shares(shares) => dealers.push(shares.sender),
                Predicate::PublicKey(public_key) => {
                    public_keys.push((public_key.sender, public_key.public_key))
                }
                _ => {}
            }
        }

        let phases = (1..=store.configuration().trustees.len())
            .map(|index| {
                if public_keys.iter().any(|(sender, _)| *sender == index) {
                    TrusteeStatus::KEY_GENERATED
                } else if dealers.contains(&index) {
                    TrusteeStatus::SHARES_POSTED
                } else {
                    TrusteeStatus::WAITING
                }
            })
            .collect::<Vec<_>>();

        let outcome = joint_public_key(store, &public_keys, &phases);

        DkgView {
            trustee_statuses: phases,
            status: outcome,
            _private: (),
        }
    }

    /// A board that says nothing about the trustees because it cannot be
    /// used at all.
    pub(crate) fn unusable(
        reason: KeysCeremonyFailureReason,
        detail: String,
    ) -> DkgView {
        DkgView {
            trustee_statuses: Vec::new(),
            status: DkgStatus::Unusable { reason, detail },
            _private: (),
        }
    }
}

/// The joint public key, once every trustee has published the same one.
///
/// Two published keys that differ are final as soon as they are both on the
/// board: the trustees halt on the disagreement and the missing keys never arrive.
fn joint_public_key(
    store: &MessageStore<Ctx>,
    public_keys: &[(TrusteeIndex, PublicKeyHash)],
    phases: &[TrusteeStatus],
) -> DkgStatus {
    // Take the first public key published, then check if there are any
    // disagreements, if so, the ceremony has failed.
    let Some((_, hash)) = public_keys.first() else {
        return DkgStatus::InProgress;
    };
    let public_key_hash = HashHex::of(&hash.0);
    if public_keys.iter().any(|(_, other)| other != hash) {
        let published = public_keys
            .iter()
            .map(|(sender, key)| {
                format!("trustee {sender}: {}", HashHex::of(&key.0).short())
            })
            .collect::<Vec<_>>()
            .join(", ");
        return DkgStatus::Unusable {
            reason: KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
            detail: format!("the trustees published different joint public keys ({published})"),
        };
    }

    // If any of the trustees are yet to complete, we're still in progress, can't conclude.
    if phases
        .iter()
        .any(|phase| *phase != TrusteeStatus::KEY_GENERATED)
    {
        return DkgStatus::InProgress;
    }

    // Make sure that the store and the derived hash are correct, then extract the joint key.
    let Some(body) = store.public_key_body(hash) else {
        return DkgStatus::Unusable {
            reason: KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
            detail: format!(
                "the board holds no body for public key {}",
                public_key_hash.short()
            ),
        };
    };
    match DkgPublicKey::<Ctx>::deser(body) {
        Ok(key) => DkgStatus::Completed {
            joint_public_key: encode_joint_public_key(&key.pk),
            public_key_hash,
        },
        Err(err) => DkgStatus::Unusable {
            reason: KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
            detail: format!(
                "public key {} cannot be deserialized as a DKG public key: {err}",
                public_key_hash.short()
            ),
        },
    }
}

#[cfg(test)]
mod test_utils {
    use crate::{DkgStatus, DkgView};
    use sequent_core::types::ceremonies::TrusteeStatus;

    impl DkgView {
        pub(crate) fn new_unchecked(
            trustees_statuses: &[TrusteeStatus],
            status: DkgStatus,
        ) -> DkgView {
            DkgView {
                trustee_statuses: trustees_statuses.to_vec(),
                status,
                _private: (),
            }
        }
    }
}
