BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."results_election" DROP CONSTRAINT "results_election_pkey";

ALTER TABLE "sequent_backend"."results_election"
    ADD CONSTRAINT "results_election_pkey" PRIMARY KEY ("election_event_id", "results_event_id", "id", "tenant_id");
COMMIT TRANSACTION;
