ALTER TABLE "sequent_backend"."keys_ceremony" ALTER COLUMN "permission_labels" TYPE text[];
alter table "sequent_backend"."keys_ceremony" rename column "permission_labels" to "permission_label";
