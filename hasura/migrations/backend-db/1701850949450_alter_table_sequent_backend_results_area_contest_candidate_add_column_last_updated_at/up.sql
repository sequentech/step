alter table "sequent_backend"."results_area_contest_candidate" add column "last_updated_at" timestamptz
 null default now();
