BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."candidate" DROP CONSTRAINT "candidate_pkey";

ALTER TABLE "sequent_backend"."candidate"
    ADD CONSTRAINT "candidate_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
