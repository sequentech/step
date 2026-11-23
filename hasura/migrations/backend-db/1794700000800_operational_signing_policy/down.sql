-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. Never roll back an effective HSM-required policy.
LOCK TABLE sequent_backend.operational_signing_policy,
    sequent_backend.operational_signing_required_tenant IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM sequent_backend.operational_signing_policy WHERE singleton AND mode = 'legacy-p12')
       OR EXISTS (SELECT FROM sequent_backend.operational_signing_required_tenant) THEN
        RAISE EXCEPTION 'operational signing policy has been promoted; refuse rollback';
    END IF;
END;
$$;
DROP TABLE sequent_backend.operational_signing_required_tenant;
DROP TABLE sequent_backend.operational_signing_policy;
DROP FUNCTION sequent_backend.protect_operational_signing_policy();
