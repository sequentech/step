
CREATE SCHEMA "sequent_backend";

CREATE TABLE "sequent_backend"."tenant" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "username" text NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "updated_at" timestamptz NOT NULL DEFAULT now(),
    "labels" jsonb NULL,
    "annotations" jsonb NULL,
    PRIMARY KEY ("id")
);

CREATE OR REPLACE FUNCTION "sequent_backend"."set_current_timestamp_updated_at"()
RETURNS trigger AS $$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."updated_at" = NOW();
  RETURN _new;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER "set_sequent_backend_tenant_updated_at"
BEFORE UPDATE ON "sequent_backend"."tenant"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_updated_at"();
COMMENT ON TRIGGER "set_sequent_backend_tenant_updated_at" ON "sequent_backend"."tenant"
IS 'trigger to set value of column "updated_at" to current timestamp on row update';
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."event" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "created_at" timestamptz DEFAULT now(),
    "updated_at" timestamptz DEFAULT now(),
    "labels" jsonb,
    "annotations" jsonb,
    "tenant_id" uuid NOT NULL,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant" (
        "id"
    ) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE OR REPLACE FUNCTION "sequent_backend"."set_current_timestamp_updated_at"()
RETURNS trigger AS $$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."updated_at" = NOW();
  RETURN _new;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER "set_sequent_backend_event_updated_at"
BEFORE UPDATE ON "sequent_backend"."event"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_updated_at"();
COMMENT ON TRIGGER "set_sequent_backend_event_updated_at" ON "sequent_backend"."event"
IS 'trigger to set value of column "updated_at" to current timestamp on row update';

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE INDEX "event_labels" ON
"sequent_backend"."event" USING btree ("labels");

CREATE INDEX "tenant_labels" ON
"sequent_backend"."tenant" USING btree ("labels");

alter table "sequent_backend"."event" rename to "election_event";

alter table "sequent_backend"."election_event" add column "is_active" boolean
 not null default 'false';

alter table "sequent_backend"."tenant" add column "is_active" boolean
 not null default 'false';

alter table "sequent_backend"."election_event" drop column "is_active" cascade;

CREATE TABLE "sequent_backend"."election" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), PRIMARY KEY ("id") );
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."contest" ("id" uuid NOT NULL, PRIMARY KEY ("id") );

alter table "sequent_backend"."contest" alter column "id" set default gen_random_uuid();

CREATE TABLE "sequent_backend"."candidate" ("id" uuid NOT NULL, PRIMARY KEY ("id") );

alter table "sequent_backend"."candidate" alter column "id" set default gen_random_uuid();

CREATE TABLE "sequent_backend"."area" ("id" uuid NOT NULL, PRIMARY KEY ("id") );

alter table "sequent_backend"."area" alter column "id" set default gen_random_uuid();

CREATE TABLE "sequent_backend"."ballot_style" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), PRIMARY KEY ("id") );
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."cast_vote" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), PRIMARY KEY ("id") );
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."election_result" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), PRIMARY KEY ("id") );
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."election_event" add column "name" varchar
 not null;

alter table "sequent_backend"."election_event" add column "description" text
 null;

alter table "sequent_backend"."election_event" add column "presentation" jsonb
 null;

alter table "sequent_backend"."election_event" add column "bulletin_board_reference" jsonb
 null;

alter table "sequent_backend"."election_event" add column "is_archived" boolean
 not null default 'false';

alter table "sequent_backend"."election_event" add column "voting_channels" jsonb
 null;

alter table "sequent_backend"."election_event" add column "dates" jsonb
 null;

alter table "sequent_backend"."election_event" add column "status" jsonb
 null;

alter table "sequent_backend"."election_event" add column "user_boards" varchar
 null;

alter table "sequent_backend"."election_event" add column "encryption_protocol" varchar
 not null;

alter table "sequent_backend"."election_event" add column "is_audit" boolean
 null;

alter table "sequent_backend"."election_event" add column "audit_election_event_id" integer
 null;

alter table "sequent_backend"."election_event" drop column "audit_election_event_id" cascade;

alter table "sequent_backend"."election_event" add column "audit_election_event_id" uuid
 null;

alter table "sequent_backend"."election_event"
  add constraint "election_event_audit_election_event_id_fkey"
  foreign key ("audit_election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."election" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."election" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."election" add column "created_at" Timestamp
 null default now();

alter table "sequent_backend"."election" drop column "created_at" cascade;

alter table "sequent_backend"."election" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."election" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."election" add column "labels" jsonb
 null;

alter table "sequent_backend"."election" add column "annotations" jsonb
 null;

alter table "sequent_backend"."election" add column "name" varchar
 not null;

alter table "sequent_backend"."election" add column "description" text
 null;

alter table "sequent_backend"."election" add column "presentation" jsonb
 null;

alter table "sequent_backend"."election" add column "dates" jsonb
 null;

alter table "sequent_backend"."election" add column "status" jsonb
 null;

alter table "sequent_backend"."election" add column "eml" bytea
 null;

alter table "sequent_backend"."election" add column "num_allowed_revotes" integer
 null;

alter table "sequent_backend"."election" add column "is_consolidated_ballot_encoding" boolean
 null;

alter table "sequent_backend"."election" add column "spoil_ballot_option" boolean
 null;

alter table "sequent_backend"."contest" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."contest" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."contest" add column "election_id" uuid
 null;

alter table "sequent_backend"."contest" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."contest" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."contest" add column "labels" jsonb
 null;

alter table "sequent_backend"."contest" add column "annotations" jsonb
 null;

alter table "sequent_backend"."contest" add column "is_acclaimed" boolean
 null;

alter table "sequent_backend"."contest" add column "is_active" boolean
 null;

alter table "sequent_backend"."contest" add column "name" varchar
 null;

alter table "sequent_backend"."contest" add column "description" text
 null;

alter table "sequent_backend"."contest" add column "presentation" jsonb
 null;

alter table "sequent_backend"."contest" add column "min_votes" integer
 null;

alter table "sequent_backend"."contest" add column "max_votes" integer
 null;

alter table "sequent_backend"."contest" add column "voting_type" varchar
 null;

alter table "sequent_backend"."contest" add column "counting_algorithm" varchar
 null;

alter table "sequent_backend"."contest" add column "is_encrypted" boolean
 null;

alter table "sequent_backend"."contest" add column "tally_configuration" jsonb
 null;

alter table "sequent_backend"."candidate" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."candidate" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."candidate" add column "contest_id" uuid
 null;

alter table "sequent_backend"."candidate" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."candidate" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."candidate" add column "labels" jsonb
 null;

alter table "sequent_backend"."candidate" add column "annotations" jsonb
 null;

alter table "sequent_backend"."candidate" add column "name" varchar
 null;

alter table "sequent_backend"."candidate" add column "description" text
 null;

alter table "sequent_backend"."candidate" add column "type" varchar
 null;

alter table "sequent_backend"."candidate" add column "presentation" jsonb
 null;

alter table "sequent_backend"."candidate" add column "is_public" boolean
 null;

alter table "sequent_backend"."area" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."area" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."area" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."area" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."area" add column "labels" jsonb
 null;

alter table "sequent_backend"."area" add column "annotations" jsonb
 null;

alter table "sequent_backend"."area" add column "name" varchar
 null;

alter table "sequent_backend"."area" add column "description" text
 null;

alter table "sequent_backend"."area" add column "type" varchar
 null;

CREATE TABLE "sequent_backend"."area_context" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), PRIMARY KEY ("id") );
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."area_context" rename to "area_contest";

alter table "sequent_backend"."area_contest" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."area_contest" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."area_contest" add column "contest_id" uuid
 null;

alter table "sequent_backend"."area_contest" add column "area_id" uuid
 null;

alter table "sequent_backend"."area_contest" add column "created_at" jsonb
 null;

alter table "sequent_backend"."area_contest" drop column "created_at" cascade;

alter table "sequent_backend"."area_contest" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."area_contest" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."area_contest" add column "labels" jsonb
 null;

alter table "sequent_backend"."area_contest" add column "annotations" jsonb
 null;

alter table "sequent_backend"."ballot_style" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."cast_vote" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."election_result" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."ballot_style" add column "election_id" uuid
 null;

alter table "sequent_backend"."ballot_style" add column "area_id" uuid
 null;

alter table "sequent_backend"."ballot_style" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."ballot_style" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."ballot_style" add column "labels" jsonb
 null;

alter table "sequent_backend"."ballot_style" add column "annotations" jsonb
 null;

alter table "sequent_backend"."ballot_style" add column "ballot_eml" bytea
 null;

alter table "sequent_backend"."ballot_style" add column "ballot_signature" bytea
 null;

alter table "sequent_backend"."ballot_style" add column "status" varchar
 null;

alter table "sequent_backend"."cast_vote" add column "election_id" uuid
 null;

alter table "sequent_backend"."cast_vote" add column "area_id" uuid
 null;

alter table "sequent_backend"."cast_vote" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."cast_vote" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."cast_vote" add column "labels" jsonb
 null;

alter table "sequent_backend"."cast_vote" add column "annotations" jsonb
 null;

alter table "sequent_backend"."cast_vote" add column "cast_ballot_eml" bytea
 null;

alter table "sequent_backend"."cast_vote" add column "cast_ballot_signature" bytea
 null;

alter table "sequent_backend"."election_result" add column "area_id" uuid
 null;

alter table "sequent_backend"."election_result" add column "election_id" uuid
 null;

alter table "sequent_backend"."election_result" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."election_result" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."election_result" add column "labels" jsonb
 null;

alter table "sequent_backend"."election_result" add column "annotations" jsonb
 null;

alter table "sequent_backend"."election_result" add column "result_eml" bytea
 null;

alter table "sequent_backend"."election_result" add column "result_eml_signature" bytea
 null;

alter table "sequent_backend"."election_result" add column "statistics" jsonb
 null;

alter table "sequent_backend"."election"
  add constraint "election_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."election"
  add constraint "election_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."contest"
  add constraint "contest_id_fkey"
  foreign key ("id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."contest"
  add constraint "contest_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."candidate"
  add constraint "candidate_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."candidate"
  add constraint "candidate_contest_id_fkey"
  foreign key ("contest_id")
  references "sequent_backend"."contest"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area"
  add constraint "area_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area"
  add constraint "area_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_contest_id_fkey"
  foreign key ("contest_id")
  references "sequent_backend"."contest"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."election_result"
  add constraint "election_result_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."election_result"
  add constraint "election_result_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" add column "voter_id_string" varchar
 null;

alter table "sequent_backend"."ballot_style" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."election_result" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" add column "election_event_id" uuid
 null;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."candidate"
  add constraint "candidate_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."contest"
  add constraint "contest_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."contest" add column "conditions" jsonb
 null;

ALTER TABLE "sequent_backend"."election" ALTER COLUMN "eml" TYPE text;

ALTER TABLE "sequent_backend"."ballot_style" ALTER COLUMN "ballot_eml" TYPE text;

ALTER TABLE "sequent_backend"."cast_vote" ALTER COLUMN "cast_ballot_eml" TYPE text;

ALTER TABLE "sequent_backend"."election_result" ALTER COLUMN "result_eml" TYPE text;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_area_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_area_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_area_id_fkey";

alter table "sequent_backend"."election_result" drop constraint "election_result_area_id_fkey";

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."area" DROP CONSTRAINT "area_pkey";

ALTER TABLE "sequent_backend"."area"
    ADD CONSTRAINT "area_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "tenant_id", "area_id")
  references "sequent_backend"."area"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_contest_id_fkey";

alter table "sequent_backend"."candidate" drop constraint "candidate_contest_id_fkey";

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."contest" DROP CONSTRAINT "contest_pkey";

ALTER TABLE "sequent_backend"."contest"
    ADD CONSTRAINT "contest_pkey" PRIMARY KEY ("id", "tenant_id", "election_id");
COMMIT TRANSACTION;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_contest_id_election_event_id_fkey"
  foreign key ("tenant_id", "contest_id", "election_event_id")
  references "sequent_backend"."contest"
  ("tenant_id", "id", "election_id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_id_fkey";

alter table "sequent_backend"."election_result" drop constraint "election_result_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_id_fkey";

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."election" DROP CONSTRAINT "election_pkey";

ALTER TABLE "sequent_backend"."election"
    ADD CONSTRAINT "election_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."contest"
  add constraint "contest_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."ballot_style" DROP CONSTRAINT "ballot_style_pkey";

ALTER TABLE "sequent_backend"."ballot_style"
    ADD CONSTRAINT "ballot_style_pkey" PRIMARY KEY ("id", "tenant_id", "election_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."candidate" DROP CONSTRAINT "candidate_pkey";

ALTER TABLE "sequent_backend"."candidate"
    ADD CONSTRAINT "candidate_pkey" PRIMARY KEY ("tenant_id", "election_event_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."cast_vote" DROP CONSTRAINT "cast_vote_pkey";

ALTER TABLE "sequent_backend"."cast_vote"
    ADD CONSTRAINT "cast_vote_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."candidate" DROP CONSTRAINT "candidate_pkey";

ALTER TABLE "sequent_backend"."candidate"
    ADD CONSTRAINT "candidate_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."ballot_style" DROP CONSTRAINT "ballot_style_pkey";

ALTER TABLE "sequent_backend"."ballot_style"
    ADD CONSTRAINT "ballot_style_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."cast_vote" DROP CONSTRAINT "cast_vote_pkey";

ALTER TABLE "sequent_backend"."cast_vote"
    ADD CONSTRAINT "cast_vote_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."election_result" drop constraint "election_result_election_event_id_tenant_id_election_id_fkey";

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."election" DROP CONSTRAINT "election_pkey";

ALTER TABLE "sequent_backend"."election"
    ADD CONSTRAINT "election_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."contest"
  add constraint "contest_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "tenant_id", "election_id")
  references "sequent_backend"."election"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" rename column "cast_ballot_eml" to "content";

alter table "sequent_backend"."contest" drop constraint "contest_id_fkey",
  add constraint "contest_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey",
  add constraint "area_contest_tenant_id_election_event_id_area_id_fkey"
  foreign key ("tenant_id", "election_event_id", "area_id")
  references "sequent_backend"."area"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_election_event_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "tenant_id", "area_id")
  references "sequent_backend"."area"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "tenant_id", "area_id")
  references "sequent_backend"."area"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_contest_id_election_event_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "tenant_id", "area_id")
  references "sequent_backend"."area"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."contest" DROP CONSTRAINT "contest_pkey";

ALTER TABLE "sequent_backend"."contest"
    ADD CONSTRAINT "contest_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id");
COMMIT TRANSACTION;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_contest_id_fkey"
  foreign key ("election_event_id", "tenant_id", "contest_id")
  references "sequent_backend"."contest"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

INSERT INTO "sequent_backend"."tenant"("is_active", "annotations", "labels", "username", "created_at", "updated_at", "id") VALUES (true, null, null, E'COMELEC-EMS-OV', E'2023-08-10T22:04:32.314715+00:00', E'2023-08-10T22:04:32.314715+00:00', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."election_event"("is_archived", "is_audit", "encryption_protocol", "name", "user_boards", "annotations", "bulletin_board_reference", "dates", "labels", "presentation", "status", "voting_channels", "description", "created_at", "updated_at", "audit_election_event_id", "id", "tenant_id") VALUES (false, null, E'RSA256', E'election_event', null, null, null, null, null, null, E'{"i18n":{"en":{"name":"election_event","alias":null,"description":null},"es":{"name":null,"alias":null,"description":null}},"language_conf":{"enabled_language_codes":["en","es"]}}', null, null, E'2023-08-10T22:05:22.214163+00:00', E'2023-08-10T22:05:22.214163+00:00', null, E'33f18502-a67c-4853-8333-a58630663559', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."area"("name", "type", "annotations", "labels", "description", "created_at", "last_updated_at", "election_event_id", "id", "tenant_id") VALUES (E'area', null, null, null, null, E'2023-08-10T22:08:51.252443+00:00', E'2023-08-10T22:08:51.252443+00:00', E'33f18502-a67c-4853-8333-a58630663559', E'2f312a36-f39c-46e4-9670-1d1ce4625745', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."election"("is_consolidated_ballot_encoding", "spoil_ballot_option", "name", "num_allowed_revotes", "annotations", "dates", "labels", "presentation", "status", "description", "eml", "created_at", "last_updated_at", "election_event_id", "id", "tenant_id") VALUES (null, null, E'Simple election plurality', null, null, null, null, null, null, E'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.', null, E'2023-08-10T22:11:05.080656+00:00', E'2023-08-10T22:11:05.080656+00:00', E'33f18502-a67c-4853-8333-a58630663559', E'f2f1065e-b784-46d1-b81a-c71bfeb9ad55', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

CREATE TABLE "sequent_backend"."election_document" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid, "election_event_id" uuid, "name" varchar, "media_type" varchar, "size" integer, "labels" jsonb, "annotations" jsonb, "created_at" timestamptz DEFAULT now(), "last_updated_at" timestamptz DEFAULT now(), PRIMARY KEY ("id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, UNIQUE ("id"));
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."election_document" rename to "document";

CREATE TABLE "sequent_backend"."scheduled_event" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid, "election_event_id" uuid, "created_at" timestamptz DEFAULT now(), "stopped_at" timestamptz, "labels" jsonb, "annotations" jsonb, "event_processor" varchar, "cron_config" varchar, "event_payload" jsonb, "created_nby" varchar, PRIMARY KEY ("id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, UNIQUE ("id"));
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."event_execution" ("id" uuid NOT NULL, "tenant_id" uuid, "election_event_id" uuid, "scheduled_event_id" uuid NOT NULL, "labels" jsonb, "annotations" jsonb, "execution_state" varchar, "execution_payload" jsonb, "result_payload" jsonb, "started_at" timestamptz, "ended_at" timestamptz, PRIMARY KEY ("id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("scheduled_event_id") REFERENCES "sequent_backend"."scheduled_event"("id") ON UPDATE restrict ON DELETE restrict, UNIQUE ("id"));

alter table "sequent_backend"."scheduled_event" rename column "created_nby" to "created_by";

alter table "sequent_backend"."event_execution" alter column "id" set default gen_random_uuid();

alter table "sequent_backend"."scheduled_event" add column "board_id" integer
 null;

CREATE TABLE "sequent_backend"."trustee" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "public_key" text, "name" varchar, "created_at" timestamptz DEFAULT now(), "last_updated_at" timestamptz DEFAULT now(), "labels" jsonb, "annotations" jsonb, PRIMARY KEY ("id") );
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."scheduled_event" drop column "board_id" cascade;
