alter table "sequent_backend"."tally_session_execution" alter column "document_id" drop not null;
alter table "sequent_backend"."tally_session_execution" add column "document_id" uuid;
