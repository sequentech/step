alter table "sequent_backend"."election" alter column "alias" drop not null;
alter table "sequent_backend"."election" add column "alias" text;
