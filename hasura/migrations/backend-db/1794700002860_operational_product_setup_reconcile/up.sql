-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Release an expired reservation only when there was provably no claim.
-- A claimed, completed, or ambiguous physical operation is never reset.
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_data_owner,sequent_os_ledger_api_owner;
CREATE TABLE sequent_backend.operational_setup_unclaimed_resolution (
    attempt uuid PRIMARY KEY,
    ticket uuid NOT NULL UNIQUE REFERENCES sequent_backend.operational_event_setup_request(id),
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    official uuid NOT NULL,
    initial_officer uuid NOT NULL,
    initial_auth_time timestamptz NOT NULL,
    dispatched_at timestamptz NOT NULL,
    release_officer uuid NOT NULL CHECK (release_officer<>official),
    release_auth_time timestamptz NOT NULL,
    released_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
ALTER TABLE sequent_backend.operational_setup_unclaimed_resolution OWNER TO sequent_os_ledger_data_owner;
REVOKE ALL ON TABLE sequent_backend.operational_setup_unclaimed_resolution FROM PUBLIC;
GRANT INSERT ON TABLE sequent_backend.operational_setup_unclaimed_resolution TO sequent_os_ledger_api_owner;
GRANT DELETE ON TABLE sequent_backend.operational_setup_generation TO sequent_os_ledger_api_owner;

CREATE FUNCTION sequent_backend.operational_reconcile_unclaimed_setup(
    p_tenant uuid,p_event uuid,p_ticket uuid,p_officer uuid,p_auth timestamptz
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp
AS $$
DECLARE
    ticket sequent_backend.operational_event_setup_request%ROWTYPE;
    candidate sequent_backend.operational_setup_generation%ROWTYPE;
    v_now timestamptz;
BEGIN
    IF p_officer IS NULL OR p_officer='00000000-0000-0000-0000-000000000000'
       OR p_auth IS NULL OR NOT isfinite(p_auth)
       OR current_setting('transaction_isolation')<>'read committed' THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0));
    IF NOT sequent_backend.operational_signing_event_hsm_required(p_tenant,p_event) THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('signing-event:'||p_tenant::text||':'||p_event::text,0));
    SELECT * INTO ticket FROM sequent_backend.operational_event_setup_request s
    WHERE (s.tenant_id,s.event_id,s.id)=(p_tenant,p_event,p_ticket)
      AND s.official<>p_officer AND s.superseded_at IS NULL AND s.dispatch_officer IS NOT NULL
    FOR UPDATE;
    IF NOT FOUND OR ticket.attempt IS NULL THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'operational-signing-official:'||p_tenant::text||':'||ticket.official::text,0));
    v_now:=clock_timestamp();
    IF p_auth>v_now OR p_auth<=v_now-interval '60 seconds' THEN RETURN false; END IF;
    SELECT * INTO candidate FROM sequent_backend.operational_setup_generation g
    WHERE (g.tenant_id,g.event_id,g.official,g.attempt,g.prepared_by)=
          (p_tenant,p_event,ticket.official,ticket.attempt,ticket.dispatch_officer)
      AND g.device_profile='yubihsm' AND g.step='prepared'
      AND g.fence IS NULL AND g.claimed_at IS NULL AND g.enrollment_id IS NULL
      AND g.claim_deadline<=v_now
    FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    INSERT INTO sequent_backend.operational_setup_unclaimed_resolution(
        attempt,ticket,tenant_id,event_id,official,initial_officer,initial_auth_time,
        dispatched_at,release_officer,release_auth_time,released_at
    ) VALUES (
        candidate.attempt,ticket.id,p_tenant,p_event,ticket.official,ticket.dispatch_officer,
        ticket.dispatch_auth_time,ticket.dispatched_at,p_officer,p_auth,v_now
    );
    -- Preserve the exact original approvals in the append-only record. The
    -- actor will need a fresh ticket and the officer must authorize anew.
    UPDATE sequent_backend.operational_event_setup_request s
    SET attempt=NULL,dispatch_officer=NULL,dispatch_auth_time=NULL,dispatched_at=NULL,superseded_at=v_now
    WHERE s.id=ticket.id AND s.attempt=candidate.attempt;
    IF NOT FOUND THEN RAISE EXCEPTION 'unclaimed setup resolution lost the exact ticket'; END IF;
    DELETE FROM sequent_backend.operational_setup_generation g
    WHERE g.attempt=candidate.attempt AND g.step='prepared' AND g.fence IS NULL;
    IF NOT FOUND THEN RAISE EXCEPTION 'unclaimed setup attempt was not released'; END IF;
    RETURN true;
END;
$$;
REVOKE ALL ON FUNCTION sequent_backend.operational_reconcile_unclaimed_setup(uuid,uuid,uuid,uuid,timestamptz) FROM PUBLIC;
ALTER FUNCTION sequent_backend.operational_reconcile_unclaimed_setup(uuid,uuid,uuid,uuid,timestamptz)
    OWNER TO sequent_os_ledger_api_owner;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_reconcile_unclaimed_setup(uuid,uuid,uuid,uuid,timestamptz)
    TO sequent_os_product_setup_dispatch;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner,sequent_os_ledger_data_owner;
