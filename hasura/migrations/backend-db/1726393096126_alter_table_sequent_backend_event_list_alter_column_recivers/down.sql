alter table "sequent_backend"."event_list" rename column "receivers" to "recivers";
ALTER TABLE "sequent_backend"."event_list" ALTER COLUMN "recivers" TYPE ARRAY;
