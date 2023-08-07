BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."contest" DROP CONSTRAINT "contest_pkey";

ALTER TABLE "sequent_backend"."contest"
    ADD CONSTRAINT "contest_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
