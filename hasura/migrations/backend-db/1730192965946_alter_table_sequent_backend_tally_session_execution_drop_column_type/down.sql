alter table "sequent_backend"."tally_session_execution" alter column "type" drop not null;
alter table "sequent_backend"."tally_session_execution" add column "type" text;
