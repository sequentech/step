ALTER TABLE "sequent_backend"."results_area_contest"
    DROP COLUMN "explicit_blank_votes",
    DROP COLUMN "implicit_blank_votes",
    DROP COLUMN "explicit_blank_votes_percent",
    DROP COLUMN "implicit_blank_votes_percent";

ALTER TABLE "sequent_backend"."results_area_contest"
    RENAME COLUMN "total_blank_votes_percent" TO "blank_votes_percent";

ALTER TABLE "sequent_backend"."results_area_contest"
    RENAME COLUMN "total_blank_votes" TO "blank_votes";

ALTER TABLE "sequent_backend"."results_contest"
    DROP COLUMN "explicit_blank_votes",
    DROP COLUMN "implicit_blank_votes",
    DROP COLUMN "explicit_blank_votes_percent",
    DROP COLUMN "implicit_blank_votes_percent";

ALTER TABLE "sequent_backend"."results_contest"
    RENAME COLUMN "total_blank_votes_percent" TO "blank_votes_percent";

ALTER TABLE "sequent_backend"."results_contest"
    RENAME COLUMN "total_blank_votes" TO "blank_votes";
