alter table "sequent_backend"."election" add column "allow_tally_at" timestamptz
 null default now();
