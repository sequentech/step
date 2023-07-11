alter table "sequent_backend"."area_contest" alter column "created_at" drop not null;
alter table "sequent_backend"."area_contest" add column "created_at" jsonb;
