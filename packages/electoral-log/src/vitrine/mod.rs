// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Step's side of Vitrine, the transparency logs: the entries step submits, the
//! statements it signs and the receipt chains it stores. Step depends on no
//! Vitrine crate: it encodes these objects itself, and the tests check them
//! against Vitrine's vectors (`tests/vitrine-vectors/`).

pub mod entry;
pub mod note;
pub mod proof;
pub mod statement;

pub use entry::{
    ballot_id_hash, leaf_hash, ElectionEntry, EventKind, EventsEntry, Field, Hash, Subject,
    ELECTION_TAG, EVENTS_TAG, MAX_ENTRY_SIZE,
};
pub use note::{NoteSigner, Vkey};
pub use proof::{receipt_chain, ChainBlock, Submitted};
pub use statement::{
    election_key_hash, style_digest, style_list, styles_hash, ConfigStatement, Demo, MilestoneKind,
    MilestoneStatement, Release, Visibility,
};
