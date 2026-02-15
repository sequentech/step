alter table "sequent_backend"."candidate" alter column "name" drop not null;
alter table "sequent_backend"."candidate" add column "name" varchar;
