-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- A tenant's settings.configuration_signing decides which configuration
-- packages it trusts. Only the validated server route may write it: that
-- route sets sequent.configuration_signing_tenant to the tenant's id for its
-- own transaction (set_config(..., true)), so the permission ends with the
-- transaction and covers that tenant alone. Every other write that would
-- add, change, null or remove the key is refused, whoever makes it: a
-- GraphQL update or insert (including _append and _delete_key), a restored
-- backup, or deleting and recreating the tenant row. Writes that leave the
-- key as it is, such as other settings, pass unchanged.

CREATE OR REPLACE FUNCTION "sequent_backend"."configuration_signing_change_allowed"(tenant uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT coalesce(current_setting('sequent.configuration_signing_tenant', true), '')
        = tenant::text;
$$;

CREATE OR REPLACE FUNCTION "sequent_backend"."guard_configuration_signing"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    before_value jsonb;
    after_value jsonb;
    tenant uuid;
BEGIN
    IF TG_OP = 'INSERT' THEN
        before_value := NULL;
        after_value := NEW.settings -> 'configuration_signing';
        tenant := NEW.id;
    ELSIF TG_OP = 'UPDATE' THEN
        before_value := OLD.settings -> 'configuration_signing';
        after_value := NEW.settings -> 'configuration_signing';
        tenant := OLD.id;
        -- Moving a row to another id would carry its trust with it.
        IF NEW.id IS DISTINCT FROM OLD.id AND before_value IS NOT NULL
            AND NOT "sequent_backend"."configuration_signing_change_allowed"(OLD.id) THEN
            RAISE EXCEPTION 'the tenant''s configuration_signing is set only through its own setting'
                USING ERRCODE = 'insufficient_privilege';
        END IF;
    ELSE
        before_value := OLD.settings -> 'configuration_signing';
        after_value := NULL;
        tenant := OLD.id;
    END IF;

    IF before_value IS DISTINCT FROM after_value
        AND NOT "sequent_backend"."configuration_signing_change_allowed"(tenant) THEN
        RAISE EXCEPTION 'the tenant''s configuration_signing is set only through its own setting'
            USING ERRCODE = 'insufficient_privilege';
    END IF;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "guard_configuration_signing" ON "sequent_backend"."tenant";
CREATE TRIGGER "guard_configuration_signing"
BEFORE INSERT OR UPDATE OR DELETE ON "sequent_backend"."tenant"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_configuration_signing"();
