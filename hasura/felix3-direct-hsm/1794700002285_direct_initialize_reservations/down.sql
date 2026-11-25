-- SPDX-License-Identifier: AGPL-3.0-only
DO $$
BEGIN
  IF NOT pg_try_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0)) THEN
     RAISE EXCEPTION 'cannot remove a busy direct signing policy';
  END IF;
  IF EXISTS (SELECT FROM felix3_initialize_direct.installation)
      OR EXISTS (SELECT FROM felix3_initialize_direct.reservation)
      OR EXISTS (SELECT FROM felix3_initialize_direct.per_actor_key)
  THEN RAISE EXCEPTION 'refusing removal of an installed direct HSM requirement or evidence'; END IF;
END;
$$;
DROP FUNCTION sequent_backend.record_local_initialize_direct_key(uuid,bytea);
DROP FUNCTION sequent_backend.claim_local_initialize_direct(uuid);
DROP FUNCTION sequent_backend.local_initialize_direct_ready(uuid,uuid,uuid,uuid,uuid,text);
DROP FUNCTION sequent_backend.operational_signing_local_initialize_scope(uuid,uuid);
DROP FUNCTION felix3_initialize_direct.consume(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea);
DROP FUNCTION felix3_initialize_direct.proof_key(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea);
DROP FUNCTION felix3_initialize_direct.device_principal();
DROP FUNCTION felix3_initialize_direct.reserve(uuid,uuid,uuid,uuid,uuid,uuid,uuid,bytea);
DROP TRIGGER direct_initialize_immutable ON felix3_initialize_direct.installation;
DROP TRIGGER direct_initialize_install ON felix3_initialize_direct.installation;
DROP FUNCTION felix3_initialize_direct.guard_installation();
DROP FUNCTION felix3_initialize_direct.is_waiting(uuid,uuid,uuid,uuid,uuid,uuid,bytea);
DROP FUNCTION felix3_initialize_direct.scope_enabled(uuid,uuid,uuid,uuid,uuid);
DROP VIEW felix3_initialize_direct.current_installation;
DROP TABLE felix3_initialize_direct.reservation;
DROP TABLE felix3_initialize_direct.per_actor_key;
DROP TABLE felix3_initialize_direct.installation;
DROP SCHEMA felix3_initialize_direct;
