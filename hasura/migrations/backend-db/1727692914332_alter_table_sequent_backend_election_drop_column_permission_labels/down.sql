alter table "sequent_backend"."election" alter column "permission_labels" drop not null;
alter table "sequent_backend"."election" add column "permission_labels" text;
