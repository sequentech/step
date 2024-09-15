alter table "sequent_backend"."event_list" alter column "schedule" drop not null;
alter table "sequent_backend"."event_list" add column "schedule" timestamptz;
