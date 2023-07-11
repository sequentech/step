alter table "sequent_backend"."area" add column "last_updated_at" timestamptz
 null default now();
