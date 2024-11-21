alter table "sequent_backend"."results_contest_candidate" add column "created_at" timestamptz
 null default now();
