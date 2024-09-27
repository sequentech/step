BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."results_contest" DROP CONSTRAINT "results_contest_pkey";

ALTER TABLE "sequent_backend"."results_contest"
    ADD CONSTRAINT "results_contest_pkey" PRIMARY KEY ("tenant_id", "id", "election_event_id", "results_event_id");
COMMIT TRANSACTION;
