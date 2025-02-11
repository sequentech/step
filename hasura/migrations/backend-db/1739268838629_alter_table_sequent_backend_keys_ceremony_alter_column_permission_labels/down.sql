alter table "sequent_backend"."keys_ceremony" rename column "permission_label" to "permission_labels";
ALTER TABLE "sequent_backend"."keys_ceremony" ALTER COLUMN "permission_labels" TYPE ARRAY;
