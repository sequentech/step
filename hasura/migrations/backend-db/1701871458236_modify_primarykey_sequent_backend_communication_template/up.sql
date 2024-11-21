BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."communication_template" DROP CONSTRAINT "communication_template_pkey";

ALTER TABLE "sequent_backend"."communication_template"
    ADD CONSTRAINT "communication_template_pkey" PRIMARY KEY ("id", "tenant_id");
COMMIT TRANSACTION;
