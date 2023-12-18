alter table "sequent_backend"."election_event" add column "statistics" jsonb
 null default jsonb_build_object();
