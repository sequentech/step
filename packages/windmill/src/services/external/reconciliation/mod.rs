// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Datafix reconciliation: importing a CSV file Datafix produces, diffing it
//! against Sequent's current voter data per the source-of-truth rules,
//! generating a downloadable patch for Datafix, and applying the
//! Sequent-side changes directly.
//!
//! Reuses the existing Datafix API helpers throughout (per the spec's
//! explicit instruction to do so): `utils::external_voter_lock_key`/`PgLock`,
//! `utils::post_operation_result_to_electoral_log`, and
//! `utils::compose_area_name`. The per-voter cast-vote guard is
//! re-implemented rather than reused — see `apply`'s module doc.

pub mod apply;
pub mod bulk_create;
pub mod csv;
pub mod diff;
pub mod patch;
