-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Apply after the standard 1794700002340 migration, in the same isolated demo
-- lineage as 2295 and 2302. Lets the central MIRU registry classify, and so
-- withhold, the output of an event this installation already scopes. It adds
-- no tenant, event allowlist, signer, proof or release authority.
SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));

-- The exact Post of the anchored event (the sticky action latch), or the
-- single Post of another event under the same installed device root.
-- Read-only; never selects a Post when the event has more than one.
CREATE FUNCTION felix3_initialize_direct.miru_event_scope(p_tenant uuid,p_event uuid,p_post uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER STRICT SET search_path=pg_catalog,pg_temp AS $$
 SELECT EXISTS (SELECT FROM felix3_initialize_direct.action_scope(p_tenant,p_event) s
                WHERE s.post_id=p_post)
     OR (EXISTS (SELECT FROM felix3_initialize_direct.action_device_root(p_tenant,p_event))
         AND (SELECT array_agg(e.id) FROM sequent_backend.election e
              WHERE (e.tenant_id,e.election_event_id)=(p_tenant,p_event)) = ARRAY[p_post]);
$$;
REVOKE ALL ON FUNCTION felix3_initialize_direct.miru_event_scope(uuid,uuid,uuid) FROM PUBLIC;
-- Only the registry's owner calls it, from operational_miru_direct_scope.
GRANT USAGE ON SCHEMA felix3_initialize_direct TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION felix3_initialize_direct.miru_event_scope(uuid,uuid,uuid)
 TO sequent_os_ledger_api_owner;
