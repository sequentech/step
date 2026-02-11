alter table "sequent_backend"."election" alter column "name" drop not null;
alter table "sequent_backend"."election" add column "name" varchar;
