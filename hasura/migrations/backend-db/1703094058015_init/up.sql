

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

alter table "sequent_backend"."trustee" add column "tenant_id" uuid
 null;

alter table "sequent_backend"."trustee"
  add constraint "trustee_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."trustee" add column "is_protocol_manager" boolean
 null default 'false';

alter table "sequent_backend"."trustee" drop column "is_protocol_manager" cascade;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_tenant_id_election_event_id_area_id_fkey"
  foreign key ("tenant_id", "election_event_id", "area_id")
  references "sequent_backend"."area"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."election_event" add column "public_key" text
 null;

alter table "sequent_backend"."scheduled_event" add column "task_id" varchar
 null;

alter table "sequent_backend"."ballot_style" add column "deleted_at" timestamptz
 null;

CREATE TABLE "sequent_backend"."tally" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "created_at" timestamptz DEFAULT now(), "last_updated_at" timestamptz DEFAULT now(), "labels" jsonb, "annotations" jsonb, "area_ids" uuid[], PRIMARY KEY ("id","tenant_id","election_event_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."tally_contest" ("id" uuid NOT NULL, "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "area_id" uuid NOT NULL, "contest_id" uuid NOT NULL, "tally_id" uuid NOT NULL, "session_id" integer NOT NULL, "document_id" uuid, "created_at" timestamptz DEFAULT now(), "last_updated_at" timestamptz DEFAULT now(), "labels" jsonb, "annotations" jsonb, PRIMARY KEY ("id","tenant_id","election_event_id") , FOREIGN KEY ("document_id") REFERENCES "sequent_backend"."document"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("tally_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."tally"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("area_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."area"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("contest_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."contest"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict);

alter table "sequent_backend"."tally" rename to "tally_session";

alter table "sequent_backend"."tally_contest" rename to "tally_session_contest";

alter table "sequent_backend"."tally_session" add column "election_ids" uuid
 null;

alter table "sequent_backend"."tally_session" add column "trustee_ids" uuid
 null;

alter table "sequent_backend"."tally_session" drop column "area_ids" cascade;

alter table "sequent_backend"."tally_session" drop column "election_ids" cascade;

alter table "sequent_backend"."tally_session" drop column "trustee_ids" cascade;

alter table "sequent_backend"."tally_session" add column "election_ids" UUID[]
 null;

alter table "sequent_backend"."tally_session" add column "trustee_ids" UUID[]
 null;

alter table "sequent_backend"."tally_session" add column "area_ids" UUID[]
 null;

alter table "sequent_backend"."tally_session_contest" alter column "id" set default gen_random_uuid();

alter table "sequent_backend"."tally_session_contest" add column "tally_session_id" uuid
 not null;

alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_session_contest_election_event_id_tenant_id_tally_sess"
  foreign key ("election_event_id", "tenant_id", "tally_session_id")
  references "sequent_backend"."tally_session"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."tally_session_contest" drop column "tally_id" cascade;

INSERT INTO "sequent_backend"."trustee"("name", "annotations", "labels", "public_key", "created_at", "last_updated_at", "id", "tenant_id") VALUES (E'trustee1', null, null, E'MCowBQYDK2VwAyEAy1vJM4P85hJ1WAPZpRX3/QsOT2usIAuVy4/+t5VHHDs=', E'2023-11-02T23:05:33.172417+00:00', E'2023-11-02T23:05:33.172417+00:00', E'7a53083a-9284-4b1b-8db8-aadfd6bc6a02', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

INSERT INTO "sequent_backend"."trustee"("name", "annotations", "labels", "public_key", "created_at", "last_updated_at", "id", "tenant_id") VALUES (E'trustee2', null, null, E'MCowBQYDK2VwAyEA50mtZzCBnubUwMhRkKyGomrUCBGgvEsbu79D3Cckjbc=', E'2023-11-02T23:06:03.60716+00:00', E'2023-11-02T23:06:03.60716+00:00', E'b84015c2-2efd-47de-a222-a8b7bf8d4782', E'90505c8a-23a9-4cdf-a26b-4e19f6a097d5');

alter table "sequent_backend"."contest" add column "winning_candidates_num" integer
 null;

CREATE TABLE "sequent_backend"."tally_session_execution" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "created_at" timestamptz DEFAULT now(), "last_updated_at" timestamptz DEFAULT now(), "labels" jsonb, "annotations" jsonb, "current_message_id" integer NOT NULL, PRIMARY KEY ("id","tenant_id","election_event_id") , FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."tally_session_execution" add column "tally_session_id" uuid
 not null;

alter table "sequent_backend"."tally_session_execution"
  add constraint "tally_session_execution_election_event_id_tenant_id_tally_se"
  foreign key ("election_event_id", "tenant_id", "tally_session_id")
  references "sequent_backend"."tally_session"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."tally_session" add column "is_execution_completed" boolean
 not null default 'false';

alter table "sequent_backend"."tally_session_contest" drop constraint "tally_contest_document_id_fkey";

alter table "sequent_backend"."tally_session_contest" drop column "document_id" cascade;

alter table "sequent_backend"."tally_session_execution" add column "document_id" uuid
 not null;

alter table "sequent_backend"."tally_session_execution"
  add constraint "tally_session_execution_document_id_fkey"
  foreign key ("document_id")
  references "sequent_backend"."document"
  ("id") on update restrict on delete restrict;

CREATE TABLE "sequent_backend"."lock" ("key" text NOT NULL, "value" text NOT NULL, "expiry_date" timestamptz, PRIMARY KEY ("key") , UNIQUE ("key"));

alter table "sequent_backend"."lock" add column "created_at" timestamptz
 not null default now();

alter table "sequent_backend"."lock" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."tenant" rename column "username" to "slug";

alter table "sequent_backend"."tenant" add constraint "tenant_slug_key" unique ("slug");

alter table "sequent_backend"."election_event" add column "alias" text
 null;

alter table "sequent_backend"."election" add column "alias" text
 null;

alter table "sequent_backend"."election" add column "voting_channels" jsonb
 null;

alter table "sequent_backend"."candidate" add column "alias" text
 null;

alter table "sequent_backend"."contest" add column "orser_answers" text
 null;

alter table "sequent_backend"."contest" rename column "orser_answers" to "order_answers";

alter table "sequent_backend"."election" add column "is_kiosk" boolean
 null default 'FALSE';

alter table "sequent_backend"."tenant" add column "voting_channels" jsonb
 null;

CREATE TABLE "sequent_backend"."election_type" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "created_at" timestamptz NOT NULL DEFAULT now(), "updated_at" timestamptz NOT NULL DEFAULT now(), "labels" jsonb, "annotations" jsonb, "name" text NOT NULL, PRIMARY KEY ("id","tenant_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."document" add column "is_public" boolean
 null default 'FALSE';

alter table "sequent_backend"."election" add column "image_document_id" text
 null;

alter table "sequent_backend"."election" add column "image_name" text
 null;

alter table "sequent_backend"."election" drop column "image_name" cascade;

alter table "sequent_backend"."contest" add column "image_document_id" text
 null;

alter table "sequent_backend"."candidate" add column "image_document_id" text
 null;

CREATE TABLE "sequent_backend"."keys_ceremony" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "created_at" timestamptz NOT NULL DEFAULT now(), "updated_at" timestamptz NOT NULL DEFAULT now(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "trustee_ids" uuid[] NOT NULL, "status" jsonb, "execution_status" text, PRIMARY KEY ("id","tenant_id","election_event_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict);
CREATE OR REPLACE FUNCTION "sequent_backend"."set_current_timestamp_updated_at"()
RETURNS TRIGGER AS $$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."updated_at" = NOW();
  RETURN _new;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER "set_sequent_backend_keys_ceremony_updated_at"
BEFORE UPDATE ON "sequent_backend"."keys_ceremony"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_updated_at"();
COMMENT ON TRIGGER "set_sequent_backend_keys_ceremony_updated_at" ON "sequent_backend"."keys_ceremony"
IS 'trigger to set value of column "updated_at" to current timestamp on row update';
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."keys_ceremony" rename column "updated_at" to "last_updated_at";

alter table "sequent_backend"."keys_ceremony" add column "labels" jsonb
 null;

alter table "sequent_backend"."keys_ceremony" add column "annotations" jsonb
 null;

alter table "sequent_backend"."tenant" add column "settings" jsonb
 null;

alter table "sequent_backend"."tally_session_execution" add column "session_ids" integer
 null;

alter table "sequent_backend"."tally_session_execution" drop column "session_ids" cascade;

alter table "sequent_backend"."tally_session_execution" add column "session_ids" Integer[]
 null;

ALTER TABLE "sequent_backend"."tally_session_execution" ALTER COLUMN "session_ids" TYPE int4[];

alter table "sequent_backend"."keys_ceremony" add column "threshold" integer
 not null;

alter table "sequent_backend"."tally_session" add column "keys_ceremony_id" uuid
 not null;

alter table "sequent_backend"."tally_session"
  add constraint "tally_session_election_event_id_tenant_id_keys_ceremony_id_f"
  foreign key ("election_event_id", "tenant_id", "keys_ceremony_id")
  references "sequent_backend"."keys_ceremony"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

CREATE TABLE "sequent_backend"."results_event" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "name" text, PRIMARY KEY ("id","tenant_id","election_event_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."results_election" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "election_id" uuid NOT NULL, "results_event_id" uuid NOT NULL, "name" text, "elegible_census" integer, "total_valid_votes" integer, "explicit_invalid_votes" integer, "implicit_invalid_votes" integer, "blank_votes" integer, PRIMARY KEY ("id","tenant_id","election_event_id","election_id","results_event_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."election"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("results_event_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."results_event"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."results_contest" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "election_id" uuid NOT NULL, "contest_id" uuid NOT NULL, "results_event_id" uuid NOT NULL, "elegible_census" integer, "total_valid_votes" integer, "explicit_invalid_votes" integer, "implicit_invalid_votes" integer, "blank_votes" integer, "voting_type" text, "counting_algorithm" text, "name" text, PRIMARY KEY ("id","tenant_id","election_event_id","election_id","contest_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."election"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("contest_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."contest"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("results_event_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."results_event"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."results_area_contest" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "election_id" uuid NOT NULL, "contest_id" uuid NOT NULL, "area_id" uuid NOT NULL, "results_event_id" uuid NOT NULL, "elegible_census" integer, "total_valid_votes" integer, "explicit_invalid_votes" integer, "implicit_invalid_votes" integer, "blank_votes" integer, PRIMARY KEY ("id","tenant_id","election_event_id","election_id","contest_id","area_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("tenant_id", "id", "election_event_id") REFERENCES "sequent_backend"."election"("tenant_id", "id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("contest_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."contest"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("area_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."area"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("results_event_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."results_event"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE "sequent_backend"."results_area_contest_candidate" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "election_id" uuid NOT NULL, "contest_id" uuid NOT NULL, "area_id" uuid NOT NULL, "candidate_id" uuid NOT NULL, "results_event_id" uuid NOT NULL, "cast_votes" integer, "winning_position" integer, "points" integer, PRIMARY KEY ("id","tenant_id","election_event_id","results_event_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."election"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("contest_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."contest"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("area_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."area"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("candidate_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."candidate"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("results_event_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."results_event"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."results_area_contest" DROP CONSTRAINT "results_area_contest_pkey";

ALTER TABLE "sequent_backend"."results_area_contest"
    ADD CONSTRAINT "results_area_contest_pkey" PRIMARY KEY ("id", "tenant_id", "election_event_id", "results_event_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."results_contest" DROP CONSTRAINT "results_contest_pkey";

ALTER TABLE "sequent_backend"."results_contest"
    ADD CONSTRAINT "results_contest_pkey" PRIMARY KEY ("tenant_id", "id", "election_event_id", "results_event_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."results_election" DROP CONSTRAINT "results_election_pkey";

ALTER TABLE "sequent_backend"."results_election"
    ADD CONSTRAINT "results_election_pkey" PRIMARY KEY ("election_event_id", "results_event_id", "id", "tenant_id");
COMMIT TRANSACTION;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."results_election" DROP CONSTRAINT "results_election_pkey";

ALTER TABLE "sequent_backend"."results_election"
    ADD CONSTRAINT "results_election_pkey" PRIMARY KEY ("election_event_id", "results_event_id", "id", "tenant_id");
COMMIT TRANSACTION;

CREATE TABLE "sequent_backend"."results_contest_candidate" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "election_id" uuid NOT NULL, "contest_id" uuid NOT NULL, "candidate_id" uuid NOT NULL, "results_event_id" uuid NOT NULL, "cast_votes" integer, "winning_position" integer, "points" integer, PRIMARY KEY ("id","tenant_id","election_event_id","results_event_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."election"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("contest_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."contest"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("candidate_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."candidate"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("results_event_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."results_event"("id", "tenant_id", "election_event_id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."results_event" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."results_event" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."results_event" add column "annotations" jsonb
 null;

alter table "sequent_backend"."results_event" add column "labels" jsonb
 null;

alter table "sequent_backend"."results_election" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."results_election" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."results_election" add column "labels" jsonb
 null;

alter table "sequent_backend"."results_election" add column "annotations" jsonb
 null;

alter table "sequent_backend"."results_contest_candidate" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."results_contest_candidate" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."results_contest_candidate" add column "labels" jsonb
 null;

alter table "sequent_backend"."results_contest_candidate" add column "annotations" jsonb
 null;

alter table "sequent_backend"."results_contest" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."results_contest" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."results_contest" add column "labels" jsonb
 null;

alter table "sequent_backend"."results_contest" add column "annotations" jsonb
 null;

alter table "sequent_backend"."results_area_contest_candidate" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."results_area_contest_candidate" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."results_area_contest_candidate" add column "labels" jsonb
 null;

alter table "sequent_backend"."results_area_contest_candidate" add column "annotations" jsonb
 null;

alter table "sequent_backend"."results_area_contest" add column "created_at" timestamptz
 null default now();

alter table "sequent_backend"."results_area_contest" add column "last_updated_at" timestamptz
 null default now();

alter table "sequent_backend"."results_area_contest" add column "labels" jsonb
 null;

alter table "sequent_backend"."results_area_contest" add column "annotations" jsonb
 null;

alter table "sequent_backend"."tally_session" add column "status" text
 null;

alter table "sequent_backend"."tally_session" add column "execution_status" text
 null;

alter table "sequent_backend"."tally_session" drop column "status" cascade;

alter table "sequent_backend"."tally_session" add column "status" jsonb
 null;

CREATE TABLE "sequent_backend"."communication_template" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "template" jsonb NOT NULL, "created_by" text NOT NULL, "labels" jsonb, "annotations" jsonb, "created_at" timestamptz NOT NULL DEFAULT now(), "updated_at" timestamptz NOT NULL DEFAULT now(), PRIMARY KEY ("id","tenant_id","election_event_id") );
CREATE OR REPLACE FUNCTION "sequent_backend"."set_current_timestamp_updated_at"()
RETURNS TRIGGER AS $$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."updated_at" = NOW();
  RETURN _new;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER "set_sequent_backend_communication_template_updated_at"
BEFORE UPDATE ON "sequent_backend"."communication_template"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_updated_at"();
COMMENT ON TRIGGER "set_sequent_backend_communication_template_updated_at" ON "sequent_backend"."communication_template"
IS 'trigger to set value of column "updated_at" to current timestamp on row update';
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."communication_template" add column "communication_method" text
 not null;

alter table "sequent_backend"."communication_template"
  add constraint "communication_template_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."communication_template"
  add constraint "communication_template_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."communication_template" add column "communication_type" text
 not null;

BEGIN TRANSACTION;
ALTER TABLE "sequent_backend"."communication_template" DROP CONSTRAINT "communication_template_pkey";

ALTER TABLE "sequent_backend"."communication_template"
    ADD CONSTRAINT "communication_template_pkey" PRIMARY KEY ("id", "tenant_id");
COMMIT TRANSACTION;

alter table "sequent_backend"."communication_template" drop constraint "communication_template_election_event_id_fkey";

alter table "sequent_backend"."communication_template" drop column "election_event_id" cascade;

alter table "sequent_backend"."tally_session" drop column "trustee_ids" cascade;

alter table "sequent_backend"."tally_session_execution" alter column "document_id" drop not null;

alter table "sequent_backend"."tally_session_execution" add column "status" jsonb
 null;

alter table "sequent_backend"."tally_session_execution" add column "results_event_id" uuid
 null;

alter table "sequent_backend"."tally_session_execution"
  add constraint "tally_session_execution_election_event_id_tenant_id_results_"
  foreign key ("election_event_id", "tenant_id", "results_event_id")
  references "sequent_backend"."results_event"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."tally_session" drop column "status" cascade;

CREATE TABLE "sequent_backend"."ballot_publication" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "labels" jsonb, "annotations" jsonb, "created_at" timestamptz NOT NULL DEFAULT now(), "deleted_at" timestamptz, "created_by_user_id" text, PRIMARY KEY ("id","tenant_id","election_event_id") , FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict, FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE restrict);
CREATE EXTENSION IF NOT EXISTS pgcrypto;

alter table "sequent_backend"."ballot_style" add column "ballot_publication_id" uuid
 not null;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_event_id_tenant_id_ballot_publication_"
  foreign key ("election_event_id", "tenant_id", "ballot_publication_id")
  references "sequent_backend"."ballot_publication"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_publication" add column "is_published" boolean
 not null default 'false';

alter table "sequent_backend"."ballot_publication" add column "election_ids" UUID[]
 null;

DROP TRIGGER "set_sequent_backend_keys_ceremony_updated_at" ON "sequent_backend"."keys_ceremony";

alter table "sequent_backend"."ballot_publication" rename column "is_published" to "is_generated";

alter table "sequent_backend"."ballot_publication" add column "published_at" timestamptz
 null;

CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_updated_at()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."updated_at" = NOW();
  RETURN _new;
END;
$function$;

CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_last_updated_at()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."last_updated_at" = NOW();
  RETURN _new;
END;
$function$;

CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_updated_at()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."updated_at" = NOW();
  RETURN _new;
END;
$function$;

CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_last_updated_at()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."last_updated_at" = NOW();
  RETURN _new;
END;
$function$;

CREATE TRIGGER "set_sequent_backend_keys_ceremony_last_updated_at"
BEFORE UPDATE ON "sequent_backend"."keys_ceremony"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_last_updated_at"();

alter table "sequent_backend"."results_area_contest" drop constraint "results_area_contest_tenant_id_id_election_event_id_fkey",
  add constraint "results_area_contest_tenant_id_election_event_id_election_id"
  foreign key ("tenant_id", "election_event_id", "election_id")
  references "sequent_backend"."election"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."tally_session" add column "threshold" integer
 not null;

alter table "sequent_backend"."election_event" add column "statistics" jsonb
 null default jsonb_build_object();

alter table "sequent_backend"."election" add column "statistics" jsonb
 null default jsonb_build_object();

alter table "sequent_backend"."tally_session_contest" add column "election_id" uuid
 not null;

alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_session_contest_tenant_id_election_event_id_election_i"
  foreign key ("tenant_id", "election_event_id", "election_id")
  references "sequent_backend"."election"
  ("tenant_id", "election_event_id", "id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_publication" add column "is_election_event" boolean
 null;

alter table "sequent_backend"."ballot_publication" drop column "is_election_event" cascade;

alter table "sequent_backend"."ballot_publication" add column "election_id" uuid
 null;

alter table "sequent_backend"."cast_vote" add column "ballot_id" text
 null;
