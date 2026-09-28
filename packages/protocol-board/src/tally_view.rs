// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a tally board says, read as the platform needs it: how far the quorum
//! has got, and the decrypted payloads once the quorum agrees on them.
//!
//! braid has verified every message against the parent's Configuration by the
//! time this runs. Judging whether the protocol halted is the trustees' job;
//! this reading counts phases and refuses only what the platform cannot use:
//! a `Ballots` message other than the board's own, which every trustee halts
//! on, and plaintexts the quorum does not agree on or that cannot be read.

use std::collections::BTreeSet;

use anyhow::{Context as _, Result};
use cryptography::utils::serialization::Deserializable;
use sequent_core::types::ceremonies::{TallyBoardPhase, TallyFailureReason};
use wbraid::board::store::MessageStore;
use wbraid::board::verify::verify;
use wbraid::messages::artifact::Plaintexts;
use wbraid::messages::newtypes::{
    ConfigurationHash, PlaintextsHash, TrusteeIndex,
};
use wbraid::messages::predicate::{
    Plaintexts as PublishedPlaintexts, Predicate,
};
use wbraid::messages::wire::ProtocolMessage;

use crate::configuration::CIPHERTEXT_WIDTH;
use crate::encoding::{decode_plaintext_element, ElementPayload, HashHex};
use crate::tally::TallyBoardReading;
use crate::tally_board::SignedBallots;
use crate::Ctx;

/// One reading of a tally board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TallyView {
    /// The board does not carry its `Ballots` message.
    BallotsPending,
    /// The board carries its `Ballots` message and no quorum member has mixed.
    BallotsPosted,
    /// A quorum member has mixed.
    Mixing,
    /// A quorum member has posted its partial decryptions or its plaintexts.
    Decrypting,
    /// `threshold` quorum members published the same plaintexts.
    Decrypted {
        /// What each plaintext element encodes, in the order the trustees
        /// published them.
        payloads: Vec<ElementPayload>,
    },
    /// Nothing will ever be decrypted from this board: its parent serves
    /// another Configuration, braid refused what it carries, it carries a
    /// `Ballots` message other than its own, or the quorum's plaintexts
    /// disagree or cannot be read.
    Unusable {
        reason: TallyFailureReason,
        detail: String,
    },
}

impl TallyView {
    /// What the tally session records of this reading.
    pub fn reading(&self) -> TallyBoardReading {
        match self {
            TallyView::BallotsPending => {
                TallyBoardReading::Phase(TallyBoardPhase::BALLOTS_PENDING)
            }
            TallyView::BallotsPosted => {
                TallyBoardReading::Phase(TallyBoardPhase::BALLOTS_POSTED)
            }
            TallyView::Mixing => {
                TallyBoardReading::Phase(TallyBoardPhase::MIXING)
            }
            TallyView::Decrypting => {
                TallyBoardReading::Phase(TallyBoardPhase::DECRYPTING)
            }
            TallyView::Decrypted { .. } => {
                TallyBoardReading::Phase(TallyBoardPhase::DECRYPTED)
            }
            TallyView::Unusable { reason, detail } => {
                TallyBoardReading::Unusable {
                    reason: *reason,
                    detail: detail.clone(),
                }
            }
        }
    }

    pub(crate) fn unusable(
        reason: TallyFailureReason,
        detail: String,
    ) -> TallyView {
        TallyView::Unusable { reason, detail }
    }
}

/// Read a tally board that braid has already verified.
///
/// **Pre-condition**: the store's Configuration must be checked to be the
/// expected one, and `configuration` must be its hash. `child_messages` are
/// the tally board's own messages, the ones the store was fed from besides
/// the parent's.
pub(crate) fn derive(
    configuration: &ConfigurationHash,
    expected: &SignedBallots,
    store: &MessageStore<Ctx>,
    child_messages: &[ProtocolMessage<Ctx>],
) -> TallyView {
    let quorum = expected.quorum();
    let mut ours = false;
    let mut mixed = false;
    let mut decrypting = false;
    let mut plaintexts: Vec<PublishedPlaintexts> = Vec::new();

    for predicate in store.get_predicates() {
        // Messages naming another Configuration, or sent by a trustee outside
        // the quorum, are not this tally's progress.
        let counts = |sender: &TrusteeIndex| {
            predicate.get_configuration() == *configuration
                && quorum.contains(sender)
        };
        match &predicate {
            // The board has a single `Ballots` slot, whatever the
            // Configuration: two distinct ones halt every trustee.
            Predicate::Ballots(_) => {
                if predicate != *expected.predicate() {
                    return TallyView::unusable(
                        TallyFailureReason::INVALID_BOARD_CONTENT,
                        format!(
                            "the board carries a Ballots message other than \
                             its own: {predicate:?}"
                        ),
                    );
                }
                ours = true;
            }
            Predicate::Mix(mix) => mixed |= counts(&mix.sender),
            Predicate::PartialDecryptions(decryptions) => {
                decrypting |= counts(&decryptions.sender)
            }
            Predicate::Plaintexts(published) => {
                if counts(&published.sender) {
                    plaintexts.push(published.clone());
                }
            }
            // The parent's key generation, and the quorum's signatures on
            // each other's mixes, say nothing a mix does not.
            Predicate::ConfigurationValid(_)
            | Predicate::Shares(_)
            | Predicate::PublicKey(_)
            | Predicate::MixSignature(_) => {}
        }
    }

    if !ours {
        return TallyView::BallotsPending;
    }
    if let Some(disagreement) = disagreement(&plaintexts) {
        return disagreement;
    }
    let senders = plaintexts
        .iter()
        .map(|published| published.sender)
        .collect::<BTreeSet<_>>();
    match plaintexts.first() {
        Some(agreed) if senders.len() >= store.configuration().threshold => {
            decrypted(&agreed.plaintexts, store, child_messages)
        }
        Some(_) => TallyView::Decrypting,
        None if decrypting => TallyView::Decrypting,
        None if mixed => TallyView::Mixing,
        None => TallyView::BallotsPosted,
    }
}

/// Two quorum members published different plaintexts, or plaintexts of
/// different ciphertexts. That is final as soon as both are on the board: the
/// trustees halt on the disagreement and the missing plaintexts never arrive.
fn disagreement(plaintexts: &[PublishedPlaintexts]) -> Option<TallyView> {
    let first = plaintexts.first()?;
    let agree = plaintexts.iter().all(|published| {
        published.ciphertexts == first.ciphertexts
            && published.plaintexts == first.plaintexts
    });
    if agree {
        return None;
    }
    let published = plaintexts
        .iter()
        .map(|published| {
            format!(
                "trustee {}: plaintexts {} of ciphertexts {}",
                published.sender,
                HashHex::of(&published.plaintexts.0).short(),
                HashHex::of(&published.ciphertexts.0).short()
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(TallyView::unusable(
        TallyFailureReason::INVALID_BOARD_CONTENT,
        format!("the quorum published different plaintexts ({published})"),
    ))
}

/// The payloads of the plaintexts the quorum agreed on. The store keeps no
/// `Plaintexts` bodies, so the body is taken from the board's own message that
/// braid verifies to the agreed predicate.
fn decrypted(
    agreed: &PlaintextsHash,
    store: &MessageStore<Ctx>,
    child_messages: &[ProtocolMessage<Ctx>],
) -> TallyView {
    let body = child_messages.iter().find_map(|message| {
        let (predicate, body) = verify(message, store.configuration()).ok()?;
        let published = matches!(
            &predicate,
            Predicate::Plaintexts(published) if published.plaintexts == *agreed
        );
        published.then_some(body).flatten()
    });
    let payloads = body
        .context("the board holds no body for them")
        .and_then(|body| payloads(&body))
        .with_context(|| {
            format!("plaintexts {}", HashHex::of(&agreed.0).short())
        });
    match payloads {
        Ok(payloads) => TallyView::Decrypted { payloads },
        Err(err) => TallyView::unusable(
            TallyFailureReason::INVALID_BOARD_CONTENT,
            format!("{err:#}"),
        ),
    }
}

fn payloads(body: &[u8]) -> Result<Vec<ElementPayload>> {
    let plaintexts = Plaintexts::<Ctx, CIPHERTEXT_WIDTH>::deser(body)
        .context("a body that cannot be read as plaintexts")?;
    plaintexts
        .0
        .iter()
        .flatten()
        .enumerate()
        .map(|(position, element)| {
            decode_plaintext_element(element)
                .with_context(|| format!("element {position}"))
        })
        .collect()
}
