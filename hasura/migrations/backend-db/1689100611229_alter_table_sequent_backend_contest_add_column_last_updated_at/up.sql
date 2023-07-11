alter table "sequent_backend"."contest" add column "last_updated_at" timestamptz
 null default now();
