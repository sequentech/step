alter table "sequent_backend"."applicant_attributes" add column "created_at" timestamptz
 not null default now();
