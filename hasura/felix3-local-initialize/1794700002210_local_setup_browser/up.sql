-- SPDX-License-Identifier: AGPL-3.0-only
-- Local overlay only. Apply transactionally after 2200. A pre-existing
-- attempt without an authenticated session binding cannot be adopted.
ALTER TABLE felix3_initialize_setup.attempt
    ADD COLUMN portal_session_sha256 text COLLATE "C",
    ADD COLUMN return_path text COLLATE "C",
    ADD COLUMN return_panel text COLLATE "C",
    ADD COLUMN delivery_revision integer NOT NULL DEFAULT 0 CHECK (delivery_revision >= 0),
    ADD CONSTRAINT local_setup_browser_binding CHECK (
        (portal_session_sha256 IS NULL AND return_path IS NULL AND return_panel IS NULL)
        OR (portal_session_sha256 IS NOT NULL AND portal_session_sha256 ~ '^[0-9a-f]{64}$'
            AND return_path IS NOT NULL AND return_panel IS NOT NULL AND (
                (return_path IN ('event-base','event-show','event-edit')
                    AND return_panel IN ('publish','signatures-requests'))
                OR (return_path IN ('election-base','election-show','election-edit')
                    AND return_panel = 'publish'))));
GRANT UPDATE (portal_session_sha256, return_path, return_panel, delivery_revision)
    ON felix3_initialize_setup.attempt TO sequent_os_ledger_api_owner;

SET LOCAL ROLE sequent_os_ledger_api_owner;
-- Harvest calls this in the SAME transaction as begin_setup. Store only a
-- digest of the verified portal session and two closed navigation enums.
CREATE FUNCTION felix3_initialize_setup.bind_browser(
    b felix3_initialize_setup.binding, challenge uuid, session_digest text, path text, panel text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_portal') THEN RETURN false; END IF;
    UPDATE felix3_initialize_setup.attempt a
       SET portal_session_sha256 = session_digest, return_path = path, return_panel = panel
      FROM felix3_initialize_setup.eligible(b,true) e
     WHERE (a.id,a.bound,a.officer) = (challenge,b,e.officer)
       AND a.abandoned_at IS NULL AND a.exchange_at IS NULL AND a.expires_at > clock_timestamp()
       AND a.portal_session_sha256 IS NULL;
    RETURN FOUND;
END;
$$;

CREATE FUNCTION felix3_initialize_setup.browser_attempt(
    b felix3_initialize_setup.binding, session_digest text
) RETURNS TABLE (attempt uuid, path text, panel text, revision integer)
LANGUAGE sql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
    SELECT a.id,a.return_path,a.return_panel,a.delivery_revision
      FROM felix3_initialize_setup.attempt a JOIN felix3_initialize_setup.eligible(b,false) e
        ON (a.bound,a.officer) = (b,e.officer)
     WHERE felix3_initialize_setup.principal('felix3_local_setup_portal')
       AND a.abandoned_at IS NULL AND a.portal_session_sha256 = session_digest;
$$;

-- Called after 2200 release in the same transaction. A replay of an HTTP
-- registration click cannot dispatch again, even before the device claims.
CREATE FUNCTION felix3_initialize_setup.take_delivery(
    b felix3_initialize_setup.binding, challenge uuid, session_digest text, reviewed_revision integer
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER STRICT SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT felix3_initialize_setup.principal('felix3_local_setup_portal') THEN RETURN false; END IF;
    UPDATE felix3_initialize_setup.attempt a SET delivery_revision = a.delivery_revision + 1
      FROM felix3_initialize_setup.eligible(b,true) e
     WHERE (a.id,a.bound,a.officer) = (challenge,b,e.officer) AND a.portal_session_sha256 = session_digest
       AND a.delivery_revision = reviewed_revision AND a.delivery_revision < 2147483647
       AND a.released_at IS NOT NULL AND a.abandoned_at IS NULL AND a.claimed_until IS NULL
       AND a.gold_until > clock_timestamp();
    RETURN FOUND;
END;
$$;
REVOKE ALL ON FUNCTION
    felix3_initialize_setup.bind_browser(felix3_initialize_setup.binding,uuid,text,text,text),
    felix3_initialize_setup.browser_attempt(felix3_initialize_setup.binding,text),
    felix3_initialize_setup.take_delivery(felix3_initialize_setup.binding,uuid,text,integer) FROM PUBLIC;
-- As in 2200, install no service grants and reject custom default ACLs.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_proc p CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
        WHERE p.pronamespace = 'felix3_initialize_setup'::regnamespace
          AND p.proname IN ('bind_browser','browser_attempt','take_delivery') AND a.grantee <> p.proowner) THEN
        RAISE EXCEPTION 'unexpected local setup browser privileges';
    END IF;
END;
$$;
RESET ROLE;
