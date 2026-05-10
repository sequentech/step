// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keys ceremony, tally ceremony, Velvet-backed tally runs, and results persistence helpers.

/// Encryption helpers used when packaging ceremony outputs.
pub mod encrypter;
/// Builds trustee ballot batches and posts ciphertexts to the bulletin board.
pub mod insert_ballots;
/// Trustee key download/check flows and creating keys ceremonies in Postgres.
pub mod keys_ceremony;
/// Safe renaming of Velvet output folders using election and contest display names.
pub mod renamer;
/// Uploads tally PDF/JSON/HTML artifacts and mirrors document ids on results tables.
pub mod result_documents;
/// Handles tally results persistence.
pub mod results;
/// Human-readable ceremony log lines derived from board messages.
pub mod serialize_logs;
/// Tally session creation, trustee reconnect handling, and execution status updates.
pub mod tally_ceremony;
/// Handles tally progress.
pub mod tally_progress;
/// IRV tie-break detection, resolution records, and validation against tally state.
pub mod tally_resolution;
/// Handles tally session errors.
pub mod tally_session_error;
/// Handles running tally pipes.
pub mod velvet_tally;
