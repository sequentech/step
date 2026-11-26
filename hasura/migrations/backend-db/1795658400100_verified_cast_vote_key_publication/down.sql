-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.verified_cast_key_job) THEN
        RAISE EXCEPTION 'cannot downgrade verified cast key publication after authorizing an election event';
    END IF;
END;
$$;
DROP TRIGGER verified_cast_key_registered_import_guard ON sequent_backend.configuration_package;
DROP FUNCTION sequent_backend.verified_cast_key_registered_import_guard();
DROP TRIGGER verified_cast_key_open_guard ON sequent_backend.election;
DROP TRIGGER verified_cast_key_open_guard ON sequent_backend.election_event;
DROP FUNCTION sequent_backend.verified_cast_key_open_guard();
DROP FUNCTION sequent_backend.verified_cast_key_complete(uuid, uuid, text);
DROP FUNCTION sequent_backend.verified_cast_key_for_open(uuid, uuid);
DROP FUNCTION sequent_backend.verified_cast_key_stage(uuid, uuid, uuid, text, bytea);
DROP FUNCTION sequent_backend.verified_cast_key_next();
DROP FUNCTION sequent_backend.verified_cast_key_publisher_tenant();
DROP FUNCTION sequent_backend.verified_cast_key_authorize(uuid, uuid, bigint, text, bigint, text);
DROP TABLE sequent_backend.verified_cast_key_job;
DROP TABLE sequent_backend.verified_cast_key_site;
REVOKE SELECT (tenant_id, election_event_id, revision, manifest_sha256, xmin)
    ON sequent_backend.configuration_package FROM sequent_os_ledger_api_owner;
REVOKE SELECT (tenant_id, sha256, generation) ON sequent_backend.managed_webauthn_roster
    FROM sequent_os_ledger_api_owner;
REVOKE USAGE ON SCHEMA sequent_backend FROM
    sequent_os_verified_cast_key_importer, sequent_os_verified_cast_key_publisher;
-- Group roles may also be used by other databases in the same cluster.
