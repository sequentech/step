-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Voter-weighted contests count each ballot by its voter's weight, so their
-- vote figures can exceed what a 32-bit integer holds. Headcounts (census,
-- voters, auditable ballots) and positions stay integers.
ALTER TABLE "sequent_backend"."results_contest"
    ALTER COLUMN "total_votes" TYPE bigint,
    ALTER COLUMN "total_valid_votes" TYPE bigint,
    ALTER COLUMN "total_invalid_votes" TYPE bigint,
    ALTER COLUMN "explicit_invalid_votes" TYPE bigint,
    ALTER COLUMN "implicit_invalid_votes" TYPE bigint,
    ALTER COLUMN "total_blank_votes" TYPE bigint,
    ALTER COLUMN "explicit_blank_votes" TYPE bigint,
    ALTER COLUMN "implicit_blank_votes" TYPE bigint;

ALTER TABLE "sequent_backend"."results_area_contest"
    ALTER COLUMN "total_votes" TYPE bigint,
    ALTER COLUMN "total_valid_votes" TYPE bigint,
    ALTER COLUMN "total_invalid_votes" TYPE bigint,
    ALTER COLUMN "explicit_invalid_votes" TYPE bigint,
    ALTER COLUMN "implicit_invalid_votes" TYPE bigint,
    ALTER COLUMN "total_blank_votes" TYPE bigint,
    ALTER COLUMN "explicit_blank_votes" TYPE bigint,
    ALTER COLUMN "implicit_blank_votes" TYPE bigint;

ALTER TABLE "sequent_backend"."results_contest_candidate"
    ALTER COLUMN "cast_votes" TYPE bigint;

ALTER TABLE "sequent_backend"."results_area_contest_candidate"
    ALTER COLUMN "cast_votes" TYPE bigint;

ALTER TABLE "sequent_backend"."results_election"
    ALTER COLUMN "blank_ballots" TYPE bigint;

ALTER TABLE "sequent_backend"."results_election_area"
    ALTER COLUMN "blank_ballots" TYPE bigint;
