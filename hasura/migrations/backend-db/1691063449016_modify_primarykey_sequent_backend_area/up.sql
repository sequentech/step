BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."area" DROP CONSTRAINT "area_pkey";

ALTER TABLE "sequent_backend"."area"
    ADD CONSTRAINT "area_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
