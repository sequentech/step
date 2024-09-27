alter table "sequent_backend"."election" alter column "image_name" drop not null;
alter table "sequent_backend"."election" add column "image_name" text;
