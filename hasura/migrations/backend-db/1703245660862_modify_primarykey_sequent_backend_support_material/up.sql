BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."support_material" DROP CONSTRAINT "support_material_pkey";

ALTER TABLE "sequent_backend"."support_material"
    ADD CONSTRAINT "support_material_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;
