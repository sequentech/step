-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Local overlay; run transactionally as the trusted audit-outbox migration owner after 2210.
-- No credential, table grant, new login or callable outbox interface.
CREATE FUNCTION sequent_backend.audit_local_setup_browser_transition()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp AS $$
DECLARE phase text; description text;
BEGIN
    IF session_user <> 'felix3_local_setup_portal' THEN
        RAISE EXCEPTION 'unexpected local setup browser writer';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.bound IS DISTINCT FROM OLD.bound THEN
        RAISE EXCEPTION 'local setup audit identity cannot change';
    END IF;
    IF OLD.portal_session_sha256 IS NULL AND NEW.portal_session_sha256 IS NOT NULL
        AND NEW.delivery_revision = OLD.delivery_revision THEN
        phase := 'browser_bound';
        description := 'Bound an authorized local Initialize setup attempt to its browser';
    ELSIF NEW.portal_session_sha256 = OLD.portal_session_sha256
        AND NEW.delivery_revision::bigint = OLD.delivery_revision::bigint + 1 THEN
        phase := 'delivery_claimed';
        description := 'Claimed delivery of an authorized local Initialize setup attempt';
    ELSE
        RAISE EXCEPTION 'unexpected local setup browser transition';
    END IF;
    -- The existing trusted purpose functions pin the authenticated official and
    -- Post to this immutable row. Do not copy the row, digest, path or nonce.
    INSERT INTO sequent_backend.audit_log_outbox
        (tenant_id, election_event_id, election_id, statement_kind, event_type,
         log_type, user_id, description, details)
    VALUES ((NEW.bound).tenant, (NEW.bound).event, (NEW.bound).post,
         'AdminChange', 'USER', 'INFO', (NEW.bound).official::text, description,
         jsonb_build_object(
             'object', jsonb_build_object('type', 'operational_initialize_attempt', 'id', NEW.id),
             'action', 'update', 'origin', 'harvest', 'changes', '[]'::jsonb,
             'actor', jsonb_build_object('user_id', (NEW.bound).official),
             'extra', jsonb_build_object('phase', phase, 'signing_request_id', (NEW.bound).request)));
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.audit_local_setup_browser_transition() FROM PUBLIC;
-- Reject default ACLs that accidentally expose the audit function to any
-- runtime principal. The row-level invocation itself needs no runtime grant.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_proc p
        CROSS JOIN LATERAL aclexplode(coalesce(p.proacl, acldefault('f', p.proowner))) a
        WHERE p.oid = 'sequent_backend.audit_local_setup_browser_transition()'::regprocedure
          AND a.grantee <> p.proowner) THEN
        RAISE EXCEPTION 'unexpected local setup audit privileges';
    END IF;
END;
$$;
CREATE TRIGGER local_setup_browser_audit
AFTER UPDATE OF portal_session_sha256, delivery_revision ON felix3_initialize_setup.attempt
FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_local_setup_browser_transition();
ALTER TABLE felix3_initialize_setup.attempt ENABLE ALWAYS TRIGGER local_setup_browser_audit;
