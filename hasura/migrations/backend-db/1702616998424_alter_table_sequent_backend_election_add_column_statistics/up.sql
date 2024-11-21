alter table "sequent_backend"."election" add column "statistics" jsonb
 null default jsonb_build_object();
