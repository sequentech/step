alter table "sequent_backend"."candidate" alter column "alias" drop not null;
alter table "sequent_backend"."candidate" add column "alias" text;
