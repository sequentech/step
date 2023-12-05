alter table "sequent_backend"."tally_session_execution" alter column "session_ids" drop not null;
alter table "sequent_backend"."tally_session_execution" add column "session_ids" int4;
