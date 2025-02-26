alter table "sequent_backend"."secret" add column "created_at" timestamptz
 null default now();
