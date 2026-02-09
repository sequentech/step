alter table "sequent_backend"."election_event" alter column "alias" drop not null;
alter table "sequent_backend"."election_event" add column "alias" text;
