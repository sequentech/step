CREATE TABLE "sequent_backend"."secret" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid, "labels" jsonb, "annotations" jsonb, "key" text NOT NULL, "value" bytea NOT NULL, PRIMARY KEY ("id","tenant_id","key") , UNIQUE ("key"));
CREATE EXTENSION IF NOT EXISTS pgcrypto;
