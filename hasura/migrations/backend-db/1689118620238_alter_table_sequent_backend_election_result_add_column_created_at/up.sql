alter table "sequent_backend"."election_result" add column "created_at" timestamptz
 null default now();
