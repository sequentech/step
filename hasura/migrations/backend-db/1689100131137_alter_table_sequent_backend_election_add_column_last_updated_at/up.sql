alter table "sequent_backend"."election" add column "last_updated_at" timestamptz
 null default now();
