-- SPDX-License-Identifier: AGPL-3.0-only
-- Apply transactionally. Preserve the authenticated return of every attempt.
LOCK TABLE felix3_initialize_setup.attempt IN ACCESS EXCLUSIVE MODE NOWAIT;
DO $$
BEGIN
    IF EXISTS (SELECT FROM felix3_initialize_setup.attempt WHERE portal_session_sha256 IS NOT NULL) THEN
        RAISE EXCEPTION 'local setup browser records require operator reconciliation';
    END IF;
END;
$$;
DROP FUNCTION felix3_initialize_setup.take_delivery(felix3_initialize_setup.binding,uuid,text,integer),
    felix3_initialize_setup.browser_attempt(felix3_initialize_setup.binding,text),
    felix3_initialize_setup.bind_browser(felix3_initialize_setup.binding,uuid,text,text,text);
ALTER TABLE felix3_initialize_setup.attempt DROP CONSTRAINT local_setup_browser_binding,
    DROP COLUMN portal_session_sha256, DROP COLUMN return_path, DROP COLUMN return_panel, DROP COLUMN delivery_revision;
