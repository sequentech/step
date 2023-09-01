CREATE TABLE "sequent_backend"."trustee" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "public_key" text, "name" varchar, "created_at" timestamptz DEFAULT now(), "last_updated_at" timestamptz DEFAULT now(), "labels" jsonb, "annotations" jsonb, PRIMARY KEY ("id") );
CREATE EXTENSION IF NOT EXISTS pgcrypto;
