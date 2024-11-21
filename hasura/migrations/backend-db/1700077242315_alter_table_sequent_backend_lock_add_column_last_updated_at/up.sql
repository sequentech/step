alter table "sequent_backend"."lock" add column "last_updated_at" timestamptz
 null default now();
