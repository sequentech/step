alter table "sequent_backend"."contest" alter column "name" drop not null;
alter table "sequent_backend"."contest" add column "name" varchar;
