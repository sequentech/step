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
