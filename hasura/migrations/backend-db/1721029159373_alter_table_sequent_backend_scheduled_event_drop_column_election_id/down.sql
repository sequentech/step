alter table "sequent_backend"."scheduled_event" alter column "election_id" drop not null;
alter table "sequent_backend"."scheduled_event" add column "election_id" uuid;
