ALTER TABLE "sequent_backend"."results_election"
    ADD COLUMN "blank_ballots" integer,
    ADD COLUMN "blank_ballots_percent" numeric;

ALTER TABLE "sequent_backend"."results_election_area"
    ADD COLUMN "blank_ballots" integer,
    ADD COLUMN "blank_ballots_percent" numeric;
