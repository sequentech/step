-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_initialize_self_setup)
       OR EXISTS (SELECT FROM sequent_backend.operational_initialize_setup_roster) THEN
        RAISE EXCEPTION 'cannot discard operational setup history or provisioning';
    END IF;
END $$;
DROP FUNCTION sequent_backend.operational_initialize_start_self_setup(uuid,uuid,uuid,uuid,uuid,text,timestamptz,uuid);
DROP FUNCTION sequent_backend.operational_initialize_claim_self_setup(uuid,uuid,uuid,uuid);
DROP FUNCTION sequent_backend.operational_initialize_self_setup_status(uuid,uuid,uuid);
DROP TABLE sequent_backend.operational_initialize_self_setup;
DROP TABLE sequent_backend.operational_initialize_setup_roster;
