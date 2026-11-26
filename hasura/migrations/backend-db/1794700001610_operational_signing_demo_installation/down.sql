-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Reconciliation of cast sealing is forward-only; it belongs to canonical
-- 1600. Never drop its tables, keys, rules, role, grants or functions here.
-- Undo ONLY inert demo DDL. Serialize with registration before inspecting
-- data, and refuse an active installation even when it predates this version.
LOCK TABLE sequent_backend.tenant, sequent_backend.operational_signing_demo_install,
    sequent_backend.operational_signing_demo_scope IN ACCESS EXCLUSIVE MODE;
DO $$ BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_demo_install)
       OR EXISTS (SELECT FROM sequent_backend.operational_signing_demo_scope) THEN
        RAISE EXCEPTION 'operational demo is installed; refuse rollback';
    END IF;
END $$;
DROP TRIGGER operational_demo_new_tenant ON sequent_backend.tenant;
DROP FUNCTION sequent_backend.operational_demo_new_tenant();
DROP FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text);
DROP FUNCTION sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text);
DROP TABLE sequent_backend.operational_signing_demo_scope, sequent_backend.operational_signing_demo_install;
DROP FUNCTION sequent_backend.operational_demo_immutable();
