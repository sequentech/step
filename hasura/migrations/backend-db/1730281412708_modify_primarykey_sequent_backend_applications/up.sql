BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."applications" DROP CONSTRAINT "applications_pkey";

ALTER TABLE "sequent_backend"."applications"
    ADD CONSTRAINT "applications_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
