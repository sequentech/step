CREATE TABLE "sequent_backend"."notification" ("id" uuid NOT NULL DEFAULT gen_random_uuid(), "tenant_id" uuid NOT NULL, "election_event_id" uuid NOT NULL, "created_at" timestamptz NOT NULL DEFAULT now(), "updated_at" timestamptz NOT NULL DEFAULT now(), "labels" jsonb, "annotations" jsonb, "name" varchar, "election_id" uuid, "type" varchar, "template_id" uuid, "alias" varchar, PRIMARY KEY ("id") );
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
CREATE TRIGGER "set_sequent_backend_notification_updated_at"
BEFORE UPDATE ON "sequent_backend"."notification"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_updated_at"();
COMMENT ON TRIGGER "set_sequent_backend_notification_updated_at" ON "sequent_backend"."notification"
IS 'trigger to set value of column "updated_at" to current timestamp on row update';
CREATE EXTENSION IF NOT EXISTS pgcrypto;
