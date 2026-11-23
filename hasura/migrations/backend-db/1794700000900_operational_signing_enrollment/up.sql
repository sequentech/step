-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Not tracked in Hasura. Only trusted server code with Security Officer scope
-- can manage these rows. Runtime must expose EXECUTE only and must not have
-- table DML or own these objects. The migration grants no application roles.
CREATE TABLE sequent_backend.operational_signing_official_enrollment (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant (id),
    actor text NOT NULL CHECK (btrim(actor) <> '' AND octet_length(actor) <= 1024),
    key_id text NOT NULL CHECK (btrim(key_id) <> '' AND octet_length(key_id) <= 1024),
    key_version integer NOT NULL CHECK (key_version > 0),
    spki text NOT NULL CHECK (spki ~ '^[0-9a-f]{64}$'),
    device_profile text NOT NULL CHECK (device_profile IN ('yubihsm', 'softhsm-test')),
    nonexportable_confirmed boolean NOT NULL CHECK (nonexportable_confirmed),
    device_evidence_hash text NOT NULL CHECK (device_evidence_hash ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    revoked_at timestamptz,
    UNIQUE (tenant_id, actor, id),
    UNIQUE (tenant_id, key_id, key_version),
    UNIQUE (tenant_id, spki)
);
CREATE UNIQUE INDEX operational_signing_one_active_device
    ON sequent_backend.operational_signing_official_enrollment (tenant_id, actor) WHERE revoked_at IS NULL;

CREATE TABLE sequent_backend.operational_signing_official_certificate (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    actor text NOT NULL,
    enrollment_id uuid NOT NULL,
    certificate_id uuid NOT NULL,
    certificate_fingerprint text NOT NULL CHECK (certificate_fingerprint ~ '^[0-9a-f]{64}$'),
    chain_pem text[] NOT NULL CHECK (cardinality(chain_pem) BETWEEN 1 AND 12 AND array_ndims(chain_pem) = 1),
    certificate_source text NOT NULL CHECK (certificate_source = 'validated-staff-ca'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    retired_at timestamptz,
    PRIMARY KEY (tenant_id, event_id, enrollment_id, certificate_id),
    FOREIGN KEY (tenant_id, actor, enrollment_id)
        REFERENCES sequent_backend.operational_signing_official_enrollment (tenant_id, actor, id),
    FOREIGN KEY (tenant_id, event_id, certificate_id)
        REFERENCES sequent_backend.staff_certificate (tenant_id, election_event_id, id)
        ON DELETE CASCADE
);
CREATE UNIQUE INDEX operational_signing_one_active_official_certificate
    ON sequent_backend.operational_signing_official_certificate (tenant_id, event_id, enrollment_id)
    WHERE retired_at IS NULL;

-- These functions use only fully qualified relations and a pinned search_path.
-- Before deployment transfer each function to its dedicated NOLOGIN owner
-- with only the table privileges it needs; explicitly grant EXECUTE only to
-- the separately authorized management and lookup callers. Never expose a
-- broad migration DB credential to the caller or link these as SQL API actions.
CREATE FUNCTION sequent_backend.operational_signing_enroll_official(
    p_tenant uuid, p_actor text, p_key_id text, p_key_version integer, p_spki text,
    p_device_profile text, p_nonexportable_confirmed boolean, p_evidence_hash text
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE i uuid;
BEGIN
    IF p_nonexportable_confirmed IS DISTINCT FROM true THEN RETURN NULL; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('operational-signing-official:' || p_tenant::text || ':' || p_actor, 0));
    -- p_* device evidence is from the separately authorized device verifier,
    -- never GraphQL/browser input. Device enrollment creates no CA trust.
    INSERT INTO sequent_backend.operational_signing_official_enrollment
        (tenant_id, actor, key_id, key_version, spki, device_profile, nonexportable_confirmed, device_evidence_hash)
    VALUES (p_tenant, p_actor, p_key_id, p_key_version, p_spki, p_device_profile, true, p_evidence_hash)
    ON CONFLICT DO NOTHING RETURNING id INTO i;
    RETURN i;
END;
$$;

CREATE FUNCTION sequent_backend.operational_signing_bind_official_certificate(
    p_tenant uuid, p_event uuid, p_actor text, p_enrollment uuid,
    p_certificate uuid, p_fingerprint text, p_chain text[]
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE
    p_spki text;
    leaf text;
    item text;
    at_time timestamptz;
BEGIN
    IF cardinality(p_chain) IS NULL OR cardinality(p_chain) NOT BETWEEN 1 AND 12
       OR array_ndims(p_chain) IS DISTINCT FROM 1 OR array_lower(p_chain, 1) IS DISTINCT FROM 1 THEN
        RETURN false;
    END IF;
    FOREACH item IN ARRAY p_chain LOOP
        IF item IS NULL OR octet_length(item) NOT BETWEEN 1 AND 32768 THEN RETURN false; END IF;
    END LOOP;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-official:' || p_tenant::text || ':' || p_actor, 0));
    SELECT e.spki INTO p_spki FROM sequent_backend.operational_signing_official_enrollment AS e
    WHERE (e.tenant_id, e.actor, e.id) = (p_tenant, p_actor, p_enrollment)
      AND e.revoked_at IS NULL;
    IF NOT FOUND THEN RETURN false; END IF;
    at_time := clock_timestamp();
    SELECT c.pem INTO leaf FROM sequent_backend.staff_certificate AS c
    WHERE (c.tenant_id, c.election_event_id, c.user_id, c.id) = (p_tenant, p_event, p_actor, p_certificate)
      AND c.fingerprint_sha256 = p_fingerprint AND c.spki_sha256 = p_spki
      AND c.status = 'active' AND c.not_before <= at_time AND c.not_after > at_time;
    IF NOT FOUND OR p_chain[1] IS DISTINCT FROM leaf THEN RETURN false; END IF;
    -- The trusted caller has already validated the entire chain against the
    -- existing staff CA policy, revocation and certificate/key proofs.
    INSERT INTO sequent_backend.operational_signing_official_certificate
        (tenant_id, event_id, actor, enrollment_id, certificate_id, certificate_fingerprint, chain_pem, certificate_source)
    VALUES (p_tenant, p_event, p_actor, p_enrollment, p_certificate, p_fingerprint, p_chain, 'validated-staff-ca')
    ON CONFLICT DO NOTHING;
    RETURN FOUND;
END;
$$;

CREATE FUNCTION sequent_backend.operational_signing_revoke_official(
    p_tenant uuid, p_actor text, p_enrollment uuid, p_key_id text, p_key_version integer
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('operational-signing-official:' || p_tenant::text || ':' || p_actor, 0));
    UPDATE sequent_backend.operational_signing_official_enrollment AS e
    SET revoked_at = clock_timestamp()
    WHERE (e.tenant_id, e.actor, e.id, e.key_id, e.key_version) =
          (p_tenant, p_actor, p_enrollment, p_key_id, p_key_version) AND e.revoked_at IS NULL;
    RETURN FOUND;
END;
$$;

CREATE FUNCTION sequent_backend.operational_signing_retire_official_certificate(
    p_tenant uuid, p_event uuid, p_actor text, p_enrollment uuid, p_certificate uuid, p_fingerprint text
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    UPDATE sequent_backend.operational_signing_official_certificate AS a
    SET retired_at = clock_timestamp()
    WHERE (a.tenant_id, a.event_id, a.actor, a.enrollment_id, a.certificate_id, a.certificate_fingerprint) =
          (p_tenant, p_event, p_actor, p_enrollment, p_certificate, p_fingerprint)
      AND a.retired_at IS NULL;
    RETURN FOUND;
END;
$$;

CREATE FUNCTION sequent_backend.operational_signing_get_official_chain(
    p_tenant uuid, p_event uuid, p_actor text, p_key_id text, p_key_version integer, p_fingerprint text, p_spki text
) RETURNS TABLE(enrollment_id uuid, certificate_id uuid, chain_pem text[], device_profile text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
BEGIN
    -- Event then official: matches binding and approval transaction order.
    PERFORM pg_advisory_xact_lock_shared(hashtextextended(
        'signing-event:' || p_tenant::text || ':' || p_event::text, 0));
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-official:' || p_tenant::text || ':' || p_actor, 0));
    RETURN QUERY SELECT e.id, a.certificate_id, a.chain_pem, e.device_profile
    FROM sequent_backend.operational_signing_official_enrollment AS e
    JOIN sequent_backend.operational_signing_official_certificate AS a
      ON (a.tenant_id, a.actor, a.enrollment_id) = (e.tenant_id, e.actor, e.id)
    JOIN sequent_backend.staff_certificate AS c
      ON (c.tenant_id, c.election_event_id, c.id) = (a.tenant_id, a.event_id, a.certificate_id)
    WHERE (e.tenant_id, e.actor, e.key_id, e.key_version, e.spki) =
          (p_tenant, p_actor, p_key_id, p_key_version, p_spki)
      AND e.revoked_at IS NULL AND e.nonexportable_confirmed
      AND a.event_id = p_event AND a.retired_at IS NULL
      AND a.certificate_fingerprint = p_fingerprint
      AND c.user_id = p_actor AND c.status = 'active'
      AND c.fingerprint_sha256 = p_fingerprint AND c.spki_sha256 = p_spki
      AND c.pem = a.chain_pem[1]
      AND c.not_before <= clock_timestamp() AND c.not_after > clock_timestamp();
END;
$$;

REVOKE ALL ON TABLE sequent_backend.operational_signing_official_enrollment,
    sequent_backend.operational_signing_official_certificate FROM PUBLIC;
REVOKE ALL ON FUNCTION
    sequent_backend.operational_signing_enroll_official(uuid, text, text, integer, text, text, boolean, text),
    sequent_backend.operational_signing_bind_official_certificate(uuid, uuid, text, uuid, uuid, text, text[]),
    sequent_backend.operational_signing_revoke_official(uuid, text, uuid, text, integer),
    sequent_backend.operational_signing_retire_official_certificate(uuid, uuid, text, uuid, uuid, text),
    sequent_backend.operational_signing_get_official_chain(uuid, uuid, text, text, integer, text, text) FROM PUBLIC;
