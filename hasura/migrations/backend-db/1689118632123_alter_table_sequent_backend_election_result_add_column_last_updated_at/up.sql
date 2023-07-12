alter table "sequent_backend"."election_result" add column "last_updated_at" timestamptz
 null default now();
