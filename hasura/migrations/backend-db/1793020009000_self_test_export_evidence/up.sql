-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE OR REPLACE FUNCTION sequent_backend.execution_security_evidence(tenant uuid,event uuid)
RETURNS jsonb LANGUAGE sql STABLE AS $$
SELECT jsonb_build_object(
    'format','sequent-execution-security-1.0','tenant_id',tenant,'election_event_id',event,
    -- The primary audit log may have been unavailable during a failed test.
    -- Preserve the durable full result, including start/stop and every check,
    -- without depending on successful delivery of its completion log record.
    'self_tests',COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.started_at,r.id)
        FROM sequent_backend.self_test_run r WHERE r.tenant_id=tenant AND r.election_event_id=event),'[]'::jsonb),
    'self_test_settings',(SELECT to_jsonb(s) FROM sequent_backend.self_test_settings s
        WHERE s.tenant_id=tenant AND s.election_event_id=event),
    'self_test_baseline',(SELECT to_jsonb(b) FROM sequent_backend.self_test_baseline b
        WHERE b.tenant_id=tenant AND b.election_event_id=event),
    'attack_settings',COALESCE((SELECT to_jsonb(s) FROM sequent_backend.attack_detection_settings s WHERE s.tenant_id=tenant),
        jsonb_build_object('failed_logins_per_minute',20,'requests_per_minute',2000)),
    'notices',COALESCE((SELECT jsonb_agg(to_jsonb(n) ORDER BY n.created_at,n.id)
        FROM sequent_backend.lifecycle_notice n WHERE n.tenant_id=tenant AND n.election_event_id=event AND n.kind<>'ExecutionDataExported'),'[]'::jsonb),
    'authentication_activity',COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY a.occurred_at,a.id)
        FROM sequent_backend.attack_auth_activity a WHERE a.tenant_id=tenant AND a.election_event_id=event),'[]'::jsonb),
    'detections',COALESCE((SELECT jsonb_agg(to_jsonb(d) ORDER BY d.detected_at,d.id)
        FROM sequent_backend.attack_detection d WHERE d.tenant_id=tenant AND (d.election_event_id IS NULL OR d.election_event_id=event)),'[]'::jsonb)
)
FROM sequent_backend.election_event e WHERE e.tenant_id=tenant AND e.id=event;
$$;

-- Reuse the event-scoped source watermark and the deletion parent-row lock.
-- No-op writes do not change the watermark; authorized cascading cleanup does
-- not invalidate its own receipt or recreate an erased event's revision row.
DO $$ DECLARE tab text; BEGIN
    FOREACH tab IN ARRAY ARRAY['self_test_run','self_test_settings','self_test_baseline'] LOOP
        EXECUTE format('CREATE TRIGGER track_self_test_execution_revision AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.%I FOR EACH ROW EXECUTE FUNCTION sequent_backend.track_security_execution_revision()',tab);
    END LOOP;
END $$;

-- An archive made before this correction did not include these full rows.
-- Existing receipts must require a fresh export, even if no new test runs.
DO $$ DECLARE affected record; BEGIN
    FOR affected IN SELECT e.tenant_id,e.id FROM sequent_backend.election_event e
        WHERE EXISTS(SELECT FROM sequent_backend.self_test_run r WHERE r.tenant_id=e.tenant_id AND r.election_event_id=e.id)
           OR EXISTS(SELECT FROM sequent_backend.self_test_settings s WHERE s.tenant_id=e.tenant_id AND s.election_event_id=e.id)
           OR EXISTS(SELECT FROM sequent_backend.self_test_baseline b WHERE b.tenant_id=e.tenant_id AND b.election_event_id=e.id)
        ORDER BY e.id FOR KEY SHARE LOOP
        INSERT INTO sequent_backend.election_execution_revision(tenant_id,election_event_id,revision)
        VALUES(affected.tenant_id,affected.id,1)
        ON CONFLICT(election_event_id) DO UPDATE SET revision=election_execution_revision.revision+1;
    END LOOP;
END $$;
