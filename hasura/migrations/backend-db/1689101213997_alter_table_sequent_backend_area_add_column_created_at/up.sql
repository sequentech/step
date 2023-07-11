alter table "sequent_backend"."area" add column "created_at" timestamptz
 null default now();
