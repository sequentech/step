// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Traits for what services need from outside Windmill: databases, the
//! bulletin board, the electoral log, storage, the clock and identifiers.
//! Production implementations live in `crate::adapters`.

pub mod clock;
pub mod results_publication;
