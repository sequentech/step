
ALTER TABLE "sequent_backend"."document" ALTER COLUMN "size" TYPE integer;

DROP TRIGGER check_revote_trigger ON cast_vote;

DROP INDEX IF EXISTS "sequent_backend"."idx_applications_tenant_election_created";

alter table "sequent_backend"."secret" drop constraint "secret_tenant_id_fkey";

alter table "sequent_backend"."secret" drop constraint "secret_election_event_id_fkey";

ALTER TABLE "sequent_backend"."secret" DROP COLUMN "created_at";
DROP TABLE "sequent_backend"."secret";

ALTER TABLE "sequent_backend"."tally_session" ALTER COLUMN "permission_label" TYPE ARRAY;

ALTER TABLE "sequent_backend"."tally_session" ALTER COLUMN "permission_label" TYPE ARRAY;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."keys_ceremony" add column "permission_label" text[]
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "permission_label" text[]
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_election_area" add column "name" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."report" add column "permission_label" text[]
--  null;

DROP TABLE "sequent_backend"."results_election_area";

alter table "sequent_backend"."tenant" alter column "voting_channels" set default '{"kiosk": true, "online": true}'::jsonb;

alter table "sequent_backend"."tally_session_contest" alter column "contest_id" set not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."template" add column "alias" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."report" add column "template_alias" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."applications" add column "permission_label" text
--  null;

ALTER TABLE "sequent_backend"."report" ALTER COLUMN "encryption_policy" drop default;

alter table "sequent_backend"."tasks_execution" alter column "election_event_id" set not null;

alter table "sequent_backend"."tasks_execution" drop constraint "tasks_execution_pkey";
alter table "sequent_backend"."tasks_execution"
    add constraint "tasks_execution_pkey"
    primary key ("election_event_id", "tenant_id", "id");

alter table "sequent_backend"."applications" alter column "area_id" set not null;

alter table "sequent_backend"."applications" drop constraint "applications_pkey";
alter table "sequent_backend"."applications"
    add constraint "applications_pkey"
    primary key ("id", "tenant_id", "area_id", "election_event_id");

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."applications" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."applications" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "tally_type" text
--  null;

alter table "sequent_backend"."applications" drop constraint "applications_pkey";
alter table "sequent_backend"."applications"
    add constraint "applications_pkey"
    primary key ("id", "election_event_id", "tenant_id");

alter table "sequent_backend"."applications" drop constraint "applications_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."applications" add column "area_id" uuid
--  not null;

alter table "sequent_backend"."applications" alter column "applicant_id" drop not null;

alter table "sequent_backend"."applications" drop constraint "applications_pkey";
alter table "sequent_backend"."applications"
    add constraint "applications_pkey"
    primary key ("id", "election_event_id", "tenant_id");

alter table "sequent_backend"."applications" drop constraint "applications_pkey";
alter table "sequent_backend"."applications"
    add constraint "applications_pkey"
    primary key ("id");

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."applications" add column "election_event_id" uuid
--  not null;

alter table "sequent_backend"."applications" alter column "applicant_id" set not null;

DROP TABLE "sequent_backend"."applications";

alter table "sequent_backend"."report" alter column "template_id" drop not null;
alter table "sequent_backend"."report" add column "template_id" text;

alter table "sequent_backend"."report" alter column "encryption_policy" drop not null;

alter table "sequent_backend"."report" alter column "encryption_policy" set default 'unencrypted'::text;

alter table "sequent_backend"."report" alter column "encryption_policy" set not null;
alter table "sequent_backend"."report" alter column "encryption_policy" set default 'unencrypted'::character varying;

ALTER TABLE "sequent_backend"."report" ALTER COLUMN "encryption_policy" TYPE character varying;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."report" add column "encryption_policy" character varying
--  not null default 'unencrypted';

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "initialization_report_generated" boolean
--  null default 'false';

alter table "sequent_backend"."keys_ceremony" rename column "settings" to "presentation";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."report" add column "created_at" timestamptz
--  null default now();

alter table "sequent_backend"."report" alter column "created_at" set default now();
alter table "sequent_backend"."report" alter column "created_at" drop not null;
alter table "sequent_backend"."report" add column "created_at" timetz;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "keys_ceremony_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."keys_ceremony" add column "is_default" Boolean
--  null default 'true';

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."keys_ceremony" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."keys_ceremony" add column "name" text
--  null;

ALTER TABLE "sequent_backend"."report" ALTER COLUMN "created_at" TYPE time without time zone;

alter table "sequent_backend"."report" alter column "created_at" set not null;

alter table "sequent_backend"."report" alter column "template_id" set not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."report" add column "created_at" time
--  not null default now();

alter table "sequent_backend"."report" rename column "template_id" to "template_alias";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."report" add column "cron_config" jsonb
--  null;

DROP TABLE "sequent_backend"."report";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."scheduled_event" add column "archived_at" timestamptz
--  null;

alter table "sequent_backend"."election_event" alter column "dates" drop not null;
alter table "sequent_backend"."election_event" add column "dates" jsonb;

alter table "sequent_backend"."election" alter column "dates" drop not null;
alter table "sequent_backend"."election" add column "dates" jsonb;

alter table "sequent_backend"."election" delete column "permission_label" text
 null;

alter table "sequent_backend"."template" rename column "type" to "communication_type";

alter table "sequent_backend"."template" rename to "communication_template";

DROP TABLE "sequent_backend"."notification";

alter table "sequent_backend"."tasks_execution"
  add constraint "tasks_execution_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."tasks_execution" rename column "executed_by_user" to "executed_by_user_id";

ALTER TABLE "sequent_backend"."tasks_execution" ALTER COLUMN "executed_by_user_id" TYPE uuid;

alter table "sequent_backend"."tasks_execution" alter column "start_at" drop not null;

alter table "sequent_backend"."tasks_execution" alter column "end_at" set default now();

alter table "sequent_backend"."tasks_execution" rename column "execution_status" to "status";

ALTER TABLE "sequent_backend"."tasks_execution" ALTER COLUMN "end_at" TYPE timestamp without time zone;

ALTER TABLE "sequent_backend"."tasks_execution" ALTER COLUMN "start_at" TYPE timestamp without time zone;

ALTER TABLE "sequent_backend"."tasks_execution" ALTER COLUMN "created_at" TYPE timestamp without time zone;

ALTER TABLE "sequent_backend"."tasks_execution" ALTER COLUMN "annotations" TYPE json;

ALTER TABLE "sequent_backend"."tasks_execution" ALTER COLUMN "labels" TYPE json;

DROP TABLE "sequent_backend"."tasks_execution";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "total_auditable_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "total_auditable_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "total_auditable_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "total_auditable_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_session" add column "configuration" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tenant" add column "test" integer
--  null default '0';

alter table "sequent_backend"."area" drop constraint "area_election_event_id_tenant_id_parent_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "parent_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."scheduled_event" add column "cron_config" jsonb
--  null;

alter table "sequent_backend"."scheduled_event" alter column "cron_config" drop not null;
alter table "sequent_backend"."scheduled_event" add column "cron_config" varchar;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."support_material" add column "is_hidden" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "alias" text
--  null;

alter table "sequent_backend"."candidate" alter column "order" drop not null;
alter table "sequent_backend"."candidate" add column "order" int4;

alter table "sequent_backend"."contest" alter column "order_answers" drop not null;
alter table "sequent_backend"."contest" add column "order_answers" text;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "order" integer
--  null;

alter table "sequent_backend"."tally_session_execution" alter column "document_id" drop not null;
alter table "sequent_backend"."tally_session_execution" add column "document_id" uuid;

alter table "sequent_backend"."tally_session_execution"
  add constraint "tally_session_execution_document_id_fkey"
  foreign key ("document_id")
  references "sequent_backend"."document"
  ("id") on update restrict on delete restrict;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_event" add column "documents" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "receipts" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_election" add column "documents" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest_candidate" add column "documents" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "documents" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest_candidate" add column "documents" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "documents" jsonb
--  null;

alter table "sequent_backend"."results_election" rename column "total_voters_percent" to "total_valid_votes_percent";

alter table "sequent_backend"."results_election" rename column "total_voters" to "total_valid_votes";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "total_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "total_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "total_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "total_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "implicit_invalid_votes_percent" numeric
--  null;

alter table "sequent_backend"."results_election" alter column "blank_votes" drop not null;
alter table "sequent_backend"."results_election" add column "blank_votes" int4;

alter table "sequent_backend"."results_election" alter column "implicit_invalid_votes" drop not null;
alter table "sequent_backend"."results_election" add column "implicit_invalid_votes" int4;

alter table "sequent_backend"."results_election" alter column "explicit_invalid_votes" drop not null;
alter table "sequent_backend"."results_election" add column "explicit_invalid_votes" int4;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_election" add column "total_valid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest_candidate" add column "cast_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "blank_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "implicit_invalid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "explicit_invalid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "total_valid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "total_invalid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_contest" add column "total_invalid_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "blank_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "explicit_invalid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "total_invalid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "total_invalid_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest" add column "total_valid_votes_percent" numeric
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."results_area_contest_candidate" add column "cast_votes_percent" numeric
--  null;

alter table "sequent_backend"."tally_sheet" alter column "created_by_user_id" drop not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_sheet" add column "created_by_user_id" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tally_sheet" add column "deleted_at" timestamptz
--  null;

DROP TABLE "sequent_backend"."tally_sheet";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."support_material" add column "document_id" text
--  null;

alter table "sequent_backend"."support_material" drop constraint "support_material_tenant_id_fkey";

alter table "sequent_backend"."support_material"
  add constraint "support_material_election_event_id_fkey2"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."support_material" drop constraint "support_material_election_event_id_fkey2",
  add constraint "support_material_tenant_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."support_material" drop constraint "support_material_tenant_id_fkey";

alter table "sequent_backend"."support_material" drop constraint "support_material_election_event_id_fkey",
  add constraint "support_material_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."support_material" drop constraint "support_material_tenant_id_fkey";

alter table "sequent_backend"."support_material" drop constraint "support_material_pkey";
alter table "sequent_backend"."support_material"
    add constraint "support_material_pkey"
    primary key ("id");

DROP TABLE "sequent_backend"."support_material";

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

DELETE FROM "sequent_backend"."area_contest" WHERE "id" = '44a37949-1481-46bc-91fb-377fd089391c';

DELETE FROM "sequent_backend"."candidate" WHERE "id" = '1822089d-ae17-4a03-8935-25164b3f2142' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5' AND "election_event_id" = '33f18502-a67c-4853-8333-a58630663559';

DELETE FROM "sequent_backend"."candidate" WHERE "id" = 'd9249345-11be-4652-ad04-298d70931610' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5' AND "election_event_id" = '33f18502-a67c-4853-8333-a58630663559';

DELETE FROM "sequent_backend"."candidate" WHERE "id" = 'a24303de-5798-47cd-9b3e-4f391d1bae7b' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5' AND "election_event_id" = '33f18502-a67c-4853-8333-a58630663559';

DELETE FROM "sequent_backend"."contest" WHERE "election_event_id" = '33f18502-a67c-4853-8333-a58630663559' AND "id" = '69f2f987-460c-48ac-ac7a-4d44d99b37e6' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

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

DELETE FROM "sequent_backend"."ballot_style" WHERE "election_event_id" = '33f18502-a67c-4853-8333-a58630663559' AND "id" = '6e5bbff4-0fb8-4971-a808-37de49573f6a' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

DELETE FROM "sequent_backend"."election" WHERE "id" = 'f2f1065e-b784-46d1-b81a-c71bfeb9ad55' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5' AND "election_event_id" = '33f18502-a67c-4853-8333-a58630663559';

DELETE FROM "sequent_backend"."area" WHERE "id" = '2f312a36-f39c-46e4-9670-1d1ce4625745' AND "election_event_id" = '33f18502-a67c-4853-8333-a58630663559' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

DELETE FROM "sequent_backend"."election_event" WHERE "id" = '33f18502-a67c-4853-8333-a58630663559';

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
