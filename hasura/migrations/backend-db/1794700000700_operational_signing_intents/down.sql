-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Rollback is allowed only before any challenge or device evidence exists.
-- Do not discard burned nonces, receipts, or an unknown device outcome.
LOCK TABLE sequent_backend.operational_signing_ceremony,
    sequent_backend.operational_signing_intent IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_ceremony)
       OR EXISTS (SELECT FROM sequent_backend.operational_signing_intent) THEN
        RAISE EXCEPTION 'operational signing ledger is not empty; refuse rollback';
    END IF;
END;
$$;
DROP FUNCTION sequent_backend.operational_signing_mark_applied_from_evidence(uuid, uuid, uuid, text, uuid, uuid, bytea, bytea);
DROP FUNCTION sequent_backend.operational_signing_mark_applied(sequent_backend.operational_signing_binding, uuid, bytea, bytea);
DROP FUNCTION sequent_backend.operational_signing_get_completed(uuid, uuid, uuid, text, uuid);
DROP FUNCTION sequent_backend.operational_signing_read(sequent_backend.operational_signing_binding);
DROP FUNCTION sequent_backend.operational_signing_quarantine(sequent_backend.operational_signing_binding, uuid);
DROP FUNCTION sequent_backend.operational_signing_settle(sequent_backend.operational_signing_binding, uuid, bytea);
DROP FUNCTION sequent_backend.operational_signing_claim_device(sequent_backend.operational_signing_binding, uuid);
DROP FUNCTION sequent_backend.operational_signing_reserve(sequent_backend.operational_signing_binding, bigint, timestamptz);
DROP FUNCTION sequent_backend.operational_signing_discard_prepared(sequent_backend.operational_signing_binding);
DROP FUNCTION sequent_backend.operational_signing_prepare(sequent_backend.operational_signing_binding);
DROP FUNCTION sequent_backend.operational_signing_validate(sequent_backend.operational_signing_binding);
DROP TABLE sequent_backend.operational_signing_intent;
DROP FUNCTION sequent_backend.operational_signing_immutable();
DROP TABLE sequent_backend.operational_signing_ceremony;
DROP TYPE sequent_backend.operational_signing_binding;
