ALTER TABLE "sequent_backend"."results_election_area"
    DROP COLUMN "blank_ballots",
    DROP COLUMN "blank_ballots_percent";

ALTER TABLE "sequent_backend"."results_election"
    DROP COLUMN "blank_ballots",
    DROP COLUMN "blank_ballots_percent";
