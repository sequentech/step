alter table "sequent_backend"."results_election" add column "last_updated_at" timestamptz
 null default now();
