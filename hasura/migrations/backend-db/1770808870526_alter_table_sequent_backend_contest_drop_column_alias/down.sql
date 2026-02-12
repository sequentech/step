alter table "sequent_backend"."contest" alter column "alias" drop not null;
alter table "sequent_backend"."contest" add column "alias" text;
