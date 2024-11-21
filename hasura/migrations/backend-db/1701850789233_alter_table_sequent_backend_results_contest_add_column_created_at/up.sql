alter table "sequent_backend"."results_contest" add column "created_at" timestamptz
 null default now();
