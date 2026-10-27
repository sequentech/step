-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DO $$ DECLARE tab text; BEGIN
    FOREACH tab IN ARRAY ARRAY['self_test_run','self_test_settings','self_test_baseline'] LOOP
        EXECUTE format('DROP TRIGGER track_self_test_execution_revision ON sequent_backend.%I',tab);
    END LOOP;
END $$;
-- Keep advanced source revisions: rollback must not revive an old receipt.

CREATE OR REPLACE FUNCTION sequent_backend.execution_security_evidence(tenant uuid,event uuid)
RETURNS jsonb LANGUAGE sql STABLE AS $$
SELECT jsonb_build_object(
    'format','sequent-execution-security-1.0','tenant_id',tenant,'election_event_id',event,
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
