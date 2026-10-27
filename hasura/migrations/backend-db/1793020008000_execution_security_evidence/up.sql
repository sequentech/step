-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Security evidence is part of the exported execution state. Delivery of an
-- existing notice and the export's own completion receipt are not new evidence.
CREATE FUNCTION sequent_backend.track_security_execution_revision()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE data jsonb; tenant uuid; event uuid; affected record;
BEGIN
    IF TG_OP='UPDATE' THEN
        IF TG_TABLE_NAME='lifecycle_notice' THEN
            IF to_jsonb(OLD)-'logged_at'=to_jsonb(NEW)-'logged_at' THEN RETURN NULL; END IF;
        ELSIF OLD IS NOT DISTINCT FROM NEW THEN RETURN NULL;
        END IF;
        IF OLD.tenant_id IS DISTINCT FROM NEW.tenant_id THEN
            RAISE EXCEPTION 'Security evidence tenant cannot change';
        END IF;
        IF TG_TABLE_NAME<>'attack_detection_settings'
           AND (to_jsonb(OLD)->'election_event_id') IS DISTINCT FROM (to_jsonb(NEW)->'election_event_id') THEN
            RAISE EXCEPTION 'Security evidence event cannot change';
        END IF;
    END IF;
    data=CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
    IF TG_TABLE_NAME='lifecycle_notice' AND data->>'kind'='ExecutionDataExported' THEN RETURN NULL; END IF;
    tenant=(data->>'tenant_id')::uuid;
    event=(data->>'election_event_id')::uuid;
    -- Staff authentication is tenant administration, not an event's voter data.
    IF TG_TABLE_NAME='attack_auth_activity' AND event IS NULL THEN RETURN NULL; END IF;
    -- Hold the parent row so teardown cannot pass its receipt check while an
    -- applicable new security record is being committed. Deleted parents are
    -- absent here; their cascading cleanup must not recreate a revision row.
    FOR affected IN SELECT e.id FROM sequent_backend.election_event e
        WHERE e.tenant_id=tenant AND (event IS NULL OR e.id=event)
        ORDER BY e.id FOR KEY SHARE LOOP
        IF TG_OP='DELETE' AND sequent_backend.authorized_execution_deletion(tenant,affected.id) THEN CONTINUE; END IF;
        INSERT INTO sequent_backend.election_execution_revision(tenant_id,election_event_id,revision)
        VALUES(tenant,affected.id,1)
        ON CONFLICT(election_event_id) DO UPDATE SET revision=election_execution_revision.revision+1;
    END LOOP;
    RETURN NULL;
END $$;

DO $$ DECLARE tab text; BEGIN
    FOREACH tab IN ARRAY ARRAY['lifecycle_notice','attack_detection_settings','attack_auth_activity','attack_detection'] LOOP
        EXECUTE format('CREATE TRIGGER track_security_execution_revision AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.%I FOR EACH ROW EXECUTE FUNCTION sequent_backend.track_security_execution_revision()',tab);
    END LOOP;
END $$;

-- Retain shared tenant activity, but erase event-specific records when the
-- already exported event is deleted through its protected teardown workflow.
ALTER TABLE sequent_backend.attack_auth_activity ADD CONSTRAINT attack_auth_event
    FOREIGN KEY(election_event_id) REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE;
ALTER TABLE sequent_backend.attack_detection ADD CONSTRAINT attack_detection_event
    FOREIGN KEY(election_event_id) REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE;

CREATE FUNCTION sequent_backend.execution_security_evidence(tenant uuid,event uuid)
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
