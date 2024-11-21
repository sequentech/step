alter table "sequent_backend"."results_area_contest_candidate" add column "created_at" timestamptz
 null default now();
