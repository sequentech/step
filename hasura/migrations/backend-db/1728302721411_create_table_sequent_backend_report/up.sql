CREATE TABLE "sequent_backend"."report" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "election_event_id" uuid NOT NULL, "tenant_id" uuid NOT NULL, "election_id" uuid, "report_type" text NOT NULL, "template_alias" text NOT NULL, PRIMARY KEY ("id") , UNIQUE ("id"));
CREATE EXTENSION IF NOT EXISTS pgcrypto;
