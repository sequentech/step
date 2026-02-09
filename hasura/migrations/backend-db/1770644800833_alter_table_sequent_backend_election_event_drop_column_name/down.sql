alter table "sequent_backend"."election_event" alter column "name" drop not null;
alter table "sequent_backend"."election_event" add column "name" varchar;
