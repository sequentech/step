// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! PostgreSQL access for Windmill against the `sequent_backend` schema exposed via Hasura.

/// Enrollment applications and related filters for an election event.
pub mod application;
/// Hierarchical areas and links to contests.
pub mod area;
/// Join rows between areas and contests for a tenant and election event.
pub mod area_contest;
/// Ballot publication records tied to publication workflows.
pub mod ballot_publication;
/// Per-contest or per-election ballot styling stored in Postgres.
pub mod ballot_style;
/// Candidate rows for contests within an election event.
pub mod candidate;
/// Cast vote payloads and metadata persisted for auditing and tally.
pub mod cast_vote;
/// Certificate authority material recorded during setup ceremonies.
pub mod certificate_authority;
/// Contest definitions scoped to tenant and election event.
pub mod contest;
/// Generated or uploaded documents (PDFs, exports) tracked in the backend.
pub mod document;
/// Election records (parent of contests) for an election event.
pub mod election;
/// Election event rows, dates, and presentation configuration.
pub mod election_event;
/// Keycloak realm identifiers and linkage stored for automation tasks.
pub mod keycloak_realm;
/// Trustee keys and ceremony progress for cryptographic setup.
pub mod keys_ceremony;
/// Advisory locks to serialize concurrent maintenance on shared rows.
pub mod lock;
/// Housekeeping queries (cleanup, backfills) run from Windmill.
pub mod maintenance;
/// Public preview configuration blobs for the voting portal.
pub mod preview;
/// Report render jobs and output paths associated with templates.
pub mod render_report;
/// Report definitions and scheduling metadata in `sequent_backend.report`.
pub mod reports;
/// Per-area, per-contest tally aggregates after a tally session.
pub mod results_area_contest;
/// Candidate-level results under an area contest aggregation.
pub mod results_area_contest_candidate;
/// Contest-wide tally totals independent of area breakdown.
pub mod results_contest;
/// Candidate totals within a contest result snapshot.
pub mod results_contest_candidate;
/// Election-wide rolled-up results for a tally session.
pub mod results_election;
/// Area-scoped slices of election-wide results.
pub mod results_election_area;
/// High-level result events (sessions, publication markers).
pub mod results_event;
/// Cron-driven scheduled events stored for the worker.
pub mod scheduled_event;
/// Opaque secret references persisted for tasks (not the plaintext values).
pub mod secret;
/// Tally session lifecycle (start, status, completion) in Postgres.
pub mod tally_session;
/// Contest-level rows inside an active tally session.
pub mod tally_session_contest;
/// Per-step execution records while a tally session runs.
pub mod tally_session_execution;
/// Resolution or cancellation markers when a tally session ends.
pub mod tally_session_resolution;
/// Tally sheet definitions and uploaded sheet metadata.
pub mod tally_sheet;
/// Celery task execution logs mirrored or queried from Postgres.
pub mod tasks_execution;
/// Communication templates (email/SMS) stored for rendering.
pub mod template;
/// Tenant-level rows used by cross-event maintenance tasks.
pub mod tenant;
/// Trustee directory entries for an election event.
pub mod trustee;
