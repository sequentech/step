-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Internal tables: never exposed through generic Hasura mutations.
CREATE TABLE sequent_backend.attack_detection_settings (
 tenant_id uuid PRIMARY KEY REFERENCES sequent_backend.tenant(id),
 failed_logins_per_minute bigint NOT NULL DEFAULT 20 CHECK(failed_logins_per_minute BETWEEN 1 AND 100000),
 requests_per_minute bigint NOT NULL DEFAULT 2000 CHECK(requests_per_minute BETWEEN 1 AND 10000000),
 updated_at timestamptz NOT NULL DEFAULT clock_timestamp(), updated_by text NOT NULL
);
CREATE TABLE sequent_backend.attack_auth_activity (
 id uuid PRIMARY KEY, tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant(id),
 election_event_id uuid, realm text NOT NULL, account_sha256 text,
 occurred_at timestamptz NOT NULL, failure_limit integer NOT NULL CHECK(failure_limit BETWEEN 1 AND 100000),
 window_seconds integer NOT NULL CHECK(window_seconds BETWEEN 1 AND 2592000),
 received_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX attack_auth_window ON sequent_backend.attack_auth_activity(tenant_id,occurred_at);
CREATE INDEX attack_auth_account ON sequent_backend.attack_auth_activity(tenant_id,realm,account_sha256,occurred_at);
CREATE TABLE sequent_backend.attack_request_bucket (
 generation text NOT NULL, minute_start timestamptz NOT NULL, host text NOT NULL,
 requests bigint NOT NULL CHECK(requests>=0), observed_at timestamptz NOT NULL,
 PRIMARY KEY(generation,minute_start,host)
);
CREATE TABLE sequent_backend.attack_activity_health (
 source text PRIMARY KEY, available boolean NOT NULL, checked_at timestamptz NOT NULL,
 reason text NOT NULL
);
CREATE TABLE sequent_backend.attack_detection (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant(id),
 election_event_id uuid, category text NOT NULL CHECK(category IN ('brute-force-account','brute-force-aggregate','network-dos')),
 scope_key text NOT NULL, window_start timestamptz NOT NULL, window_end timestamptz NOT NULL,
 detected_at timestamptz NOT NULL DEFAULT clock_timestamp(), observed_count bigint NOT NULL,
 threshold bigint NOT NULL CHECK(threshold>0), details jsonb NOT NULL,
 UNIQUE(tenant_id,category,scope_key,window_end)
);
CREATE INDEX attack_detection_feed ON sequent_backend.attack_detection(tenant_id,detected_at DESC);
CREATE TABLE sequent_backend.attack_detection_audit (
 detection_id uuid NOT NULL REFERENCES sequent_backend.attack_detection(id),
 election_event_id uuid NOT NULL REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE,
 notice_id uuid NOT NULL, PRIMARY KEY(detection_id,election_event_id)
);
-- The activity receipt and both comparisons commit together. A duplicate queue
-- delivery cannot increase either count or emit another detection.
CREATE FUNCTION sequent_backend.record_attack_auth_activity(
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

-- Missing, failed or stale attack-activity observations block voter interfaces.
-- Staff interfaces remain available to investigate and explicitly recover.
CREATE FUNCTION sequent_backend.attack_activity_allows_voters() RETURNS boolean LANGUAGE sql AS $$
 SELECT COALESCE((SELECT available AND checked_at BETWEEN clock_timestamp()-interval '30 seconds' AND clock_timestamp()
 FROM sequent_backend.attack_activity_health WHERE source='external-nginx'),false)
$$;
CREATE OR REPLACE VIEW sequent_backend.election_lifecycle_access AS
SELECT l.tenant_id,l.election_event_id,l.phase,l.safety,l.generation,
 (l.phase='execution' AND l.safety='operational'
  AND sequent_backend.audit_storage_allows_voters()
  AND sequent_backend.attack_activity_allows_voters()
  AND pg_try_advisory_xact_lock_shared(hashtextextended(l.tenant_id::text||':'||l.election_event_id::text,848527))
  AND (SELECT count(*) FROM sequent_backend.lifecycle_service_lease WHERE component IN ('harvest','windmill') AND expires_at>clock_timestamp())=2
  AND NOT EXISTS (SELECT 1 FROM sequent_backend.self_test_run f WHERE f.tenant_id=l.tenant_id AND f.election_event_id=l.election_event_id AND f.outcome='unrecoverable-failure')
  AND COALESCE((SELECT r.outcome='passed' AND r.finished_at BETWEEN clock_timestamp()-make_interval(mins=>COALESCE((s.settings->>'interval_minutes')::int,15)) AND clock_timestamp()
   FROM sequent_backend.self_test_run r LEFT JOIN sequent_backend.self_test_settings s USING(tenant_id,election_event_id)
   WHERE r.tenant_id=l.tenant_id AND r.election_event_id=l.election_event_id AND r.generation=l.generation AND r.finished_at IS NOT NULL
   ORDER BY r.finished_at DESC,r.id DESC LIMIT 1),false)) AS voter_allowed
FROM sequent_backend.election_lifecycle l;

-- Compare all externally observed requests, including those without authentication.
CREATE FUNCTION sequent_backend.compare_attack_request_activity() RETURNS void LANGUAGE sql AS $$
 INSERT INTO sequent_backend.attack_detection(tenant_id,category,scope_key,window_start,window_end,observed_count,threshold,details) SELECT t.id,'network-dos','cluster',b.minute_start,b.minute_start+interval '1 minute',b.requests,COALESCE(s.requests_per_minute,2000),jsonb_build_object('source','external-nginx','scope','all external cluster interfaces','includes_unauthenticated',true) FROM sequent_backend.tenant t LEFT JOIN sequent_backend.attack_detection_settings s ON s.tenant_id=t.id CROSS JOIN (SELECT minute_start,sum(requests)::bigint requests FROM sequent_backend.attack_request_bucket WHERE minute_start>=clock_timestamp()-interval '16 minutes' GROUP BY minute_start) b WHERE t.is_active AND b.requests>COALESCE(s.requests_per_minute,2000) ON CONFLICT(tenant_id,category,scope_key,window_end) DO UPDATE SET observed_count=GREATEST(attack_detection.observed_count,EXCLUDED.observed_count);
$$;
