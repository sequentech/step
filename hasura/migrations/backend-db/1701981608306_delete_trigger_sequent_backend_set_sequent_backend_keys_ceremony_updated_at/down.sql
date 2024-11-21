CREATE TRIGGER "set_sequent_backend_keys_ceremony_updated_at"
BEFORE UPDATE ON "sequent_backend"."keys_ceremony"
FOR EACH ROW EXECUTE FUNCTION sequent_backend.set_current_timestamp_updated_at();COMMENT ON TRIGGER "set_sequent_backend_keys_ceremony_updated_at" ON "sequent_backend"."keys_ceremony"
IS E'trigger to set value of column "updated_at" to current timestamp on row update';
