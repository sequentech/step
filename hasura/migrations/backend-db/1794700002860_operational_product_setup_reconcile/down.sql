-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DO $$
BEGIN
    IF EXISTS(SELECT FROM sequent_backend.operational_setup_unclaimed_resolution) THEN
        RAISE EXCEPTION 'unclaimed setup resolution contains audit records; preserve them before rollback';
    END IF;
END;
$$;
DROP FUNCTION sequent_backend.operational_reconcile_unclaimed_setup(uuid,uuid,uuid,uuid,timestamptz);
REVOKE DELETE ON TABLE sequent_backend.operational_setup_generation FROM sequent_os_ledger_api_owner;
DROP TABLE sequent_backend.operational_setup_unclaimed_resolution;
