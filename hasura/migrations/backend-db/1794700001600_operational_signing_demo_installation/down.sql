-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DO $$ BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_demo_install) THEN
        RAISE EXCEPTION 'operational demo is installed; refuse rollback';
    END IF;
END $$;
DROP TRIGGER operational_demo_new_tenant ON sequent_backend.tenant;
DROP FUNCTION sequent_backend.operational_demo_new_tenant();
DROP FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text);
DROP FUNCTION sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text);
DROP TABLE sequent_backend.operational_signing_demo_scope, sequent_backend.operational_signing_demo_install;
DROP FUNCTION sequent_backend.operational_demo_immutable();
