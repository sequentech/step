alter table "sequent_backend"."election_event" alter column "dates" drop not null;
alter table "sequent_backend"."election_event" add column "dates" jsonb;
