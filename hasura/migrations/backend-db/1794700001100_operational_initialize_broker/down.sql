-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. Rollback is allowed only before any challenge or
-- device grant exists: the rows fence single-use callbacks and record the
-- provenance of device receipts.
LOCK TABLE sequent_backend.operational_initialize_attempt,
    sequent_backend.operational_signing_device_grant IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_initialize_attempt)
       OR EXISTS (SELECT FROM sequent_backend.operational_signing_device_grant) THEN
        RAISE EXCEPTION 'operational Initialize ledger is not empty; refuse rollback';
    END IF;
END;
$$;
DROP FUNCTION sequent_backend.operational_initialize_get_browser_return(uuid, uuid, uuid, text, uuid);
DROP FUNCTION sequent_backend.operational_initialize_record_browser_return(uuid, uuid, uuid, text, uuid, text, text, uuid);
DROP FUNCTION sequent_backend.operational_initialize_discard_unredeemed(uuid, uuid, uuid, text, text, uuid);
DROP FUNCTION sequent_backend.operational_initialize_current_state(uuid, uuid, uuid, text, text);
DROP FUNCTION sequent_backend.operational_initialize_reserve_own_callback(uuid, uuid, uuid, text, uuid, text, text, text, bigint, timestamptz, text);
DROP FUNCTION sequent_backend.operational_initialize_take_callback(uuid, uuid, uuid, text, uuid, text);
DROP FUNCTION sequent_backend.operational_initialize_load(uuid, uuid, uuid, text, text, uuid);
DROP FUNCTION sequent_backend.operational_initialize_prepare(uuid, uuid, uuid, text, text, text, uuid, text, text, text);
DROP FUNCTION sequent_backend.operational_initialize_context(uuid, uuid, uuid, text, text, text);
DROP FUNCTION sequent_backend.operational_initialize_current(uuid, uuid, uuid, text, text, text);
DROP TABLE sequent_backend.operational_signing_device_grant;
DROP TABLE sequent_backend.operational_initialize_attempt;
DROP FUNCTION sequent_backend.operational_signing_device_grant_guard();
DROP FUNCTION sequent_backend.operational_initialize_attempt_guard();
DROP TYPE sequent_backend.operational_initialize_return_panel;
DROP TYPE sequent_backend.operational_initialize_return_path;
DROP TYPE sequent_backend.operational_signing_flow;
DROP TYPE sequent_backend.operational_initialize_step;
