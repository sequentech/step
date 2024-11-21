alter table "sequent_backend"."results_contest_candidate" add column "last_updated_at" timestamptz
 null default now();
