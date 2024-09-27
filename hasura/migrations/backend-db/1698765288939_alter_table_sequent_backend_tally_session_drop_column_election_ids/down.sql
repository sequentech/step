alter table "sequent_backend"."tally_session" alter column "election_ids" drop not null;
alter table "sequent_backend"."tally_session" add column "election_ids" uuid;
