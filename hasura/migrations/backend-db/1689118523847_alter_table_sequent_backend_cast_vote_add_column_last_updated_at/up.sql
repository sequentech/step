alter table "sequent_backend"."cast_vote" add column "last_updated_at" timestamptz
 null default now();
