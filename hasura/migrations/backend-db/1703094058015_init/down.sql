
-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "ballot_id" text
--  null unique;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_publication" add column "election_id" uuid
--  null;

alter table "sequent_backend"."ballot_publication" alter column "is_election_event" drop not null;
alter table "sequent_backend"."ballot_publication" add column "is_election_event" bool;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_publication" add column "is_election_event" boolean
--  null;

alter table "sequent_backend"."tally_session_contest" drop constraint "tally_session_contest_tenant_id_election_event_id_election_i";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_contest" add column "election_id" uuid
--  not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "statistics" jsonb
--  null default jsonb_build_object();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "statistics" jsonb
--  null default jsonb_build_object();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "threshold" integer
--  not null;

alter table "sequent_backend"."results_area_contest" drop constraint "results_area_contest_tenant_id_election_event_id_election_id",
  add constraint "results_area_contest_tenant_id_id_election_event_id_fkey"
  foreign key ("id", "election_event_id", "tenant_id")
  references "sequent_backend"."election"
  ("id", "election_event_id", "tenant_id") on update restrict on delete restrict;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_updated_at()
--  RETURNS trigger
--  LANGUAGE plpgsql
-- AS $function$
-- DECLARE
--   _new record;
-- BEGIN
--   _new := NEW;
--   _new."updated_at" = NOW();
--   RETURN _new;
-- END;
-- $function$;
--
-- CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_last_updated_at()
--  RETURNS trigger
--  LANGUAGE plpgsql
-- AS $function$
-- DECLARE
--   _new record;
-- BEGIN
--   _new := NEW;
--   _new."last_updated_at" = NOW();
--   RETURN _new;
-- END;
-- $function$;
--
-- CREATE TRIGGER "set_sequent_backend_keys_ceremony_last_updated_at"
-- BEFORE UPDATE ON "sequent_backend"."keys_ceremony"
-- FOR EACH ROW
-- EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_last_updated_at"();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_updated_at()
--  RETURNS trigger
--  LANGUAGE plpgsql
-- AS $function$
-- DECLARE
--   _new record;
-- BEGIN
--   _new := NEW;
--   _new."updated_at" = NOW();
--   RETURN _new;
-- END;
-- $function$;
--
-- CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_last_updated_at()
--  RETURNS trigger
--  LANGUAGE plpgsql
-- AS $function$
-- DECLARE
--   _new record;
-- BEGIN
--   _new := NEW;
--   _new."last_updated_at" = NOW();
--   RETURN _new;
-- END;
-- $function$;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_publication" add column "published_at" timestamptz
--  null;

alter table "sequent_backend"."ballot_publication" rename column "is_generated" to "is_published";

CREATE TRIGGER "set_sequent_backend_keys_ceremony_updated_at"
BEFORE UPDATE ON "sequent_backend"."keys_ceremony"
FOR EACH ROW EXECUTE FUNCTION sequent_backend.set_current_timestamp_updated_at();COMMENT ON TRIGGER "set_sequent_backend_keys_ceremony_updated_at" ON "sequent_backend"."keys_ceremony"
IS E'trigger to set value of column "updated_at" to current timestamp on row update';

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_publication" add column "election_ids" UUID[]
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_publication" add column "is_published" boolean
--  not null default 'false';

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_tenant_id_ballot_publication_";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "ballot_publication_id" uuid
--  not null;

DROP TABLE "sequent_backend"."ballot_publication";

alter table "sequent_backend"."tally_session" alter column "status" drop not null;
alter table "sequent_backend"."tally_session" add column "status" jsonb;

alter table "sequent_backend"."tally_session_execution" drop constraint "tally_session_execution_election_event_id_tenant_id_results_";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_execution" add column "results_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_execution" add column "status" jsonb
--  null;

alter table "sequent_backend"."tally_session_execution" alter column "document_id" set not null;

alter table "sequent_backend"."tally_session" alter column "trustee_ids" drop not null;
alter table "sequent_backend"."tally_session" add column "trustee_ids" _uuid;

alter table "sequent_backend"."communication_template" alter column "election_event_id" drop not null;
alter table "sequent_backend"."communication_template" add column "election_event_id" uuid;

alter table "sequent_backend"."communication_template"
  add constraint "communication_template_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."communication_template" drop constraint "communication_template_pkey";
alter table "sequent_backend"."communication_template"
    add constraint "communication_template_pkey"
    primary key ("id", "election_event_id", "tenant_id");

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."communication_template" add column "communication_type" text
--  not null;

alter table "sequent_backend"."communication_template" drop constraint "communication_template_tenant_id_fkey";

alter table "sequent_backend"."communication_template" drop constraint "communication_template_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."communication_template" add column "communication_method" text
--  not null;

DROP TABLE "sequent_backend"."communication_template";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "status" jsonb
--  null;

alter table "sequent_backend"."tally_session" alter column "status" drop not null;
alter table "sequent_backend"."tally_session" add column "status" text;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "execution_status" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "status" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest_candidate" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest_candidate" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest_candidate" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest_candidate" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest_candidate" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest_candidate" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest_candidate" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest_candidate" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_election" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_election" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_election" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_election" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_event" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_event" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_event" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_event" add column "created_at" timestamptz
--  null default now();

DROP TABLE "sequent_backend"."results_contest_candidate";

alter table "sequent_backend"."results_election" drop constraint "results_election_pkey";
alter table "sequent_backend"."results_election"
    add constraint "results_election_pkey"
    primary key ("results_event_id", "id", "tenant_id", "election_event_id");

alter table "sequent_backend"."results_election" drop constraint "results_election_pkey";
alter table "sequent_backend"."results_election"
    add constraint "results_election_pkey"
    primary key ("election_event_id", "election_id", "results_event_id", "id", "tenant_id");

alter table "sequent_backend"."results_contest" drop constraint "results_contest_pkey";
alter table "sequent_backend"."results_contest"
    add constraint "results_contest_pkey"
    primary key ("tenant_id", "contest_id", "election_id", "id", "election_event_id");

alter table "sequent_backend"."results_area_contest" drop constraint "results_area_contest_pkey";
alter table "sequent_backend"."results_area_contest"
    add constraint "results_area_contest_pkey"
    primary key ("id", "tenant_id", "election_event_id", "election_id", "contest_id", "area_id");

DROP TABLE "sequent_backend"."results_area_contest_candidate";

DROP TABLE "sequent_backend"."results_area_contest";

DROP TABLE "sequent_backend"."results_contest";

DROP TABLE "sequent_backend"."results_election";

DROP TABLE "sequent_backend"."results_event";

alter table "sequent_backend"."tally_session" drop constraint "tally_session_election_event_id_tenant_id_keys_ceremony_id_f";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "keys_ceremony_id" uuid
--  not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."keys_ceremony" add column "threshold" integer
--  not null;

ALTER TABLE "sequent_backend"."tally_session_execution" ALTER COLUMN "session_ids" TYPE ARRAY;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_execution" add column "session_ids" Integer[]
--  null;

alter table "sequent_backend"."tally_session_execution" alter column "session_ids" drop not null;
alter table "sequent_backend"."tally_session_execution" add column "session_ids" int4;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_execution" add column "session_ids" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tenant" add column "settings" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."keys_ceremony" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."keys_ceremony" add column "labels" jsonb
--  null;

alter table "sequent_backend"."keys_ceremony" rename column "last_updated_at" to "updated_at";

DROP TABLE "sequent_backend"."keys_ceremony";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "image_document_id" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "image_document_id" text
--  null;

alter table "sequent_backend"."election" alter column "image_name" drop not null;
alter table "sequent_backend"."election" add column "image_name" text;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "image_name" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "image_document_id" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."document" add column "is_public" boolean
--  null default 'FALSE';

DROP TABLE "sequent_backend"."election_type";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tenant" add column "voting_channels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "is_kiosk" boolean
--  null default 'FALSE';

alter table "sequent_backend"."contest" rename column "order_answers" to "orser_answers";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "orser_answers" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "alias" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "voting_channels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "alias" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "alias" text
--  null;

alter table "sequent_backend"."tenant" drop constraint "tenant_slug_key";

alter table "sequent_backend"."tenant" rename column "slug" to "username";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."lock" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."lock" add column "created_at" timestamptz
--  not null default now();

DROP TABLE "sequent_backend"."lock";

alter table "sequent_backend"."tally_session_execution" drop constraint "tally_session_execution_document_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_execution" add column "document_id" uuid
--  not null;

alter table "sequent_backend"."tally_session_contest" alter column "document_id" drop not null;
alter table "sequent_backend"."tally_session_contest" add column "document_id" uuid;

alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_contest_document_id_fkey"
  foreign key ("document_id")
  references "sequent_backend"."document"
  ("id") on update restrict on delete restrict;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "is_execution_completed" boolean
--  not null default 'false';

alter table "sequent_backend"."tally_session_execution" drop constraint "tally_session_execution_election_event_id_tenant_id_tally_se";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_execution" add column "tally_session_id" uuid
--  not null;

DROP TABLE "sequent_backend"."tally_session_execution";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "winning_candidates_num" integer
--  null;

DELETE FROM "sequent_backend"."trustee" WHERE "id" = 'b84015c2-2efd-47de-a222-a8b7bf8d4782';

DELETE FROM "sequent_backend"."trustee" WHERE "id" = '7a53083a-9284-4b1b-8db8-aadfd6bc6a02';

alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_contest_tally_id_tenant_id_election_event_id_fkey"
  foreign key (election_event_id, tally_id, tenant_id)
  references "sequent_backend"."tally_session"
  (election_event_id, id, tenant_id) on update restrict on delete restrict;
alter table "sequent_backend"."tally_session_contest" alter column "tally_id" drop not null;
alter table "sequent_backend"."tally_session_contest" add column "tally_id" uuid;

alter table "sequent_backend"."tally_session_contest" drop constraint "tally_session_contest_election_event_id_tenant_id_tally_sess";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session_contest" add column "tally_session_id" uuid
--  not null;

ALTER TABLE "sequent_backend"."tally_session_contest" ALTER COLUMN "id" drop default;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "area_ids" UUID[]
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "trustee_ids" UUID[]
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "election_ids" UUID[]
--  null;

alter table "sequent_backend"."tally_session" alter column "trustee_ids" drop not null;
alter table "sequent_backend"."tally_session" add column "trustee_ids" uuid;

alter table "sequent_backend"."tally_session" alter column "election_ids" drop not null;
alter table "sequent_backend"."tally_session" add column "election_ids" uuid;

alter table "sequent_backend"."tally_session" alter column "area_ids" drop not null;
alter table "sequent_backend"."tally_session" add column "area_ids" _uuid;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "trustee_ids" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "election_ids" uuid
--  null;

alter table "sequent_backend"."tally_session_contest" rename to "tally_contest";

alter table "sequent_backend"."tally_session" rename to "tally";

DROP TABLE "sequent_backend"."tally_contest";

DROP TABLE "sequent_backend"."tally";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "deleted_at" timestamptz
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."scheduled_event" add column "task_id" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "public_key" text
--  null;

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_tenant_id_election_event_id_area_id_fkey";

alter table "sequent_backend"."trustee" alter column "is_protocol_manager" set default false;
alter table "sequent_backend"."trustee" alter column "is_protocol_manager" drop not null;
alter table "sequent_backend"."trustee" add column "is_protocol_manager" bool;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."trustee" add column "is_protocol_manager" boolean
--  null default 'false';

alter table "sequent_backend"."trustee" drop constraint "trustee_tenant_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."trustee" add column "tenant_id" uuid
--  null;


alter table "sequent_backend"."scheduled_event" alter column "board_id" drop not null;
alter table "sequent_backend"."scheduled_event" add column "board_id" int4;

DROP TABLE "sequent_backend"."trustee";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."scheduled_event" add column "board_id" integer
--  null;

ALTER TABLE "sequent_backend"."event_execution" ALTER COLUMN "id" drop default;

alter table "sequent_backend"."scheduled_event" rename column "created_by" to "created_nby";

DROP TABLE "sequent_backend"."event_execution";

DROP TABLE "sequent_backend"."scheduled_event";

alter table "sequent_backend"."document" rename to "election_document";

DROP TABLE "sequent_backend"."election_document";

DELETE FROM "sequent_backend"."tenant" WHERE "id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_contest_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_pkey";
alter table "sequent_backend"."contest"
    add constraint "contest_pkey"
    primary key ("election_id", "tenant_id", "id");

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_contest_id_election_event_id_fkey"
  foreign key ("election_event_id", "contest_id", "tenant_id")
  references "sequent_backend"."contest"
  ("election_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_election_event_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_election_event_id_area_id_fkey",
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."contest" drop constraint "contest_tenant_id_fkey",
  add constraint "contest_id_fkey"
  foreign key ("id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" rename column "content" to "cast_ballot_eml";

alter table "sequent_backend"."election_result" drop constraint "election_result_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_pkey";
alter table "sequent_backend"."election"
    add constraint "election_pkey"
    primary key ("tenant_id", "id", "election_event_id");

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_id", "tenant_id", "election_event_id")
  references "sequent_backend"."election"
  ("id", "tenant_id", "election_event_id") on update restrict on delete restrict;

alter table "sequent_backend"."contest"
  add constraint "contest_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "election_id", "tenant_id")
  references "sequent_backend"."election"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "election_id", "tenant_id")
  references "sequent_backend"."election"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_id", "tenant_id", "election_event_id")
  references "sequent_backend"."election"
  ("id", "tenant_id", "election_event_id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_pkey";
alter table "sequent_backend"."cast_vote"
    add constraint "cast_vote_pkey"
    primary key ("election_event_id", "id", "tenant_id");

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_pkey";
alter table "sequent_backend"."ballot_style"
    add constraint "ballot_style_pkey"
    primary key ("id", "election_id", "tenant_id");

alter table "sequent_backend"."candidate" drop constraint "candidate_pkey";
alter table "sequent_backend"."candidate"
    add constraint "candidate_pkey"
    primary key ("election_event_id", "tenant_id");

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_pkey";
alter table "sequent_backend"."cast_vote"
    add constraint "cast_vote_pkey"
    primary key ("id");

alter table "sequent_backend"."candidate" drop constraint "candidate_pkey";
alter table "sequent_backend"."candidate"
    add constraint "candidate_pkey"
    primary key ("id");

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_pkey";
alter table "sequent_backend"."ballot_style"
    add constraint "ballot_style_pkey"
    primary key ("id");

alter table "sequent_backend"."election_result" drop constraint "election_result_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_pkey";
alter table "sequent_backend"."election"
    add constraint "election_pkey"
    primary key ("id");

alter table "sequent_backend"."contest"
  add constraint "contest_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_contest_id_election_event_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_pkey";
alter table "sequent_backend"."contest"
    add constraint "contest_pkey"
    primary key ("id");

alter table "sequent_backend"."candidate"
  add constraint "candidate_contest_id_fkey"
  foreign key ("contest_id")
  references "sequent_backend"."contest"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_contest_id_fkey"
  foreign key ("contest_id")
  references "sequent_backend"."contest"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area" drop constraint "area_pkey";
alter table "sequent_backend"."area"
    add constraint "area_pkey"
    primary key ("id");

alter table "sequent_backend"."election_result"
  add constraint "election_result_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

ALTER TABLE "sequent_backend"."election_result" ALTER COLUMN "result_eml" TYPE bytea;

ALTER TABLE "sequent_backend"."cast_vote" ALTER COLUMN "cast_ballot_eml" TYPE bytea;

ALTER TABLE "sequent_backend"."ballot_style" ALTER COLUMN "ballot_eml" TYPE bytea;

ALTER TABLE "sequent_backend"."election" ALTER COLUMN "eml" TYPE bytea;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "conditions" jsonb
--  null;

alter table "sequent_backend"."contest" drop constraint "contest_election_event_id_fkey";

alter table "sequent_backend"."candidate" drop constraint "candidate_election_event_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "election_event_id" uuid
--  null;

alter table "sequent_backend"."election_result" drop constraint "election_result_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "voter_id_string" varchar
--  null;

alter table "sequent_backend"."election_result" drop constraint "election_result_area_id_fkey";

alter table "sequent_backend"."election_result" drop constraint "election_result_election_id_fkey";

alter table "sequent_backend"."election_result" drop constraint "election_result_tenant_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_area_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_tenant_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_area_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_tenant_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_contest_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_area_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_fkey";

alter table "sequent_backend"."area" drop constraint "area_election_event_id_fkey";

alter table "sequent_backend"."area" drop constraint "area_tenant_id_fkey";

alter table "sequent_backend"."candidate" drop constraint "candidate_contest_id_fkey";

alter table "sequent_backend"."candidate" drop constraint "candidate_tenant_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_election_event_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_tenant_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "statistics" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "result_eml_signature" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "result_eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "cast_ballot_signature" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "cast_ballot_eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "status" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "ballot_signature" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "ballot_eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "created_at" timestamptz
--  null default now();

alter table "sequent_backend"."area_contest" alter column "created_at" drop not null;
alter table "sequent_backend"."area_contest" add column "created_at" jsonb;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "created_at" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "contest_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "tenant_id" uuid
--  null;

alter table "sequent_backend"."area_contest" rename to "area_context";

DROP TABLE "sequent_backend"."area_context";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "type" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "name" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "is_public" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "type" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "name" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "contest_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "tally_configuration" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "is_encrypted" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "counting_algorithm" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "voting_type" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "max_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "min_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "name" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "is_active" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "is_acclaimed" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "spoil_ballot_option" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "is_consolidated_ballot_encoding" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "num_allowed_revotes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "status" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "dates" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "name" varchar
--  not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "created_at" timestamptz
--  null default now();

alter table "sequent_backend"."election" alter column "created_at" set default now();
alter table "sequent_backend"."election" alter column "created_at" drop not null;
alter table "sequent_backend"."election" add column "created_at" timestamp;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "created_at" Timestamp
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "tenant_id" uuid
--  null;

alter table "sequent_backend"."election_event" drop constraint "election_event_audit_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "audit_election_event_id" uuid
--  null;

alter table "sequent_backend"."election_event" alter column "audit_election_event_id" drop not null;
alter table "sequent_backend"."election_event" add column "audit_election_event_id" int4;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "audit_election_event_id" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "is_audit" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "encryption_protocol" varchar
--  not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "user_boards" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "status" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "dates" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "voting_channels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "is_archived" boolean
--  not null default 'false';

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "bulletin_board_reference" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "name" varchar
--  not null;

DROP TABLE "sequent_backend"."election_result";

DROP TABLE "sequent_backend"."cast_vote";

DROP TABLE "sequent_backend"."ballot_style";

ALTER TABLE "sequent_backend"."area" ALTER COLUMN "id" drop default;

DROP TABLE "sequent_backend"."area";

ALTER TABLE "sequent_backend"."candidate" ALTER COLUMN "id" drop default;

DROP TABLE "sequent_backend"."candidate";

ALTER TABLE "sequent_backend"."contest" ALTER COLUMN "id" drop default;

DROP TABLE "sequent_backend"."contest";

DROP TABLE "sequent_backend"."election";

alter table "sequent_backend"."election_event" alter column "is_active" set default false;
alter table "sequent_backend"."election_event" alter column "is_active" drop not null;
alter table "sequent_backend"."election_event" add column "is_active" bool;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tenant" add column "is_active" boolean
--  not null default 'false';

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "is_active" boolean
--  not null default 'false';

alter table "sequent_backend"."election_event" rename to "event";


DROP INDEX IF EXISTS "sequent_backend"."tenant_labels";

DROP INDEX IF EXISTS "sequent_backend"."event_labels";

DROP TABLE "sequent_backend"."event";

DROP TABLE "sequent_backend"."tenant";

drop schema "sequent_backend" cascade;
