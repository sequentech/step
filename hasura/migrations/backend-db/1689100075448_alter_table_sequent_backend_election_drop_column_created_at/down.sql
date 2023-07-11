alter table "sequent_backend"."election" alter column "created_at" set default now();
alter table "sequent_backend"."election" alter column "created_at" drop not null;
alter table "sequent_backend"."election" add column "created_at" timestamp;
