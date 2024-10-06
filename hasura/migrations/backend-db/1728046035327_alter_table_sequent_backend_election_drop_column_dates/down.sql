alter table "sequent_backend"."election" alter column "dates" drop not null;
alter table "sequent_backend"."election" add column "dates" jsonb;
