BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."election" DROP CONSTRAINT "election_pkey";

ALTER TABLE "sequent_backend"."election"
    ADD CONSTRAINT "election_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
