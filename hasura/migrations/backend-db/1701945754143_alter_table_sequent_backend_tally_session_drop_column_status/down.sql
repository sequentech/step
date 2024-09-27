alter table "sequent_backend"."tally_session" alter column "status" drop not null;
alter table "sequent_backend"."tally_session" add column "status" jsonb;
