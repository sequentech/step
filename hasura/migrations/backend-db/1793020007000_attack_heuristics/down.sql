-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
CREATE OR REPLACE VIEW sequent_backend.election_lifecycle_access AS
SELECT l.tenant_id,l.election_event_id,l.phase,l.safety,l.generation,
 (l.phase='execution' AND l.safety='operational'
  AND sequent_backend.audit_storage_allows_voters()
  AND pg_try_advisory_xact_lock_shared(hashtextextended(l.tenant_id::text||':'||l.election_event_id::text,848527))
  AND (SELECT count(*) FROM sequent_backend.lifecycle_service_lease WHERE component IN ('harvest','windmill') AND expires_at>clock_timestamp())=2
  AND NOT EXISTS (SELECT 1 FROM sequent_backend.self_test_run f WHERE f.tenant_id=l.tenant_id AND f.election_event_id=l.election_event_id AND f.outcome='unrecoverable-failure')
  AND COALESCE((SELECT r.outcome='passed' AND r.finished_at BETWEEN clock_timestamp()-make_interval(mins=>COALESCE((s.settings->>'interval_minutes')::int,15)) AND clock_timestamp()
   FROM sequent_backend.self_test_run r LEFT JOIN sequent_backend.self_test_settings s USING(tenant_id,election_event_id)
   WHERE r.tenant_id=l.tenant_id AND r.election_event_id=l.election_event_id AND r.generation=l.generation AND r.finished_at IS NOT NULL
   ORDER BY r.finished_at DESC,r.id DESC LIMIT 1),false)) AS voter_allowed
FROM sequent_backend.election_lifecycle l;

DROP FUNCTION sequent_backend.attack_activity_allows_voters();
DROP FUNCTION IF EXISTS sequent_backend.record_attack_auth_activity(uuid,uuid,uuid,text,text,timestamptz,integer,integer);
DROP FUNCTION IF EXISTS sequent_backend.compare_attack_request_activity();
DROP TABLE IF EXISTS sequent_backend.attack_detection_audit;
DROP TABLE IF EXISTS sequent_backend.attack_detection;
DROP TABLE IF EXISTS sequent_backend.attack_activity_health;
DROP TABLE IF EXISTS sequent_backend.attack_request_bucket;
DROP TABLE IF EXISTS sequent_backend.attack_auth_activity;
DROP TABLE IF EXISTS sequent_backend.attack_detection_settings;
