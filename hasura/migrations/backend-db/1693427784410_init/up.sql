
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

INSERT INTO "sequent_backend"."tenant"("is_active", "annotations", "labels", "username", "created_at", "updated_at", "id") VALUES (true, null, null, E'tenant_user', E'2023-08-10T22:04:32.314715+00:00', E'2023-08-10T22:04:32.314715+00:00', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."election_event"("is_archived", "is_audit", "encryption_protocol", "name", "user_boards", "annotations", "bulletin_board_reference", "dates", "labels", "presentation", "status", "voting_channels", "description", "created_at", "updated_at", "audit_election_event_id", "id", "tenant_id") VALUES (false, null, E'RSA256', E'election_event', null, null, null, null, null, null, null, null, null, E'2023-08-10T22:05:22.214163+00:00', E'2023-08-10T22:05:22.214163+00:00', null, E'33f18502-a67c-4853-8333-a58630663559', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."area"("name", "type", "annotations", "labels", "description", "created_at", "last_updated_at", "election_event_id", "id", "tenant_id") VALUES (E'area', null, null, null, null, E'2023-08-10T22:08:51.252443+00:00', E'2023-08-10T22:08:51.252443+00:00', E'33f18502-a67c-4853-8333-a58630663559', E'2f312a36-f39c-46e4-9670-1d1ce4625745', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."election"("is_consolidated_ballot_encoding", "spoil_ballot_option", "name", "num_allowed_revotes", "annotations", "dates", "labels", "presentation", "status", "description", "eml", "created_at", "last_updated_at", "election_event_id", "id", "tenant_id") VALUES (null, null, E'Simple election plurality', null, null, null, null, null, null, E'This is the description of the election. You can add simple html like <strong>bold</strong> or <a href="https://sequentech.io" rel="nofollow">links to websites</a>.\n\n<br /><br />You need to use two br element for new paragraphs.', null, E'2023-08-10T22:11:05.080656+00:00', E'2023-08-10T22:11:05.080656+00:00', E'33f18502-a67c-4853-8333-a58630663559', E'f2f1065e-b784-46d1-b81a-c71bfeb9ad55', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."ballot_style"("ballot_signature", "status", "annotations", "labels", "ballot_eml", "created_at", "last_updated_at", "area_id", "election_event_id", "election_id", "id", "tenant_id") VALUES (null, null, null, null, E'eyJpZCI6MzQ1NzAwMDIsImNvbmZpZ3VyYXRpb24iOnsiaWQiOjM0NTcwMDAyLCJsYXlvdXQiOiJzaW1wbGUiLCJkaXJlY3RvciI6IjZ4eC1hMSIsImF1dGhvcml0aWVzIjpbIjZ4eC1hMiJdLCJ0aXRsZSI6IlNpbXBsZSBlbGVjdGlvbiBwbHVyYWxpdHkiLCJkZXNjcmlwdGlvbiI6IlRoaXMgaXMgdGhlIGRlc2NyaXB0aW9uIG9mIHRoZSBlbGVjdGlvbi4gWW91IGNhbiBhZGQgc2ltcGxlIGh0bWwgbGlrZSA8c3Ryb25nPmJvbGQ8L3N0cm9uZz4gb3IgPGEgaHJlZj1cImh0dHBzOi8vc2VxdWVudGVjaC5pb1wiIHJlbD1cIm5vZm9sbG93XCI+bGlua3MgdG8gd2Vic2l0ZXM8L2E+LlxuXG48YnIgLz48YnIgLz5Zb3UgbmVlZCB0byB1c2UgdHdvIGJyIGVsZW1lbnQgZm9yIG5ldyBwYXJhZ3JhcGhzLiIsInF1ZXN0aW9ucyI6W3siZGVzY3JpcHRpb24iOiJUaGlzIGlzIHRoZSBkZXNjcmlwdGlvbiBvZiB0aGlzIHF1ZXN0aW9uLiBZb3UgY2FuIGhhdmUgbXVsdGlwbGUgcXVlc3Rpb25zLiBZb3UgY2FuIGFkZCBzaW1wbGUgaHRtbCBsaWtlIDxzdHJvbmc+Ym9sZDwvc3Ryb25nPiBvciA8YSBocmVmPVwiaHR0cHM6Ly9zZXF1ZW50ZWNoLmlvXCIgcmVsPVwibm9mb2xsb3dcIj5saW5rcyB0byB3ZWJzaXRlczwvYT4uXG5cbjxiciAvPjxiciAvPllvdSBuZWVkIHRvIHVzZSB0d28gYnIgZWxlbWVudCBmb3IgbmV3IHBhcmFncmFwaHMuIiwibGF5b3V0Ijoic2ltdWx0YW5lb3VzLXF1ZXN0aW9ucyIsIm1heCI6MywibWluIjoxLCJudW1fd2lubmVycyI6MSwidGl0bGUiOiJUZXN0IHF1ZXN0aW9uIHRpdGxlIiwidGFsbHlfdHlwZSI6InBsdXJhbGl0eS1hdC1sYXJnZSIsImFuc3dlcl90b3RhbF92b3Rlc19wZXJjZW50YWdlIjoib3Zlci10b3RhbC12YWxpZC12b3RlcyIsImFuc3dlcnMiOlt7ImlkIjowLCJjYXRlZ29yeSI6IiIsImRldGFpbHMiOiJUaGlzIGlzIGFuIG9wdGlvbiB3aXRoIGFuIHNpbXBsZSBleGFtcGxlIGRlc2NyaXB0aW9uLiIsInNvcnRfb3JkZXIiOjAsInVybHMiOlt7InRpdGxlIjoiSW1hZ2UgVVJMIiwidXJsIjoiaHR0cHM6Ly9pLmltZ3VyLmNvbS9YRlF3VkZMLmpwZyJ9XSwidGV4dCI6IkV4YW1wbGUgb3B0aW9uIDEifSx7ImlkIjoxLCJjYXRlZ29yeSI6IiIsImRldGFpbHMiOiJBbiBvcHRpb24gY2FuIGNvbnRhaW4gYSBkZXNjcmlwdGlvbi4gWW91IGNhbiBhZGQgc2ltcGxlIGh0bWwgbGlrZSA8c3Ryb25nPmJvbGQ8L3N0cm9uZz4gb3IgPGEgaHJlZj1cImh0dHBzOi8vc2VxdWVudGVjaC5pb1wiIHJlbD1cIm5vZm9sbG93XCI+bGlua3MgdG8gd2Vic2l0ZXM8L2E+LiBZb3UgY2FuIGFsc28gc2V0IGFuIGltYWdlIHVybCBiZWxvdywgYnV0IGJlIHN1cmUgaXQmIzM5O3MgSFRUUFMgb3IgZWxzZSBpdCB3b24mIzM5O3QgbG9hZC5cblxuPGJyIC8+PGJyIC8+WW91IG5lZWQgdG8gdXNlIHR3byBiciBlbGVtZW50IGZvciBuZXcgcGFyYWdyYXBocy4iLCJzb3J0X29yZGVyIjoxLCJ1cmxzIjpbeyJ0aXRsZSI6IlVSTCIsInVybCI6Imh0dHBzOi8vc2VxdWVudGVjaC5pbyJ9LHsidGl0bGUiOiJJbWFnZSBVUkwiLCJ1cmwiOiIvWEZRd1ZGTC5qcGcifV0sInRleHQiOiJFeGFtcGxlIG9wdGlvbiAyIn0seyJpZCI6MiwiY2F0ZWdvcnkiOiIiLCJkZXRhaWxzIjoiIiwic29ydF9vcmRlciI6MiwidXJscyI6W10sInRleHQiOiJFeGFtcGxlIG9wdGlvbiAzIn1dLCJleHRyYV9vcHRpb25zIjp7InNodWZmbGVfY2F0ZWdvcmllcyI6dHJ1ZSwic2h1ZmZsZV9hbGxfb3B0aW9ucyI6dHJ1ZSwic2h1ZmZsZV9jYXRlZ29yeV9saXN0IjpbXSwic2hvd19wb2ludHMiOmZhbHNlfX1dLCJwcmVzZW50YXRpb24iOnsic2hhcmVfdGV4dCI6W3sibmV0d29yayI6IlR3aXR0ZXIiLCJidXR0b25fdGV4dCI6IiIsInNvY2lhbF9tZXNzYWdlIjoiSSBoYXZlIGp1c3Qgdm90ZWQgaW4gZWxlY3Rpb24gX19VUkxfXywgeW91IGNhbiB0b28hICNzZXF1ZW50In1dLCJ0aGVtZSI6ImRlZmF1bHQiLCJ1cmxzIjpbXSwidGhlbWVfY3NzIjoiIn0sImV4dHJhX2RhdGEiOiJ7fSIsInRhbGx5UGlwZXNDb25maWciOiJ7XCJ2ZXJzaW9uXCI6XCJtYXN0ZXJcIixcInBpcGVzXCI6W3tcInR5cGVcIjpcInRhbGx5X3BpcGVzLnBpcGVzLnJlc3VsdHMuZG9fdGFsbGllc1wiLFwicGFyYW1zXCI6e319LHtcInR5cGVcIjpcInRhbGx5X3BpcGVzLnBpcGVzLnNvcnQuc29ydF9ub25faXRlcmF0aXZlXCIsXCJwYXJhbXNcIjp7fX1dfSIsImJhbGxvdEJveGVzUmVzdWx0c0NvbmZpZyI6IiIsInZpcnR1YWwiOmZhbHNlLCJ0YWxseV9hbGxvd2VkIjpmYWxzZSwicHVibGljQ2FuZGlkYXRlcyI6dHJ1ZSwidmlydHVhbFN1YmVsZWN0aW9ucyI6W10sImxvZ29fdXJsIjoiIn0sInN0YXRlIjoiY3JlYXRlZCIsInBrcyI6Ilt7XCJxXCI6XCIyNDc5Mjc3NDUwODczNjg4NDY0Mjg2ODY0OTU5NDk4MjgyOTY0NjY3NzA0NDE0MzQ1NjY4NTk2NjkwMjA5MDQ1MDM4OTEyNjkyODEwODgzMTQwMTI2MDU1NjUyMDQxMjYzNTEwNzAxMDU1NzQ3MjAzMzk1OTQxMzE4MjcyMTc0MDM0NDIwMTc0NDQzOTMzMjQ4NTY4NTk2MTQwMzI0MzgzMjA1NTcwMzQ4NTAwNjMzMTYyMjU5NzUxNjcxNDM1MzMzNDQ3NTAwMzM1NjEwNzIxNDQxNTEzMzkzMDUyMTkzMTUwMTMzNTYzNjI2Nzg2MzU0MjM2NTA1MTUzNDI1MDM0NzM3MjM3MTA2NzUzMTQ1NDU2NzI3MjM4NTE4NTg5MTE2Mzk0NTc1NjUyMDg4NzI0OTkwNDY1NDI1ODYzNTM1NDIyNTE4NTE4Mzg4MzA3MjQzNjcwNjY5ODgwMjkxNTQzMDY2NTMzMDMxMDE3MTgxNzE0NzAzMDUxMTI5NjgxNTEzODQwMjYzODQxODE5NzY1MjA3Mjc1ODUyNTkxNTY0MDgwMzA2NjY3OTg4MzMwOTY1NjgyOTUyMTAwMzMxNzk0NTM4OTMxNDQyMjI1NDExMjg0Njk4OTQxMjU3OTE5NjAwMDMxOTM1MjEwNTMyODIzNzczNjcyNzI4NzkzMzc2NTY3NTYyMzg3Mjk1Njc2NTUwMTk4NTU4ODE3MDM4NDE3MTgxMjQ2MzA1Mjg5MzA1NTg0MDEzMjA4OTUzMzk4MDUxMzEyMzU1Nzc3MDcyODQ5MTI4MDEyNDk5NjI2Mjg4MzEwODY1MzcyM1wiLFwicFwiOlwiNDk1ODU1NDkwMTc0NzM3NjkyODU3MzcyOTkxODk5NjU2NTkyOTMzNTQwODgyODY5MTMzNzE5MzM4MDQxODA5MDA3NzgyNTM4NTYyMTc2NjI4MDI1MjExMTMwNDA4MjUyNzAyMTQwMjExMTQ5NDQwNjc5MTg4MjYzNjU0NDM0ODA2ODg0MDM0ODg4Nzg2NjQ5NzEzNzE5MjI4MDY0ODc2NjQxMTE0MDY5NzAwMTI2NjMyNDUxOTUwMzM0Mjg3MDY2Njg5NTAwMDY3MTIyMTQ0Mjg4MzAyNjc4NjEwNDM4NjMwMDI2NzEyNzI1MzU3MjcwODQ3MzAxMDMwNjg1MDA2OTQ3NDQ3NDIxMzUwNjI5MDkxMzQ1NDQ3NzAzNzE3ODIzMjc4OTE1MTMwNDE3NzQ0OTk4MDkzMDg1MTcyNzA3MDg0NTAzNzAzNjc3NjYxNDQ4NzM0MTMzOTc2MDU4MzA4NjEzMzA2NjA2MjAzNDM2MzQyOTQwNjEwMjI1OTM2MzAyNzY4MDUyNzY4MzYzOTUzMDQxNDU1MTcwNTE4MzEyODE2MDYxMzMzNTk3NjY2MTkzMTM2NTkwNDIwMDY2MzU4OTA3Nzg2Mjg4NDQ1MDgyMjU2OTM5Nzg4MjUxNTgzOTIwMDA2Mzg3MDQyMTA2NTY0NzU0NzM0NTQ1NzU4Njc1MzEzNTEyNDc3NDU5MTM1MzEwMDM5NzExNzYzNDA3NjgzNDM2MjQ5MjYxMDU3ODYxMTE2ODAyNjQxNzkwNjc5NjEwMjYyNDcxMTU1NDE0NTY5ODI1NjAyNDk5OTI1MjU3NjYyMTczMDc0NDdcIixcInlcIjpcIjMxOTI1MTU2NjA2MTkxMDgxNjkzNjUwMTQ3MjA4NzU0OTU2ODk1MTA3MTU2MDQ2NTY4ODM3NDg2MTIzNDM5MDM4MjM5OTU0MDM0MjU3OTA2MzEzMzkyNTEyMDU4OTM4OTI0MzQ2NTkzNTU1MDU3NDY2NTE3MDA0NzkwNTQ4Mzg2MDQ0NDAxODUwNTgxMDYxOTU5NjU0OTUxOTU5MDg2NDcxNTYwODA5NzQ1MzQyNjc1MzAyMzk0MjkwNTM4NTY1MTc1MjE2MjUzMDE1MDczNjc4NTYwNTAwNTAyMzk2ODI5Njk2NjExODQ2NzQ3MjE1NDY0OTA5NjM5MjM4MTIwNTE5NjQ3MTY4NzQ1Nzk2MTYyMzk0NzA2NDQ5MDYwMDYwMDAzNzU0MjA2NTAyOTMxODg0MzE2Mzc2MzU1OTkyNjgwOTgyOTc2ODA1MTcxNjg4NzAwMzEwMjYzMzU3NzQwMDI1MDQ4NDI0NjgyNzU3ODc0NTg1MTExNTA5ODUwMDg1NTU2OTYxOTM5ODM0NDUzOTkxMTg3NTI1Nzk0MTIxMzQxNjA0NjM1NDkxODI3NjEyMDgwODEzNzYwMzQwMDMxODUwNzk0NDUyNTkwNzE5OTk1MDY5MTUwMDU1NTgzMTEzNjE2Njk3Njg2NDEzMjExNDIxODM0NTQ5NTIzMTE3NzU5ODgyNjU5NjY0MDA3NTM5NTAzNjU3NjE4MTYyOTU3NDQ0NTc3OTUwNjYzNzQ3NzMwMTU3NDExOTYxMjgxOTE4NjU4NjMwMjcwNjk0MDkyODQ5ODY4MTY3OTEwMTY5MjI2NTc3Mjg3ODdcIixcImdcIjpcIjI3MjU3NDY5MzgzNDMzNDY4MzA3ODUxODIxMjMyMzM2MDI5MDA4Nzk3OTYzNDQ2NTE2MjY2ODY4Mjc4NDc2NTk4OTkxNjE5Nzk5NzE4NDE2MTE5MDUwNjY5MDMyMDQ0ODYxNjM1OTc3MjE2NDQ1MDM0MDU0NDE0MTQ5Nzk1NDQzNDY2NjE2NTMyNjU3NzM1NjI0NDc4MjA3NDYwNTc3NTkwODkxMDc5Nzk1NTY0MTE0OTEyNDE4NDQyMzk2NzA3ODY0OTk1OTM4NTYzMDY3NzU1NDc5NTYzODUwNDc0ODcwNzY2MDY3MDMxMzI2NTExNDcxMDUxNTA0NTk0Nzc3OTI4MjY0MDI3MTc3MzA4NDUzNDQ2Nzg3NDc4NTg3NDQyNjYzNTU0MjAzMDM5MzM3OTAyNDczODc5NTAyOTE3MjkyNDAzNTM5ODIwODc3OTU2MjUxNDcxNjEyNzAxMjAzNTcyMTQzOTcyMzUyOTQzNzUzNzkxMDYyNjk2NzU3NzkxNjY3MzE4NDg2MTkwMTU0NjEwNzc3NDc1NzIxNzUyNzQ5NTY3OTc1MDEzMTAwODQ0MDMyODUzNjAwMTIwMTk1NTM0MjU5ODAyMDE3MDkwMjgxOTAwMjY0NjQ2MjIwNzgxMjI0MTM2NDQzNzAwNTIxNDE5MzkzMjQ1MDU4NDIxNzE4NDU1MDM0MzMwMTc3NzM5NjEyODk1NDk0NTUzMDY5NDUwNDM4MzE3ODkzNDA2MDI3NzQxMDQ1NTc1ODIxMjgzNDExODkxNTM1NzEzNzkzNjM5MTIzMTA5OTMzMTk2NTQ0MDE3MzA5MTQ3XCJ9XSIsInRhbGx5UGlwZXNDb25maWciOiJ7XCJ2ZXJzaW9uXCI6XCJtYXN0ZXJcIixcInBpcGVzXCI6W3tcInR5cGVcIjpcInRhbGx5X3BpcGVzLnBpcGVzLnJlc3VsdHMuZG9fdGFsbGllc1wiLFwicGFyYW1zXCI6e319LHtcInR5cGVcIjpcInRhbGx5X3BpcGVzLnBpcGVzLnNvcnQuc29ydF9ub25faXRlcmF0aXZlXCIsXCJwYXJhbXNcIjp7fX1dfSIsImJhbGxvdEJveGVzUmVzdWx0c0NvbmZpZyI6IiIsInZpcnR1YWwiOmZhbHNlLCJ0YWxseUFsbG93ZWQiOmZhbHNlLCJwdWJsaWNDYW5kaWRhdGVzIjp0cnVlLCJsb2dvX3VybCI6IiIsInRydXN0ZWVLZXlzU3RhdGUiOlt7ImlkIjoiNnh4LWExIiwic3RhdGUiOiJpbml0aWFsIn0seyJpZCI6IjZ4eC1hMiIsInN0YXRlIjoiaW5pdGlhbCJ9XX0=', E'2023-08-10T22:12:17.057014+00:00', E'2023-08-10T22:12:17.057014+00:00', E'2f312a36-f39c-46e4-9670-1d1ce4625745', E'33f18502-a67c-4853-8333-a58630663559', E'f2f1065e-b784-46d1-b81a-c71bfeb9ad55', E'6e5bbff4-0fb8-4971-a808-37de49573f6a', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

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
