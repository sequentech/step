-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));

-- Output already classified through this scope stays withheld by the
-- standard registry; refuse rather than leave it without its admission.
LOCK TABLE sequent_backend.operational_miru_artifact IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
 IF EXISTS (SELECT FROM sequent_backend.operational_miru_artifact a
            WHERE NOT sequent_backend.operational_signing_hsm_required(a.tenant_id))
 THEN RAISE EXCEPTION 'direct HSM MIRU artifacts exist: refuse rollback'; END IF;
END;
$$;
REVOKE EXECUTE ON FUNCTION felix3_initialize_direct.miru_event_scope(uuid,uuid,uuid)
 FROM sequent_os_ledger_api_owner;
REVOKE USAGE ON SCHEMA felix3_initialize_direct FROM sequent_os_ledger_api_owner;
DROP FUNCTION felix3_initialize_direct.miru_event_scope(uuid,uuid,uuid);
