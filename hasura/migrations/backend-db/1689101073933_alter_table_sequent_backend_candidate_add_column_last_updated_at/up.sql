alter table "sequent_backend"."candidate" add column "last_updated_at" timestamptz
 null default now();
