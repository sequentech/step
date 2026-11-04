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

CREATE OR REPLACE FUNCTION sequent_backend.record_attack_auth_activity(
 p_id uuid,p_tenant uuid,p_event uuid,p_realm text,p_account text,p_time timestamptz,p_limit integer,p_window integer
) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE inserted integer; aggregate_limit bigint; observed bigint; bucket_end timestamptz; evaluation_time timestamptz;
BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text,848528));
 IF p_time>clock_timestamp()+interval '5 seconds' OR p_time<clock_timestamp()-interval '30 days' THEN
  RAISE EXCEPTION 'Invalid authentication activity timestamp';
 END IF;
 INSERT INTO sequent_backend.attack_auth_activity(id,tenant_id,election_event_id,realm,account_sha256,occurred_at,failure_limit,window_seconds)
 VALUES(p_id,p_tenant,p_event,p_realm,p_account,p_time,p_limit,p_window) ON CONFLICT(id) DO NOTHING;
 GET DIAGNOSTICS inserted=ROW_COUNT;
 IF inserted=0 THEN RETURN false; END IF;
 SELECT COALESCE((SELECT failed_logins_per_minute FROM sequent_backend.attack_detection_settings WHERE tenant_id=p_tenant),20) INTO aggregate_limit;
 bucket_end:=date_trunc('minute',p_time)+interval '1 minute';
 -- Closed minute buckets are reproducible even when independent queues deliver out of order.
 SELECT count(*) INTO observed FROM sequent_backend.attack_auth_activity
 WHERE tenant_id=p_tenant AND occurred_at>=bucket_end-interval '1 minute' AND occurred_at<bucket_end;
 IF observed>aggregate_limit THEN
  INSERT INTO sequent_backend.attack_detection(tenant_id,category,scope_key,window_start,window_end,observed_count,threshold,details)
  VALUES(p_tenant,'brute-force-aggregate','tenant',bucket_end-interval '1 minute',bucket_end,observed,aggregate_limit,
   jsonb_build_object('source','Keycloak authentication activity','scope','all tenant accounts'))
  ON CONFLICT(tenant_id,category,scope_key,window_end) DO UPDATE SET observed_count=GREATEST(attack_detection.observed_count,EXCLUDED.observed_count);
 END IF;
 IF p_account IS NOT NULL THEN
  SELECT max(occurred_at) INTO evaluation_time FROM sequent_backend.attack_auth_activity WHERE tenant_id=p_tenant AND realm=p_realm AND account_sha256=p_account;
  bucket_end:=date_trunc('minute',evaluation_time)+interval '1 minute';
  SELECT count(*) INTO observed FROM sequent_backend.attack_auth_activity
   WHERE tenant_id=p_tenant AND realm=p_realm AND account_sha256=p_account
     AND occurred_at>=evaluation_time-make_interval(secs=>p_window) AND occurred_at<=evaluation_time;
  IF observed>=p_limit THEN
   INSERT INTO sequent_backend.attack_detection(tenant_id,election_event_id,category,scope_key,window_start,window_end,observed_count,threshold,details)
   VALUES(p_tenant,p_event,'brute-force-account',p_realm||':'||p_account,evaluation_time-make_interval(secs=>p_window),bucket_end,observed,p_limit,
    jsonb_build_object('source','Keycloak authentication activity','realm',p_realm,'account_sha256',p_account,'window_seconds',p_window))
   ON CONFLICT(tenant_id,category,scope_key,window_end) DO UPDATE SET observed_count=GREATEST(attack_detection.observed_count,EXCLUDED.observed_count);
  END IF;
 END IF;
 RETURN true;
END $$;

ALTER TABLE sequent_backend.attack_detection_settings DROP COLUMN account_failure_threshold;
