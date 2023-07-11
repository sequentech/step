alter table "sequent_backend"."candidate" add column "created_at" timestamptz
 null default now();
