-- Trusted production broker lane. The same ordinary action validator, global
-- nonce replay ledger, intent phases and one-shot device fence are retained.
-- No credentials or runtime memberships are provisioned by this migration.
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'sequent_os_gold_broker') THEN
        CREATE ROLE sequent_os_gold_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'sequent_os_gold_ca') THEN
        CREATE ROLE sequent_os_gold_ca NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'sequent_os_gold_gc') THEN
        CREATE ROLE sequent_os_gold_gc NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    END IF;
END;
$$;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_gold_broker, sequent_os_gold_ca, sequent_os_gold_gc;
GRANT USAGE ON TYPE sequent_backend.operational_signing_binding TO sequent_os_gold_broker;

CREATE TABLE sequent_backend.operational_gold_callback (
    challenge_id uuid PRIMARY KEY REFERENCES sequent_backend.operational_signing_intent(challenge_id) ON DELETE CASCADE,
    taken_at timestamptz
);
ALTER TABLE sequent_backend.operational_gold_callback OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON TABLE sequent_backend.operational_gold_callback FROM PUBLIC;

-- Short-lived encrypted messages, not proof/token material. Runtime
-- deployment must schedule physical GC of expired ciphertext. Reads refuse
-- expiry regardless of GC. No role has direct access to this table.
CREATE TABLE sequent_backend.operational_gold_work (
    first_attempt uuid PRIMARY KEY REFERENCES sequent_backend.operational_gold_callback(challenge_id) ON DELETE CASCADE,
    artifact_attempt uuid UNIQUE REFERENCES sequent_backend.operational_gold_callback(challenge_id) ON DELETE SET NULL,
    encrypted bytea CHECK (octet_length(encrypted) BETWEEN 29 AND 50331648),
    expires_at timestamptz NOT NULL,
    CHECK (first_attempt IS DISTINCT FROM artifact_attempt)
);
ALTER TABLE sequent_backend.operational_gold_work OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON TABLE sequent_backend.operational_gold_work FROM PUBLIC;

CREATE TABLE sequent_backend.operational_gold_device_reservation (
    reservation_id uuid PRIMARY KEY REFERENCES sequent_backend.operational_signing_device_grant(reservation_id) ON DELETE CASCADE
);
ALTER TABLE sequent_backend.operational_gold_device_reservation OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON TABLE sequent_backend.operational_gold_device_reservation FROM PUBLIC;

-- Prevent any old generic claimant from taking a production grant without
-- the CA snapshot. A trigger failure also rolls back the intent phase change
-- performed earlier in the same statement. Old/sealed grants are unchanged.
CREATE FUNCTION sequent_backend.operational_gold_require_ca()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF OLD.claimed_at IS NULL AND NEW.claimed_at IS NOT NULL AND NEW.ca_version IS NULL
       AND EXISTS (SELECT FROM sequent_backend.operational_gold_device_reservation AS r
                   WHERE r.reservation_id = NEW.reservation_id) THEN
        RAISE EXCEPTION 'production HSM claim requires current CA assessment' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;
ALTER FUNCTION sequent_backend.operational_gold_require_ca() OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.operational_gold_require_ca() FROM PUBLIC;
CREATE TRIGGER operational_gold_require_ca BEFORE UPDATE ON sequent_backend.operational_signing_device_grant
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_gold_require_ca();

-- The audited application applies document and payload receipts in one
-- savepoint. Defend that boundary from a partial receipt update as well:
-- the underlying document receipt must already be durably settled. Sealed
-- and historical non-Gold tuples do not enter this trigger's policy.
CREATE FUNCTION sequent_backend.operational_gold_require_document()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE expected text;
BEGIN
    IF NEW.phase <> 'applied' OR OLD.phase = 'applied' OR OLD.purpose <> 'approval-payload' THEN
        RETURN NEW;
    END IF;
    expected := CASE (OLD.binding).action
                    WHEN 'transmit-results' THEN 'eml'
                    WHEN 'generate-election-returns' THEN 'pdf-cms'
                    WHEN 'generate-reports' THEN 'pdf-cms'
                END;
    IF expected IS NOT NULL AND EXISTS (
        SELECT FROM sequent_backend.operational_gold_callback AS c WHERE c.challenge_id=OLD.challenge_id
    ) AND NOT EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent AS doc
        JOIN sequent_backend.operational_gold_device_reservation AS r
          ON r.reservation_id=doc.reservation_id
        JOIN sequent_backend.operational_signing_device_grant AS g
          ON g.reservation_id=r.reservation_id
        WHERE (doc.tenant_id,doc.event_id,doc.request_id,doc.actor,doc.purpose)
            = (OLD.tenant_id,OLD.event_id,OLD.request_id,OLD.actor,expected)
          AND doc.phase IN ('completed','applied') AND doc.receipt IS NOT NULL AND g.settled_at IS NOT NULL
          AND ((doc.binding).action,(doc.binding).payload_hash,(doc.binding).key_id,
               (doc.binding).key_version,(doc.binding).cert_fingerprint,(doc.binding).cert_spki)
            = ((OLD.binding).action,(OLD.binding).payload_hash,(OLD.binding).key_id,
               (OLD.binding).key_version,(OLD.binding).cert_fingerprint,(OLD.binding).cert_spki)
    ) THEN
        RAISE EXCEPTION 'production document receipt is required before apply' USING ERRCODE='42501';
    END IF;
    RETURN NEW;
END;
$$;
ALTER FUNCTION sequent_backend.operational_gold_require_document() OWNER TO sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.operational_gold_require_document() FROM PUBLIC;
CREATE TRIGGER operational_gold_require_document BEFORE UPDATE ON sequent_backend.operational_signing_intent
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.operational_gold_require_document();

CREATE FUNCTION sequent_backend.operational_gold_current_key(p sequent_backend.operational_signing_binding)
RETURNS boolean LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed'
       OR p.tenant_id IS NULL OR p.event_id IS NULL
       OR p.issuer IS NULL OR p.issuer !~ '^https://[^/@[:space:]]+(/[^[:space:]]*)?$' THEN
        RETURN false;
    END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0));
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p.tenant_id::text || ':' || p.event_id::text, 0));
    RETURN sequent_backend.operational_signing_event_hsm_required(p.tenant_id, p.event_id)
       AND EXISTS (
           SELECT FROM sequent_backend.operational_signing_current_approval_key(
               p.tenant_id, p.event_id, p.request_id, p.actor) AS k
           WHERE (k.key_id, k.key_version, k.cert_fingerprint, k.cert_spki, k.device_profile)
             = (p.key_id, p.key_version, p.cert_fingerprint, p.cert_spki, 'yubihsm')
       );
END;
$$;

CREATE FUNCTION sequent_backend.operational_gold_prepare(p sequent_backend.operational_signing_binding)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NOT sequent_backend.operational_gold_current_key(p) THEN RETURN false; END IF;
    -- An EML/PDF step is a new ceremony, never a reuse of payload proof.
    -- Require the earlier exact production payload receipt from this actor;
    -- settlement is not approval and does not weaken action/quorum checks.
    IF p.purpose IN ('pdf-cms', 'eml') AND NOT EXISTS (
        SELECT FROM sequent_backend.operational_signing_intent AS prior
        JOIN sequent_backend.operational_gold_callback AS c USING (challenge_id)
        JOIN sequent_backend.operational_gold_device_reservation AS r
          ON r.reservation_id = prior.reservation_id
        JOIN sequent_backend.operational_signing_device_grant AS g
          ON g.reservation_id = r.reservation_id
        WHERE (prior.tenant_id, prior.event_id, prior.request_id, prior.actor, prior.purpose)
            = (p.tenant_id, p.event_id, p.request_id, p.actor, 'approval-payload')
          AND prior.phase = 'completed' AND prior.receipt IS NOT NULL AND g.settled_at IS NOT NULL
          AND ((prior.binding).action, (prior.binding).payload_hash, (prior.binding).key_id,
               (prior.binding).key_version, (prior.binding).cert_fingerprint, (prior.binding).cert_spki)
            = (p.action, p.payload_hash, p.key_id, p.key_version, p.cert_fingerprint, p.cert_spki)
    ) THEN RETURN false; END IF;
    -- An expired, never-reserved challenge is known not to have reached
    -- hardware. Replacing this exact part leaves the global immutable
    -- issuer/nonce tombstone intact. Reserved or later is never removed.
    DELETE FROM sequent_backend.operational_signing_intent AS old
    USING sequent_backend.operational_gold_callback AS c
    WHERE c.challenge_id=old.challenge_id
      AND (old.tenant_id,old.event_id,old.request_id,old.actor,old.purpose,(old.binding).action)
        = (p.tenant_id,p.event_id,p.request_id,p.actor,p.purpose,p.action)
      AND old.phase='prepared' AND old.reservation_id IS NULL
      AND (old.binding).intent_expires_at <= clock_timestamp();
    IF NOT sequent_backend.operational_signing_prepare(p) THEN RETURN false; END IF;
    INSERT INTO sequent_backend.operational_gold_callback (challenge_id) VALUES (p.challenge_id);
    RETURN true;
END;
$$;

-- Burn this exact callback before the external token exchange. Neither
-- a retry nor a second concurrent request may redeem the same attempt.
CREATE FUNCTION sequent_backend.operational_gold_take_callback(
    p sequent_backend.operational_signing_binding
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NOT sequent_backend.operational_gold_current_key(p) THEN RETURN false; END IF;
    UPDATE sequent_backend.operational_gold_callback AS c SET taken_at = clock_timestamp()
    FROM sequent_backend.operational_signing_intent AS i
    WHERE c.challenge_id = i.challenge_id AND c.taken_at IS NULL
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.purpose, i.challenge_id)
        = (p.tenant_id, p.event_id, p.request_id, p.actor, p.purpose, p.challenge_id)
      AND i.binding IS NOT DISTINCT FROM p AND i.phase = 'prepared'
      AND p.intent_expires_at > clock_timestamp();
    RETURN FOUND;
END;
$$;

-- Pod-independent callback lookup. Only the broker credential has EXECUTE;
-- binds the broker's authenticated actor and complete scope, never just a UUID.
CREATE FUNCTION sequent_backend.operational_gold_load_attempt(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid
) RETURNS jsonb LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT to_jsonb(i.binding)
    FROM sequent_backend.operational_gold_callback AS c
    JOIN sequent_backend.operational_signing_intent AS i USING (challenge_id)
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.challenge_id)
        = (p_tenant, p_event, p_request, p_actor, p_challenge)
      AND c.taken_at IS NULL AND i.phase = 'prepared'
      AND (i.binding).intent_expires_at > clock_timestamp()
      AND sequent_backend.operational_signing_event_hsm_required(p_tenant, p_event);
$$;

-- Read-only reconciliation after an ambiguous transport outcome. This
-- exposes no proof, CA inputs, signing bytes or other official's status.
CREATE FUNCTION sequent_backend.operational_gold_status(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid
) RETURNS TABLE(phase text, purpose text, recorded boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT CASE WHEN i.phase = 'prepared' AND c.taken_at IS NOT NULL
                THEN 'callback-taken' ELSE i.phase END,
           i.purpose, coalesce(g.settled_at IS NOT NULL
             AND i.phase IN ('completed','applied') AND i.receipt IS NOT NULL, false)
    FROM sequent_backend.operational_gold_callback AS c
    JOIN sequent_backend.operational_signing_intent AS i USING (challenge_id)
    LEFT JOIN sequent_backend.operational_gold_device_reservation AS r
      ON r.reservation_id = i.reservation_id
    LEFT JOIN sequent_backend.operational_signing_device_grant AS g
      ON g.reservation_id = r.reservation_id
    WHERE (i.tenant_id, i.event_id, i.request_id, i.actor, i.challenge_id)
        = (p_tenant, p_event, p_request, p_actor, p_challenge);
$$;

CREATE FUNCTION sequent_backend.operational_gold_store_work(
    p sequent_backend.operational_signing_binding, p_encrypted bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF p.purpose <> 'approval-payload' OR p_encrypted IS NULL
       OR octet_length(p_encrypted) NOT BETWEEN 29 AND 50331648 THEN RETURN false; END IF;
    INSERT INTO sequent_backend.operational_gold_work(first_attempt,encrypted,expires_at)
    SELECT i.challenge_id,p_encrypted,least((i.binding).challenge_issued_at + interval '8 minutes',
                                           coalesce((i.binding).request_expires_at,'infinity'))
    FROM sequent_backend.operational_signing_intent AS i
    JOIN sequent_backend.operational_gold_callback AS c USING (challenge_id)
    WHERE i.challenge_id=p.challenge_id AND i.binding IS NOT DISTINCT FROM p AND i.phase='prepared'
      AND c.taken_at IS NULL AND (i.binding).intent_expires_at > clock_timestamp()
    ON CONFLICT DO NOTHING;
    RETURN FOUND;
END;
$$;

CREATE FUNCTION sequent_backend.operational_gold_link_work(
    p sequent_backend.operational_signing_binding, p_first uuid, p_new_work bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF p.purpose NOT IN ('eml','pdf-cms')
       OR (p_new_work IS NOT NULL AND octet_length(p_new_work) NOT BETWEEN 29 AND 50331648)
    THEN RETURN false; END IF;
    UPDATE sequent_backend.operational_gold_work AS w
    SET artifact_attempt = p.challenge_id,
        encrypted = coalesce(p_new_work,w.encrypted),
        expires_at = (child.binding).intent_expires_at
    FROM sequent_backend.operational_signing_intent AS parent,
         sequent_backend.operational_signing_intent AS child,
         sequent_backend.operational_gold_callback AS c
    WHERE w.first_attempt=p_first AND parent.challenge_id=w.first_attempt
      AND child.challenge_id=p.challenge_id AND c.challenge_id=child.challenge_id
      AND (p_new_work IS NOT NULL OR (w.expires_at > clock_timestamp() AND w.encrypted IS NOT NULL))
      AND w.artifact_attempt IS NULL
      AND parent.phase='completed' AND child.phase='prepared' AND c.taken_at IS NULL
      AND child.binding IS NOT DISTINCT FROM p
      AND (parent.tenant_id,parent.event_id,parent.request_id,parent.actor,parent.purpose,
           (parent.binding).action,(parent.binding).payload_hash)
        = (p.tenant_id,p.event_id,p.request_id,p.actor,'approval-payload',p.action,p.payload_hash);
    RETURN FOUND;
END;
$$;

CREATE FUNCTION sequent_backend.operational_gold_load_work(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_action text, p_challenge uuid
) RETURNS TABLE(first_attempt uuid, encrypted bytea)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT w.first_attempt, w.encrypted
    FROM sequent_backend.operational_gold_work AS w
    JOIN sequent_backend.operational_signing_intent AS i ON i.challenge_id = p_challenge
    WHERE w.encrypted IS NOT NULL AND w.expires_at > clock_timestamp() AND
          (p_challenge = w.first_attempt OR p_challenge = w.artifact_attempt)
      AND (i.tenant_id,i.event_id,i.request_id,i.actor,(i.binding).action,i.challenge_id)
        = (p_tenant,p_event,p_request,p_actor,p_action,p_challenge)
      AND sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event);
$$;

CREATE FUNCTION sequent_backend.operational_gold_reconcile(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_action text
) RETURNS TABLE(challenge uuid, purpose text, recorded boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT i.challenge_id, i.purpose,
           coalesce(g.settled_at IS NOT NULL
             AND i.phase IN ('completed','applied') AND i.receipt IS NOT NULL, false)
    FROM sequent_backend.operational_gold_callback AS c
    JOIN sequent_backend.operational_signing_intent AS i USING (challenge_id)
    LEFT JOIN sequent_backend.operational_gold_device_reservation AS r
      ON r.reservation_id = i.reservation_id
    LEFT JOIN sequent_backend.operational_signing_device_grant AS g
      ON g.reservation_id = r.reservation_id
    WHERE (i.tenant_id,i.event_id,i.request_id,i.actor,(i.binding).action)
        = (p_tenant,p_event,p_request,p_actor,p_action)
      AND NOT (i.phase='prepared' AND i.reservation_id IS NULL
               AND (i.binding).intent_expires_at <= clock_timestamp())
      AND sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event);
$$;

-- Determine whether a new authorized Begin may resume just the document
-- part. Read-only; a previously reserved, invoked or active artifact is
-- never replaced. Prepare/link still recheck under the signing event lock.
CREATE FUNCTION sequent_backend.operational_gold_resume_payload(
    p_tenant uuid,p_event uuid,p_request uuid,p_actor text,p_action text
) RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT i.challenge_id
    FROM sequent_backend.operational_gold_work AS w
    JOIN sequent_backend.operational_signing_intent AS i ON i.challenge_id=w.first_attempt
    JOIN sequent_backend.operational_gold_device_reservation AS r
      ON r.reservation_id=i.reservation_id
    JOIN sequent_backend.operational_signing_device_grant AS g
      ON g.reservation_id=r.reservation_id
    LEFT JOIN sequent_backend.operational_signing_intent AS doc ON doc.challenge_id=w.artifact_attempt
    WHERE (i.tenant_id,i.event_id,i.request_id,i.actor,(i.binding).action,i.purpose)
      = (p_tenant,p_event,p_request,p_actor,p_action,'approval-payload')
      AND p_action IN ('transmit-results','generate-election-returns','generate-reports')
      AND i.phase='completed' AND i.receipt IS NOT NULL AND g.settled_at IS NOT NULL
      AND (doc.challenge_id IS NULL OR (doc.phase='prepared' AND doc.reservation_id IS NULL
                  AND (doc.binding).intent_expires_at <= clock_timestamp()))
      AND sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event);
$$;

-- Physical GC runs as its own maintenance principal. Never remove group
-- metadata: an earlier durable payload receipt may still need its document.
CREATE FUNCTION sequent_backend.operational_gold_scrub_expired_work(p_limit integer)
RETURNS integer LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE count_scrubbed integer;
BEGIN
    IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 16 THEN RETURN 0; END IF;
    WITH expired AS (
        SELECT w.first_attempt
        FROM sequent_backend.operational_gold_work AS w
        WHERE w.encrypted IS NOT NULL AND w.expires_at <= clock_timestamp()
        ORDER BY w.expires_at,w.first_attempt LIMIT p_limit
        FOR UPDATE SKIP LOCKED
    )
    UPDATE sequent_backend.operational_gold_work AS w
    SET encrypted=NULL
    FROM expired
    WHERE w.first_attempt=expired.first_attempt;
    GET DIAGNOSTICS count_scrubbed = ROW_COUNT;
    RETURN count_scrubbed;
END;
$$;

CREATE FUNCTION sequent_backend.operational_gold_reserve(
    p sequent_backend.operational_signing_binding, p_auth_time bigint, p_proof_expires_at timestamptz
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE fence uuid;
BEGIN
    IF NOT sequent_backend.operational_gold_current_key(p)
       OR NOT EXISTS (SELECT FROM sequent_backend.operational_gold_callback AS c
                      WHERE c.challenge_id = p.challenge_id AND c.taken_at IS NOT NULL) THEN RETURN NULL; END IF;
    fence := sequent_backend.operational_signing_reserve(p, p_auth_time, p_proof_expires_at);
    IF fence IS NULL THEN RETURN NULL; END IF;
    INSERT INTO sequent_backend.operational_signing_device_grant (
        reservation_id, challenge_id, tenant_id, event_id, request_id, actor, flow,
        device_profile, verified_issuer, verified_auth_time, proof_expires_at
    ) VALUES (
        fence, p.challenge_id, p.tenant_id, p.event_id, p.request_id, p.actor, 'generic',
        'yubihsm', p.issuer, p_auth_time, p_proof_expires_at
    );
    INSERT INTO sequent_backend.operational_gold_device_reservation VALUES (fence);
    RETURN fence;
END;
$$;

-- Only the exact live production device reference can read its CA inputs.
-- The device verifies the chain and revocation positively against these inputs.
CREATE FUNCTION sequent_backend.operational_gold_get_ca(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_fence uuid, p_binding_sha256 text
) RETURNS TABLE (
    binding jsonb, certificate_id uuid, chain_pem text[], grant_until timestamptz,
    ca_version text, anchors_pem text[], revocation_lists jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT to_jsonb(i.binding), k.certificate_id, k.chain_pem, i.reservation_deadline,
           sequent_backend.operational_initialize_ca_version(
               p_tenant, p_event, k.cert_fingerprint, k.cert_spki),
           (SELECT coalesce(array_agg(ca.pem ORDER BY ca.created_at, ca.id), '{}'::text[])
            FROM sequent_backend.certificate_authority AS ca
            WHERE (ca.tenant_id, ca.election_event_id, ca.purpose)
                = (p_tenant, p_event, 'staff-signatures')),
           (SELECT coalesce(jsonb_agg(jsonb_build_object(
                       'url', l.url, 'der_hex', encode(l.der, 'hex'), 'fetched_at', l.last_ok_at)
                       ORDER BY l.url), '[]'::jsonb)
            FROM sequent_backend.staff_crl AS l
            WHERE (l.tenant_id, l.election_event_id) = (p_tenant, p_event)
              AND l.der IS NOT NULL AND l.last_ok_at IS NOT NULL)
    FROM sequent_backend.operational_gold_device_reservation AS r
    JOIN sequent_backend.operational_signing_device_grant AS g USING (reservation_id)
    JOIN sequent_backend.operational_signing_intent AS i USING (reservation_id)
    CROSS JOIN LATERAL sequent_backend.operational_signing_current_approval_key(
        p_tenant, p_event, p_request, p_actor) AS k
    WHERE r.reservation_id = p_fence AND g.challenge_id = p_challenge
      AND (g.tenant_id, g.event_id, g.request_id, g.actor) = (p_tenant, p_event, p_request, p_actor)
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.challenge_id)
        = (p_tenant, p_event, p_request, p_actor, p_challenge)
      AND g.flow = 'generic' AND g.device_profile = 'yubihsm' AND g.claimed_at IS NULL
      AND i.phase = 'reserved' AND i.reservation_deadline > clock_timestamp()
      AND g.verified_issuer = (i.binding).issuer AND (i.binding).binding_sha256 = p_binding_sha256
      AND (k.key_id, k.key_version, k.cert_fingerprint, k.cert_spki, k.device_profile)
        = ((i.binding).key_id, (i.binding).key_version,
           (i.binding).cert_fingerprint, (i.binding).cert_spki, 'yubihsm')
      AND pg_has_role(session_user, 'sequent_os_gold_ca', 'USAGE')
      AND sequent_backend.operational_signing_event_hsm_required(p_tenant, p_event);
$$;

CREATE FUNCTION sequent_backend.operational_gold_claim(
    p_tenant uuid, p_event uuid, p_request uuid, p_actor text, p_challenge uuid,
    p_fence uuid, p_binding_sha256 text, p_ca_version text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE b sequent_backend.operational_signing_binding;
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed'
       OR p_tenant IS NULL OR p_event IS NULL OR p_fence IS NULL
       OR p_ca_version IS NULL OR p_ca_version !~ '^[0-9a-f]{64}$'
       OR NOT sequent_backend.operational_signing_device_caller('generic') THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0));
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    SELECT (i.binding).* INTO b
    FROM sequent_backend.operational_signing_intent AS i
    JOIN sequent_backend.operational_gold_device_reservation AS r USING (reservation_id)
    JOIN sequent_backend.operational_signing_device_grant AS g USING (reservation_id)
    WHERE r.reservation_id = p_fence AND g.challenge_id = p_challenge
      AND (g.tenant_id,g.event_id,g.request_id,g.actor) = (p_tenant,p_event,p_request,p_actor)
      AND (i.tenant_id,i.event_id,i.request_id,i.actor,i.challenge_id)
        = (p_tenant,p_event,p_request,p_actor,p_challenge)
      AND g.flow='generic' AND g.device_profile='yubihsm' AND g.claimed_at IS NULL
      AND g.verified_issuer=(i.binding).issuer
      AND i.phase='reserved' AND i.reservation_deadline > clock_timestamp()
      AND (i.binding).binding_sha256 = p_binding_sha256
    FOR UPDATE OF g;
    -- The device credential cannot read or compute this version. It is
    -- furnished only by the independently authenticated positive CA result.
    IF NOT FOUND OR NOT sequent_backend.operational_gold_current_key(b)
       OR p_ca_version IS DISTINCT FROM sequent_backend.operational_initialize_ca_version(
             p_tenant,p_event,b.cert_fingerprint,b.cert_spki)
       OR NOT EXISTS (
            SELECT FROM sequent_backend.certificate_authority AS ca
            WHERE (ca.tenant_id,ca.election_event_id,ca.purpose)
                    = (p_tenant,p_event,'staff-signatures'))
       OR NOT sequent_backend.operational_signing_claim_device(b, p_fence) THEN
        RETURN false;
    END IF;
    UPDATE sequent_backend.operational_signing_device_grant
    SET claimed_at = clock_timestamp(), ca_version = p_ca_version
    WHERE reservation_id = p_fence AND claimed_at IS NULL;
    IF NOT FOUND THEN RAISE EXCEPTION 'production device grant changed while claiming'; END IF;
    RETURN true;
END;
$$;

REVOKE ALL ON FUNCTION
    sequent_backend.operational_gold_scrub_expired_work(integer),
    sequent_backend.operational_gold_current_key(sequent_backend.operational_signing_binding),
    sequent_backend.operational_gold_prepare(sequent_backend.operational_signing_binding),
    sequent_backend.operational_gold_store_work(sequent_backend.operational_signing_binding,bytea),
    sequent_backend.operational_gold_link_work(sequent_backend.operational_signing_binding,uuid,bytea),
    sequent_backend.operational_gold_load_work(uuid,uuid,uuid,text,text,uuid),
    sequent_backend.operational_gold_resume_payload(uuid,uuid,uuid,text,text),
    sequent_backend.operational_gold_reconcile(uuid,uuid,uuid,text,text),
    sequent_backend.operational_gold_status(uuid,uuid,uuid,text,uuid),
    sequent_backend.operational_gold_load_attempt(uuid,uuid,uuid,text,uuid),
    sequent_backend.operational_gold_take_callback(sequent_backend.operational_signing_binding),
    sequent_backend.operational_gold_reserve(sequent_backend.operational_signing_binding,bigint,timestamptz),
    sequent_backend.operational_gold_get_ca(uuid,uuid,uuid,text,uuid,uuid,text),
    sequent_backend.operational_gold_claim(uuid,uuid,uuid,text,uuid,uuid,text,text) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_gold_scrub_expired_work(integer) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_current_key(sequent_backend.operational_signing_binding) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_prepare(sequent_backend.operational_signing_binding) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_store_work(sequent_backend.operational_signing_binding,bytea) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_link_work(sequent_backend.operational_signing_binding,uuid,bytea) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_load_work(uuid,uuid,uuid,text,text,uuid) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_resume_payload(uuid,uuid,uuid,text,text) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_reconcile(uuid,uuid,uuid,text,text) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_status(uuid,uuid,uuid,text,uuid) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_load_attempt(uuid,uuid,uuid,text,uuid) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_take_callback(sequent_backend.operational_signing_binding) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_reserve(sequent_backend.operational_signing_binding,bigint,timestamptz) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_get_ca(uuid,uuid,uuid,text,uuid,uuid,text) OWNER TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_gold_claim(uuid,uuid,uuid,text,uuid,uuid,text,text) OWNER TO sequent_os_ledger_api_owner;

GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_gold_prepare(sequent_backend.operational_signing_binding),
    sequent_backend.operational_gold_store_work(sequent_backend.operational_signing_binding,bytea),
    sequent_backend.operational_gold_link_work(sequent_backend.operational_signing_binding,uuid,bytea),
    sequent_backend.operational_gold_load_work(uuid,uuid,uuid,text,text,uuid),
    sequent_backend.operational_gold_resume_payload(uuid,uuid,uuid,text,text),
    sequent_backend.operational_gold_reconcile(uuid,uuid,uuid,text,text),
    sequent_backend.operational_gold_status(uuid,uuid,uuid,text,uuid),
    sequent_backend.operational_gold_load_attempt(uuid,uuid,uuid,text,uuid),
    sequent_backend.operational_gold_take_callback(sequent_backend.operational_signing_binding),
    sequent_backend.operational_gold_reserve(sequent_backend.operational_signing_binding,bigint,timestamptz)
    TO sequent_os_gold_broker;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_gold_get_ca(uuid,uuid,uuid,text,uuid,uuid,text)
    TO sequent_os_gold_ca;
GRANT EXECUTE ON FUNCTION
    sequent_backend.operational_gold_claim(uuid,uuid,uuid,text,uuid,uuid,text,text)
    TO sequent_os_device;

GRANT EXECUTE ON FUNCTION sequent_backend.operational_gold_scrub_expired_work(integer) TO sequent_os_gold_gc;
