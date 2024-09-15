ALTER TABLE "sequent_backend"."event_list" ALTER COLUMN "recivers" TYPE text[];
alter table "sequent_backend"."event_list" rename column "recivers" to "receivers";
