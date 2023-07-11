alter table "sequent_backend"."area_contest" add column "last_updated_at" timestamptz
 null default now();
