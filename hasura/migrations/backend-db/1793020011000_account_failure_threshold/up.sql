-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
ALTER TABLE sequent_backend.attack_detection_settings
 ADD COLUMN account_failure_threshold integer NOT NULL DEFAULT 5 CHECK(account_failure_threshold BETWEEN 1 AND 100000);

-- The tenant's threshold decides the per-account comparison; the realm's own
-- failure limit is only recorded. Failures that arrive without an account count
-- together per realm, because an external authentication server may not name one.
CREATE OR REPLACE FUNCTION sequent_backend.record_attack_auth_activity(
 p_id uuid,p_tenant uuid,p_event uuid,p_realm text,p_account text,p_time timestamptz,p_limit integer,p_window integer
) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE inserted integer; aggregate_limit bigint; account_limit integer; observed bigint; bucket_end timestamptz; evaluation_time timestamptz; scope text; scope_window integer;
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
 SELECT COALESCE((SELECT account_failure_threshold FROM sequent_backend.attack_detection_settings WHERE tenant_id=p_tenant),5) INTO account_limit;
 IF p_account IS NOT NULL THEN
  scope:=p_realm||':'||p_account;
  scope_window:=p_window;
  SELECT max(occurred_at) INTO evaluation_time FROM sequent_backend.attack_auth_activity WHERE tenant_id=p_tenant AND realm=p_realm AND account_sha256=p_account;
 ELSE
  scope:=p_realm||':unidentified';
  scope_window:=LEAST(p_window,900);
  SELECT max(occurred_at) INTO evaluation_time FROM sequent_backend.attack_auth_activity WHERE tenant_id=p_tenant AND realm=p_realm AND account_sha256 IS NULL;
 END IF;
 -- One detection per window for each scope: a repeat adds no record until a full
 -- window has passed, so the next one counts only later failures. A changed
 -- threshold applies at once. Checking before counting keeps a burst of failures
 -- from recounting the window while holding the tenant's lock.
 IF EXISTS(SELECT 1 FROM sequent_backend.attack_detection d
   WHERE d.tenant_id=p_tenant AND d.category='brute-force-account' AND d.scope_key=scope AND d.threshold=account_limit
     AND d.window_end>evaluation_time-make_interval(secs=>scope_window)) THEN
  RETURN true;
 END IF;
 IF p_account IS NOT NULL THEN
  SELECT count(*) INTO observed FROM sequent_backend.attack_auth_activity
   WHERE tenant_id=p_tenant AND realm=p_realm AND account_sha256=p_account
     AND occurred_at>=evaluation_time-make_interval(secs=>scope_window) AND occurred_at<=evaluation_time;
 ELSE
  SELECT count(*) INTO observed FROM sequent_backend.attack_auth_activity
   WHERE tenant_id=p_tenant AND realm=p_realm AND account_sha256 IS NULL
     AND occurred_at>=evaluation_time-make_interval(secs=>scope_window) AND occurred_at<=evaluation_time;
 END IF;
 IF observed>=account_limit THEN
  bucket_end:=date_trunc('minute',evaluation_time)+interval '1 minute';
  INSERT INTO sequent_backend.attack_detection(tenant_id,election_event_id,category,scope_key,window_start,window_end,observed_count,threshold,details)
  VALUES(p_tenant,p_event,'brute-force-account',scope,evaluation_time-make_interval(secs=>scope_window),bucket_end,observed,account_limit,
   jsonb_build_object('source','Keycloak authentication activity','scope',CASE WHEN p_account IS NULL THEN 'failures without an identified account' ELSE 'one account' END,'realm',p_realm,'account_sha256',p_account,'window_seconds',scope_window))
  ON CONFLICT(tenant_id,category,scope_key,window_end) DO UPDATE SET observed_count=GREATEST(attack_detection.observed_count,EXCLUDED.observed_count);
 END IF;
 RETURN true;
END $$;

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
        jsonb_build_object('failed_logins_per_minute',20,'requests_per_minute',2000,'account_failure_threshold',5)),
    'notices',COALESCE((SELECT jsonb_agg(to_jsonb(n) ORDER BY n.created_at,n.id)
        FROM sequent_backend.lifecycle_notice n WHERE n.tenant_id=tenant AND n.election_event_id=event AND n.kind<>'ExecutionDataExported'),'[]'::jsonb),
    'authentication_activity',COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY a.occurred_at,a.id)
        FROM sequent_backend.attack_auth_activity a WHERE a.tenant_id=tenant AND a.election_event_id=event),'[]'::jsonb),
    'detections',COALESCE((SELECT jsonb_agg(to_jsonb(d) ORDER BY d.detected_at,d.id)
        FROM sequent_backend.attack_detection d WHERE d.tenant_id=tenant AND (d.election_event_id IS NULL OR d.election_event_id=event)),'[]'::jsonb)
)
FROM sequent_backend.election_event e WHERE e.tenant_id=tenant AND e.id=event;
$$;
