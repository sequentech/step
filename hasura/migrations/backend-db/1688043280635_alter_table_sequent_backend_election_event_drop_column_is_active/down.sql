alter table "sequent_backend"."election_event" alter column "is_active" set default false;
alter table "sequent_backend"."election_event" alter column "is_active" drop not null;
alter table "sequent_backend"."election_event" add column "is_active" bool;
