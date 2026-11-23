-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. Rollback is allowed only before any document is
-- classified: dropping the registry would release withheld MIRU output.
LOCK TABLE sequent_backend.operational_miru_inventory,
    sequent_backend.operational_miru_artifact,
    sequent_backend.operational_miru_revision_artifact IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_miru_inventory)
       OR EXISTS (SELECT FROM sequent_backend.operational_miru_artifact)
       OR EXISTS (SELECT FROM sequent_backend.operational_miru_revision_artifact) THEN
        RAISE EXCEPTION 'operational MIRU registry is not empty; refuse rollback';
    END IF;
END;
$$;
DROP FUNCTION sequent_backend.operational_miru_export_exclusions(uuid, uuid);
DROP FUNCTION sequent_backend.operational_miru_generic_withhold(uuid, uuid, uuid);
DROP FUNCTION sequent_backend.operational_miru_register_artifact(uuid, uuid, uuid, uuid, uuid, uuid, uuid, text, text, uuid);
DROP TABLE sequent_backend.operational_miru_revision_artifact;
DROP TABLE sequent_backend.operational_miru_artifact;
DROP TABLE sequent_backend.operational_miru_inventory;
DROP FUNCTION sequent_backend.operational_miru_guard();
DROP TYPE sequent_backend.operational_miru_inventory_state;
