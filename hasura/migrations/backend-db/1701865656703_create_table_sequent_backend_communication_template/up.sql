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
