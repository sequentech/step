alter table "sequent_backend"."results_contest" add column "last_updated_at" timestamptz
 null default now();
