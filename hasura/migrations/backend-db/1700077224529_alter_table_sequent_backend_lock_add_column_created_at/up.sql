alter table "sequent_backend"."lock" add column "created_at" timestamptz
 not null default now();
