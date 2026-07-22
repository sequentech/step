// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Datafix reconciliation: importing a CSV file Datafix produces, diffing it
//! against Sequent's current voter data per the source-of-truth rules,
//! generating a downloadable patch for
//! Datafix, and applying the Sequent-side changes directly. See
//! `DatafixReconciliationImplementationPlan.md` at the repository root for
//! the full design (data model, Hasura wiring, task/route plumbing).
//!
//! Reuses the existing Datafix API helpers throughout (per the spec's
//! explicit instruction to do so): `api_datafix::ensure_voter_has_no_valid_vote`,
//! `utils::datafix_voter_lock_key`/`PgLock`, `utils::post_operation_result_to_electoral_log`,
//! and `utils::compose_area_name`.

pub mod apply;
pub mod csv;
pub mod diff;
pub mod patch;
