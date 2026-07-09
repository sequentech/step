ALTER TABLE "sequent_backend"."results_contest"
    RENAME COLUMN "blank_votes" TO "total_blank_votes";

ALTER TABLE "sequent_backend"."results_contest"
    RENAME COLUMN "blank_votes_percent" TO "total_blank_votes_percent";

ALTER TABLE "sequent_backend"."results_contest"
    ADD COLUMN "explicit_blank_votes" integer,
    ADD COLUMN "implicit_blank_votes" integer,
    ADD COLUMN "explicit_blank_votes_percent" numeric,
    ADD COLUMN "implicit_blank_votes_percent" numeric;

ALTER TABLE "sequent_backend"."results_area_contest"
    RENAME COLUMN "blank_votes" TO "total_blank_votes";

ALTER TABLE "sequent_backend"."results_area_contest"
    RENAME COLUMN "blank_votes_percent" TO "total_blank_votes_percent";

ALTER TABLE "sequent_backend"."results_area_contest"
    ADD COLUMN "explicit_blank_votes" integer,
    ADD COLUMN "implicit_blank_votes" integer,
    ADD COLUMN "explicit_blank_votes_percent" numeric,
    ADD COLUMN "implicit_blank_votes_percent" numeric;
