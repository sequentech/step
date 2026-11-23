-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. Rollback is allowed only before any installation or
-- generation attempt exists: a claimed attempt may have created a hardware
-- key, and an installation row decides the provenance of future enrollments.
LOCK TABLE sequent_backend.operational_setup_installation,
    sequent_backend.operational_setup_generation IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_setup_installation)
       OR EXISTS (SELECT FROM sequent_backend.operational_setup_generation) THEN
        RAISE EXCEPTION 'operational setup ledger is not empty; refuse rollback';
    END IF;
END;
$$;
DROP FUNCTION sequent_backend.operational_official_setup_status(uuid, uuid, text);
DROP FUNCTION sequent_backend.operational_setup_attach_certificate(uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text[]);
DROP FUNCTION sequent_backend.operational_setup_pin_for_attachment(uuid, uuid, uuid, uuid, uuid);
DROP FUNCTION sequent_backend.operational_setup_public_csr(uuid, uuid, uuid, uuid, uuid);
DROP FUNCTION sequent_backend.operational_setup_complete_generation(uuid, uuid, uuid, uuid, uuid, text, text, text);
DROP FUNCTION sequent_backend.operational_setup_claim_generation(uuid, uuid, uuid, uuid);
DROP FUNCTION sequent_backend.operational_setup_prepare_generation(uuid, uuid, uuid, uuid, uuid);
DROP FUNCTION sequent_backend.operational_setup_management_status(uuid, uuid, uuid, uuid);
DROP TABLE sequent_backend.operational_setup_generation;
DROP TABLE sequent_backend.operational_setup_installation;
DROP FUNCTION sequent_backend.operational_setup_guard();
DROP TYPE sequent_backend.operational_setup_step;
