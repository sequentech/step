CREATE TABLE "sequent_backend"."support_material" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "created_at" timestamptz NOT NULL DEFAULT now(), "last_updated_at" timestamptz NOT NULL DEFAULT now(), "kind" text NOT NULL, "data" jsonb NOT NULL, "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "labels" jsonb NOT NULL, "annotations" jsonb NOT NULL, PRIMARY KEY ("id") );
CREATE OR REPLACE FUNCTION "sequent_backend"."set_current_timestamp_last_updated_at"()
RETURNS TRIGGER AS $$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."last_updated_at" = NOW();
  RETURN _new;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER "set_sequent_backend_support_material_last_updated_at"
BEFORE UPDATE ON "sequent_backend"."support_material"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_last_updated_at"();
COMMENT ON TRIGGER "set_sequent_backend_support_material_last_updated_at" ON "sequent_backend"."support_material"
IS 'trigger to set value of column "last_updated_at" to current timestamp on row update';
CREATE EXTENSION IF NOT EXISTS pgcrypto;
