alter table "sequent_backend"."applicant_attributes" add column "updated_at" timestamptz
 not null default now();
