-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. Rolling back after enrollment would destroy pins,
-- blocks and historical lifecycle evidence; do not make that an API action.
LOCK TABLE sequent_backend.operational_signing_official_enrollment,
    sequent_backend.operational_signing_official_certificate IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_official_enrollment) THEN
        RAISE EXCEPTION 'operational signing enrollments exist; refuse rollback';
    END IF;
END;
$$;
DROP FUNCTION sequent_backend.operational_signing_get_official_chain(uuid, uuid, text, text, integer, text, text);
DROP FUNCTION sequent_backend.operational_signing_retire_official_certificate(uuid, uuid, text, uuid, uuid, text);
DROP FUNCTION sequent_backend.operational_signing_revoke_official(uuid, text, uuid, text, integer);
DROP FUNCTION sequent_backend.operational_signing_bind_official_certificate(uuid, uuid, text, uuid, uuid, text, text[]);
DROP FUNCTION sequent_backend.operational_signing_enroll_official(uuid, text, text, integer, text, text, boolean, text);
DROP TABLE sequent_backend.operational_signing_official_certificate;
DROP TABLE sequent_backend.operational_signing_official_enrollment;
