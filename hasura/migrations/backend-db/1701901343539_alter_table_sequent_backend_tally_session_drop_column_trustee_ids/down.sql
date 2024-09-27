alter table "sequent_backend"."tally_session" alter column "trustee_ids" drop not null;
alter table "sequent_backend"."tally_session" add column "trustee_ids" _uuid;
