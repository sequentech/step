-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Receipt ownership lives in the private event store. Its document metadata
-- cannot be renamed or reclassified into an ordinary administrative download.
CREATE FUNCTION sequent_backend.protect_receipt_document() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (OLD.annotations->>'purpose' = 'ballot-receipt'
        OR lower(coalesce(OLD.name, '')) LIKE 'ballot\_receipt\_%')
       AND (NEW.annotations->>'purpose' IS DISTINCT FROM OLD.annotations->>'purpose'
            OR NEW.name IS DISTINCT FROM OLD.name
            OR NEW.is_public IS DISTINCT FROM OLD.is_public) THEN
        RAISE EXCEPTION 'Receipt document classification is immutable';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER protect_receipt_document
BEFORE UPDATE ON sequent_backend.document FOR EACH ROW
EXECUTE FUNCTION sequent_backend.protect_receipt_document();
