-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

\set ON_ERROR_STOP on

-- Run before rollback on populated deployments, or to remove a failed build.
-- Never pass --single-transaction.
DROP INDEX CONCURRENTLY IF EXISTS sequent_backend.ballot_style_voter_reference_idx;
