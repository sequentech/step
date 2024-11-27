BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."applicant_attributes" DROP CONSTRAINT "applicant_attributes_pkey";

ALTER TABLE "sequent_backend"."applicant_attributes"
    ADD CONSTRAINT "applicant_attributes_pkey" PRIMARY KEY ("id");
COMMIT TRANSACTION;
