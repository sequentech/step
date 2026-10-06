// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Actions the datalog engine derives (§7.5): the boundary to the action/crypto
//! layer.

use super::types::*;

/// Where a mix's input ciphertexts come from (§8). The datalog — which knows the
/// mixing position — tags each mix action with its source so the trustee fetches
/// from the correct store directly, instead of probing both. Because the store
/// accessors are content-addressed by the action's input hash, naming the wrong
/// source simply yields no body (an explicit error), which doubles as a sanity
/// check that the source and the input hash agree.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MixSource {
    /// The manager's `Ballots` ciphertexts — the first mixer's input.
    Ballots,
    /// A previous mixer's `Mix` output ciphertexts — a later mixer's input.
    PriorMix,
}

/// Actions a trustee can take during protocol execution.
///
/// Each variant corresponds to a computation the trustee must perform; they are
/// derived by the ascent rules and consumed by the action layer, which performs
/// the underlying cryptography and posts the resulting message to the board,
/// advancing the protocol.
///
/// Unlike the vs_lift original this does **not** derive `Ord`: ascent relations
/// only require `Clone + Eq + Hash`, and dropping `Ord` avoids imposing an
/// ordering requirement on the configuration/public-key/ciphertexts hashes that
/// appear as fields (only the *accumulated* hashes need `Ord`, for the
/// [`AccumulatorSet`](super::accumulator::AccumulatorSet)).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Compute and post this trustee's DKG shares.
    ComputeShares(CfgHash, TrusteeIndex),
    /// Compute and post this trustee's view of the joint public key.
    ComputePublicKey(CfgHash, SharesHashes, TrusteeIndex),
    /// Compute and post a mix of the input ciphertexts drawn from `MixSource`.
    ComputeMix(
        CfgHash,
        PublicKeyHash,
        MixSource,
        CiphertextsHash,
        TrusteeIndex,
    ),
    /// Verify and sign a mix (`input` -> `output`); `input` is drawn from `MixSource`.
    SignMix(
        CfgHash,
        PublicKeyHash,
        MixSource,
        CiphertextsHash,
        CiphertextsHash,
        TrusteeIndex,
    ),
    /// Compute and post partial decryptions of the given ciphertexts.
    ///
    /// Carries the accumulated DKG shares hashes explicitly (like
    /// [`Action::ComputePublicKey`]) so the action is a self-contained,
    /// hash-bound description of every input the trustee decrypts its own share
    /// from, even though they are also recoverable from the message store.
    ComputePartialDecryptions(
        CfgHash,
        PublicKeyHash,
        CiphertextsHash,
        SharesHashes,
        TrusteeIndex,
    ),
    /// Combine partial decryptions into plaintexts and post them.
    ComputePlaintexts(
        CfgHash,
        PublicKeyHash,
        CiphertextsHash,
        PartialDecryptionsHashes,
        TrusteeIndex,
    ),
}
