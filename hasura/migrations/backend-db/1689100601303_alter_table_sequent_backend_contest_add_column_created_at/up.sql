alter table "sequent_backend"."contest" add column "created_at" timestamptz
 null default now();
