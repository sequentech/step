BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."tasks_execution" DROP CONSTRAINT "tasks_execution_pkey";

ALTER TABLE "sequent_backend"."tasks_execution"
    ADD CONSTRAINT "tasks_execution_pkey" PRIMARY KEY ("tenant_id", "id");
COMMIT TRANSACTION;
