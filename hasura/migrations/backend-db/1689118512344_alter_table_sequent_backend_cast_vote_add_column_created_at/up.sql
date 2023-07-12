alter table "sequent_backend"."cast_vote" add column "created_at" timestamptz
 null default now();
