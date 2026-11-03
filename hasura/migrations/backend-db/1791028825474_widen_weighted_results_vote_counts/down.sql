-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE "sequent_backend"."results_election_area"
    ALTER COLUMN "blank_ballots" TYPE integer;

ALTER TABLE "sequent_backend"."results_election"
    ALTER COLUMN "blank_ballots" TYPE integer;

ALTER TABLE "sequent_backend"."results_area_contest_candidate"
    ALTER COLUMN "cast_votes" TYPE integer;

ALTER TABLE "sequent_backend"."results_contest_candidate"
    ALTER COLUMN "cast_votes" TYPE integer;

ALTER TABLE "sequent_backend"."results_area_contest"
    ALTER COLUMN "total_votes" TYPE integer,
    ALTER COLUMN "total_valid_votes" TYPE integer,
    ALTER COLUMN "total_invalid_votes" TYPE integer,
    ALTER COLUMN "explicit_invalid_votes" TYPE integer,
    ALTER COLUMN "implicit_invalid_votes" TYPE integer,
    ALTER COLUMN "total_blank_votes" TYPE integer,
    ALTER COLUMN "explicit_blank_votes" TYPE integer,
    ALTER COLUMN "implicit_blank_votes" TYPE integer;

ALTER TABLE "sequent_backend"."results_contest"
    ALTER COLUMN "total_votes" TYPE integer,
    ALTER COLUMN "total_valid_votes" TYPE integer,
    ALTER COLUMN "total_invalid_votes" TYPE integer,
    ALTER COLUMN "explicit_invalid_votes" TYPE integer,
    ALTER COLUMN "implicit_invalid_votes" TYPE integer,
    ALTER COLUMN "total_blank_votes" TYPE integer,
    ALTER COLUMN "explicit_blank_votes" TYPE integer,
    ALTER COLUMN "implicit_blank_votes" TYPE integer;
