-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Never implicitly reinterpret already activated approvals via rollback.
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_product_activation WHERE active) THEN
        RAISE EXCEPTION 'Cannot remove activated product signing policy';
    END IF;
END;
$$;
DROP FUNCTION sequent_backend.operational_signing_event_mode_facts(uuid, uuid);
DROP TABLE sequent_backend.operational_signing_event_mode;
DROP TABLE sequent_backend.operational_signing_product_activation;
DROP FUNCTION sequent_backend.operational_signing_guard_product_activation();
REVOKE USAGE ON SCHEMA sequent_backend FROM sequent_os_mode_reader;
-- Leave the cluster-wide NOLOGIN role itself in place for other databases.
