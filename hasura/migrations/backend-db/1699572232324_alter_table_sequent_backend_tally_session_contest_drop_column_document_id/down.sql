alter table "sequent_backend"."tally_session_contest" alter column "document_id" drop not null;
alter table "sequent_backend"."tally_session_contest" add column "document_id" uuid;
