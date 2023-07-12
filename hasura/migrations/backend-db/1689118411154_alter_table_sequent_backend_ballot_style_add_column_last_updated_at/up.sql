alter table "sequent_backend"."ballot_style" add column "last_updated_at" timestamptz
 null default now();
