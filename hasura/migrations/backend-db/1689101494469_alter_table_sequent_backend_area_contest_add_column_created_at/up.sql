alter table "sequent_backend"."area_contest" add column "created_at" timestamptz
 null default now();
