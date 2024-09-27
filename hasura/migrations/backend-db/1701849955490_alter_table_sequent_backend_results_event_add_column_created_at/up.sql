alter table "sequent_backend"."results_event" add column "created_at" timestamptz
 null default now();
