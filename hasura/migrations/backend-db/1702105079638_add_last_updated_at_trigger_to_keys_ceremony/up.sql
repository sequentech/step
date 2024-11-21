CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_updated_at()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."updated_at" = NOW();
  RETURN _new;
END;
$function$;

CREATE OR REPLACE FUNCTION sequent_backend.set_current_timestamp_last_updated_at()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
  _new record;
BEGIN
  _new := NEW;
  _new."last_updated_at" = NOW();
  RETURN _new;
END;
$function$;

CREATE TRIGGER "set_sequent_backend_keys_ceremony_last_updated_at"
BEFORE UPDATE ON "sequent_backend"."keys_ceremony"
FOR EACH ROW
EXECUTE PROCEDURE "sequent_backend"."set_current_timestamp_last_updated_at"();
