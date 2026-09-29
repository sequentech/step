-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Configurable monitoring dashboards. Internal to Windmill and Harvest: no
-- Hasura metadata tracks these tables, so the Admin Portal reaches them only
-- through Harvest's permission-checked routes. Every row belongs to one
-- election event of its own tenant and goes when the event goes.

-- An event without a row keeps the standard dashboard.
CREATE TABLE sequent_backend.monitoring_event (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    dashboard_mode text NOT NULL DEFAULT 'LEGACY'
        CHECK (dashboard_mode IN ('LEGACY', 'CONFIGURED')),
    -- The preset the configuration was last reset to.
    preset_id text CHECK (preset_id ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    preset_version integer CHECK (preset_version > 0),
    -- Raised by every saved change, so a snapshot says which configuration
    -- it was counted for.
    config_generation bigint NOT NULL DEFAULT 0 CHECK (config_generation >= 0),
    -- The snapshot run viewers are shown; none until the first completes.
    live_snapshot_revision bigint,
    -- Where each incremental pass of the snapshot job stopped.
    watermarks jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(watermarks) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id),
    CHECK ((preset_id IS NULL) = (preset_version IS NULL)),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- Every saved version of every document, never updated: a DELETE revision
-- records that a document was removed.
CREATE TABLE sequent_backend.monitoring_config (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('dashboard', 'widget', 'theme', 'settings')),
    key text NOT NULL CHECK (key ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    revision integer NOT NULL CHECK (revision > 0),
    change text NOT NULL CHECK (change IN ('UPSERT', 'DELETE')),
    yaml text,
    -- Lowercase hex SHA-256 of `yaml`, as the electoral log records it.
    sha256 text CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    origin text NOT NULL CHECK (origin IN ('PRESET', 'EDITOR')),
    preset_id text CHECK (preset_id ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    preset_version integer CHECK (preset_version > 0),
    author_id text,
    author_name text,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, kind, key, revision),
    CHECK ((change = 'UPSERT') = (yaml IS NOT NULL)),
    CHECK ((yaml IS NULL) = (sha256 IS NULL)),
    CHECK ((preset_id IS NULL) = (preset_version IS NULL)),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- The live revision of each document. A save moves it with
-- `UPDATE ... WHERE revision = <the revision the editor started from>`;
-- touching no row means someone else saved first.
CREATE TABLE sequent_backend.monitoring_config_head (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    kind text NOT NULL,
    key text NOT NULL,
    revision integer NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, kind, key),
    FOREIGN KEY (tenant_id, election_event_id, kind, key, revision)
        REFERENCES sequent_backend.monitoring_config
            (tenant_id, election_event_id, kind, key, revision)
        ON DELETE CASCADE
);

-- One pass of the snapshot job.
CREATE TABLE sequent_backend.monitoring_snapshot_run (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    status text NOT NULL CHECK (status IN ('RUNNING', 'COMPLETE', 'FAILED')),
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    -- The moment the figures are from.
    as_of timestamptz,
    -- Last time a later pass found nothing had changed since `as_of`.
    checked_at timestamptz,
    settings_revision integer CHECK (settings_revision > 0),
    config_generation bigint CHECK (config_generation >= 0),
    error text,
    PRIMARY KEY (tenant_id, election_event_id, revision),
    CHECK (status <> 'COMPLETE' OR (as_of IS NOT NULL AND finished_at IS NOT NULL)),
    CHECK (status <> 'FAILED' OR (error IS NOT NULL AND finished_at IS NOT NULL)),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- Pruning cannot remove the run viewers are shown.
ALTER TABLE sequent_backend.monitoring_event
    ADD FOREIGN KEY (tenant_id, election_event_id, live_snapshot_revision)
        REFERENCES sequent_backend.monitoring_snapshot_run
            (tenant_id, election_event_id, revision);

-- What a run produced for each source and set of visible elections: a
-- payload hash per scope (the event, a region, a Post, a country, ...).
CREATE TABLE sequent_backend.monitoring_snapshot_manifest (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    revision bigint NOT NULL,
    source text NOT NULL CHECK (source IN (
        'voter_turnout', 'test_voting', 'enrollment_decisions',
        'voting_credentials', 'poll_status', 'final_testing_lockdown',
        'counting_transmission', 'voting_enrollment_activity',
        'access_security', 'attack_detections', 'helpdesk'
    )),
    -- The first 16 hex digits of the SHA-256 of the sorted election ids.
    election_set_key text NOT NULL CHECK (election_set_key ~ '^[0-9a-f]{16}$'),
    producer_status text NOT NULL
        CHECK (producer_status IN ('CONNECTED', 'NOT_CONNECTED')),
    reason text,
    scopes jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(scopes) = 'object'),
    PRIMARY KEY (tenant_id, election_event_id, revision, source, election_set_key),
    CHECK ((producer_status = 'NOT_CONNECTED') = (reason IS NOT NULL AND reason <> '')),
    FOREIGN KEY (tenant_id, election_event_id, revision)
        REFERENCES sequent_backend.monitoring_snapshot_run
            (tenant_id, election_event_id, revision)
        ON DELETE CASCADE
);

-- Figures for one scope, stored once by content so a scope that did not
-- change costs a hash lookup rather than a new row.
CREATE TABLE sequent_backend.monitoring_snapshot_payload (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    payload jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, sha256),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- One row per voter and election (Post), kept up to date by the snapshot
-- job. `dims` holds derived values only (an age band, never a birth date).
CREATE TABLE sequent_backend.monitoring_voter (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    voter_id text NOT NULL CHECK (voter_id <> ''),
    area_id uuid,
    region text,
    country text,
    dims jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(dims) = 'object'),
    enrollment_state text
        CHECK (enrollment_state IN ('PENDING', 'ACCEPTED', 'REJECTED')),
    enrollment_reason text,
    pre_enrolled_at timestamptz,
    credentials_at timestamptz,
    test_voted_at timestamptz,
    first_voted_at timestamptz,
    -- Hash of the attributes the row was derived from, so an unchanged
    -- voter is not rewritten.
    attributes_hash text NOT NULL,
    settings_revision integer NOT NULL CHECK (settings_revision > 0),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, election_id, voter_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE,
    FOREIGN KEY (election_id, tenant_id, election_event_id)
        REFERENCES sequent_backend.election (id, tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- Sign-in attempts per 15 minutes of UTC, so hourly buckets rebuild
-- exactly in any time zone, including those a quarter or half hour off.
-- Counted from the electoral log whether or not the event is configured
-- yet, so a dashboard switched on mid-election still has the history.
-- `area_id` is as recorded: an area deleted later counts as Unknown.
CREATE TABLE sequent_backend.monitoring_login_counter (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    bucket_start timestamptz NOT NULL
        CHECK (extract(epoch FROM bucket_start) % 900 = 0),
    event_type text NOT NULL CHECK (event_type ~ '^[A-Z][A-Z0-9_]{0,63}$'),
    registration text NOT NULL CHECK (registration IN ('REGISTERED', 'UNREGISTERED')),
    area_id uuid,
    attempts bigint NOT NULL CHECK (attempts > 0),
    UNIQUE NULLS NOT DISTINCT
        (tenant_id, election_event_id, bucket_start, event_type, registration, area_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- Deliveries already counted, so a redelivered log message counts once.
-- Pruned after a week, well past the log's redelivery window.
CREATE TABLE sequent_backend.monitoring_login_counter_receipt (
    delivery_id text PRIMARY KEY CHECK (delivery_id <> ''),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

CREATE INDEX monitoring_login_counter_receipt_received_at
    ON sequent_backend.monitoring_login_counter_receipt (received_at);

-- updated_at follows every change, as on the other backend tables.
CREATE TRIGGER set_sequent_backend_monitoring_event_updated_at
    BEFORE UPDATE ON sequent_backend.monitoring_event
    FOR EACH ROW EXECUTE PROCEDURE sequent_backend.set_current_timestamp_updated_at();
CREATE TRIGGER set_sequent_backend_monitoring_config_head_updated_at
    BEFORE UPDATE ON sequent_backend.monitoring_config_head
    FOR EACH ROW EXECUTE PROCEDURE sequent_backend.set_current_timestamp_updated_at();
CREATE TRIGGER set_sequent_backend_monitoring_voter_updated_at
    BEFORE UPDATE ON sequent_backend.monitoring_voter
    FOR EACH ROW EXECUTE PROCEDURE sequent_backend.set_current_timestamp_updated_at();
