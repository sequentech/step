alter table "sequent_backend"."election" add column "created_at" Timestamp
 null default now();
