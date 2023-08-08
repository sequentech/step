BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."ballot_style" DROP CONSTRAINT "ballot_style_pkey";

ALTER TABLE "sequent_backend"."ballot_style"
    ADD CONSTRAINT "ballot_style_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
