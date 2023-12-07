alter table "sequent_backend"."results_event" add column "last_updated_at" timestamptz
 null default now();
