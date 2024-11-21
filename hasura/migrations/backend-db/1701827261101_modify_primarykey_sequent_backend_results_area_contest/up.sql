BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."results_area_contest" DROP CONSTRAINT "results_area_contest_pkey";

ALTER TABLE "sequent_backend"."results_area_contest"
    ADD CONSTRAINT "results_area_contest_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id", "results_event_id");
COMMIT TRANSACTION;
