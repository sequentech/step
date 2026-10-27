-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
CREATE TABLE sequent_backend.self_test_settings (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    settings jsonb NOT NULL,
    PRIMARY KEY (tenant_id,election_event_id),
    FOREIGN KEY (tenant_id,election_event_id) REFERENCES sequent_backend.election_lifecycle(tenant_id,election_event_id) ON DELETE CASCADE
);
CREATE TABLE sequent_backend.self_test_baseline (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    publication_id uuid NOT NULL,
    snapshot jsonb NOT NULL,
    signature jsonb NOT NULL,
    digest text NOT NULL CHECK (digest ~ '^[0-9a-f]{64}$'),
    approved_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id,election_event_id),
    FOREIGN KEY (tenant_id,election_event_id) REFERENCES sequent_backend.election_lifecycle(tenant_id,election_event_id) ON DELETE CASCADE
);
CREATE TABLE sequent_backend.self_test_run (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    generation bigint NOT NULL,
    trigger text NOT NULL,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    outcome text CHECK (outcome IN ('passed','recoverable-failure','unrecoverable-failure')),
    result jsonb NOT NULL,
    FOREIGN KEY (tenant_id,election_event_id) REFERENCES sequent_backend.election_lifecycle(tenant_id,election_event_id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX self_test_one_pending ON sequent_backend.self_test_run(tenant_id,election_event_id) WHERE finished_at IS NULL;
CREATE INDEX self_test_event_history ON sequent_backend.self_test_run(tenant_id,election_event_id,started_at DESC);
-- Deliberately untracked by Hasura: only the authenticated TSF actions expose these records.

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
