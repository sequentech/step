-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Configurable monitoring dashboards. Internal to Windmill and Harvest: no
-- Hasura metadata tracks these tables, so the Admin Portal reaches them only
-- through Harvest's permission-checked routes. Every row belongs to one
-- election event of its own tenant and goes when the event goes.
--
-- The CHECK lists of configuration kinds and data sources mirror
-- sequent_core::monitoring::{ConfigKind, DataSourceId}; a new variant needs a
-- migration deployed before the code that writes it.

-- An event without a row keeps the standard dashboard. Going back to it is an
-- UPDATE to LEGACY, never a DELETE, which would take the configuration
-- history with it.
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
    -- Lets the foreign key below require the shown run to be complete.
    live_snapshot_status text GENERATED ALWAYS AS (
        CASE WHEN live_snapshot_revision IS NOT NULL THEN 'COMPLETE' END
    ) STORED,
    -- Where each incremental pass of the snapshot job stopped.
    watermarks jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(watermarks) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    -- Any change to the row, the snapshot job's included.
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id),
    CONSTRAINT monitoring_event_preset_named_with_version
        CHECK ((preset_id IS NULL) = (preset_version IS NULL)),
    CONSTRAINT monitoring_event_of_its_tenant
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- Every saved version of every document. Rows are never changed or removed
-- except with their event: a DELETE revision records a removed document.
CREATE TABLE sequent_backend.monitoring_config (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('dashboard', 'widget', 'theme', 'settings')),
    key text NOT NULL CHECK (key ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    revision integer NOT NULL CHECK (revision > 0),
    change text NOT NULL CHECK (change IN ('UPSERT', 'DELETE')),
    yaml text CHECK (yaml <> ''),
    -- Lowercase hex SHA-256 of the UTF-8 `yaml`, as the electoral log
    -- records it.
    sha256 text,
    origin text NOT NULL CHECK (origin IN ('PRESET', 'EDITOR')),
    preset_id text CHECK (preset_id ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    preset_version integer CHECK (preset_version > 0),
    author_id text,
    author_name text,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, kind, key, revision),
    CONSTRAINT monitoring_config_document_follows_change
        CHECK ((change = 'UPSERT') = (yaml IS NOT NULL)),
    CONSTRAINT monitoring_config_digest_is_of_document
        CHECK (
            CASE WHEN yaml IS NULL THEN sha256 IS NULL
            ELSE sha256 IS NOT DISTINCT FROM encode(sha256(convert_to(yaml, 'UTF8')), 'hex')
            END
        ),
    CONSTRAINT monitoring_config_preset_named_with_version
        CHECK ((preset_id IS NULL) = (preset_version IS NULL)),
    CONSTRAINT monitoring_config_preset_follows_origin
        CHECK ((origin = 'PRESET') = (preset_id IS NOT NULL)),
    CONSTRAINT monitoring_config_editor_is_named
        CHECK (origin <> 'EDITOR' OR COALESCE(author_id, '') <> ''),
    CONSTRAINT monitoring_config_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

CREATE FUNCTION sequent_backend.monitoring_config_is_append_only()
RETURNS trigger AS $$
BEGIN
    -- A cascade from the event runs inside the foreign key's own trigger.
    IF TG_OP = 'DELETE' AND pg_trigger_depth() > 1 THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'monitoring configuration revisions are append-only'
        USING ERRCODE = 'integrity_constraint_violation',
              CONSTRAINT = 'monitoring_config_is_append_only';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_config_is_append_only
    BEFORE UPDATE OR DELETE ON sequent_backend.monitoring_config
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_config_is_append_only();

-- The live revision of each document, a DELETE revision for a removed one.
-- A save moves it before writing the revision it names, which the deferred
-- foreign key allows:
--   UPDATE ... SET revision = <expected + 1> WHERE ... AND revision = <expected>
-- or, for a new document, INSERT ... (revision 1) ON CONFLICT DO NOTHING.
-- A concurrent save waits on the row and then matches nothing: touching no
-- row is the conflict to report, never an error.
CREATE TABLE sequent_backend.monitoring_config_head (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    kind text NOT NULL,
    key text NOT NULL,
    revision integer NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, kind, key),
    CONSTRAINT monitoring_config_head_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE,
    CONSTRAINT monitoring_config_head_names_a_revision
        FOREIGN KEY (tenant_id, election_event_id, kind, key, revision)
        REFERENCES sequent_backend.monitoring_config
            (tenant_id, election_event_id, kind, key, revision)
        DEFERRABLE INITIALLY DEFERRED
);

-- True when the ids are in strictly ascending order.
CREATE FUNCTION sequent_backend.monitoring_uuids_ascend(ids uuid[])
RETURNS boolean AS $$
    SELECT COALESCE(bool_and(ids[i] < ids[i + 1]), true)
    FROM generate_subscripts(ids, 1) AS i
    WHERE i < array_upper(ids, 1)
$$ LANGUAGE sql IMMUTABLE STRICT;

-- The sets of elections viewers may see, as their permission labels allow.
-- Harvest records a set a viewer asks for; the snapshot job counts for every
-- recorded set.
CREATE TABLE sequent_backend.monitoring_election_set (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    -- The first 16 hex digits of the SHA-256 of the ids, in ascending
    -- order, lowercase and comma-separated.
    election_set_key text NOT NULL,
    election_ids uuid[] NOT NULL,
    requested_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, election_set_key),
    CONSTRAINT monitoring_election_set_ids_ascend
        CHECK (
            array_ndims(election_ids) IS NOT DISTINCT FROM 1
            AND array_position(election_ids, NULL) IS NULL
            AND sequent_backend.monitoring_uuids_ascend(election_ids)
        ),
    CONSTRAINT monitoring_election_set_key_is_of_ids
        CHECK (
            election_set_key = left(
                encode(sha256(convert_to(array_to_string(election_ids, ','), 'UTF8')), 'hex'),
                16
            )
        ),
    CONSTRAINT monitoring_election_set_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- Snapshot revisions only grow, even across events, so a revision a viewer
-- saw is never reused for other figures.
CREATE SEQUENCE sequent_backend.monitoring_snapshot_revision;

-- One pass of the snapshot job.
CREATE TABLE sequent_backend.monitoring_snapshot_run (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    revision bigint NOT NULL
        DEFAULT nextval('sequent_backend.monitoring_snapshot_revision')
        CHECK (revision > 0),
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
    CONSTRAINT monitoring_snapshot_run_status_key
        UNIQUE (tenant_id, election_event_id, revision, status),
    CONSTRAINT monitoring_snapshot_run_running_is_open
        CHECK (status <> 'RUNNING' OR (finished_at IS NULL AND error IS NULL)),
    CONSTRAINT monitoring_snapshot_run_complete_is_counted
        CHECK (
            status <> 'COMPLETE'
            OR (
                as_of IS NOT NULL AND finished_at IS NOT NULL AND error IS NULL
                AND settings_revision IS NOT NULL AND config_generation IS NOT NULL
            )
        ),
    CONSTRAINT monitoring_snapshot_run_failure_says_why
        CHECK (status <> 'FAILED' OR (COALESCE(error, '') <> '' AND finished_at IS NOT NULL)),
    CONSTRAINT monitoring_snapshot_run_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- Viewers are only shown a complete run, and that run can be neither pruned
-- nor changed while it is shown.
ALTER TABLE sequent_backend.monitoring_event
    ADD CONSTRAINT monitoring_event_shows_a_complete_run
        FOREIGN KEY (tenant_id, election_event_id, live_snapshot_revision, live_snapshot_status)
        REFERENCES sequent_backend.monitoring_snapshot_run
            (tenant_id, election_event_id, revision, status);

-- Figures for one scope, stored once by content so an unchanged scope costs
-- a hash lookup rather than a new row.
CREATE TABLE sequent_backend.monitoring_snapshot_payload (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    payload jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, sha256),
    CONSTRAINT monitoring_snapshot_payload_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- What one source produced for one set of elections, stored once by content
-- (the hash covers the source, status, reason and every scope's payload
-- hash), so a source that did not change costs no new rows. Immutable, so
-- Harvest may cache it by hash.
CREATE TABLE sequent_backend.monitoring_snapshot_manifest (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    source text NOT NULL CHECK (source IN (
        'voter_turnout', 'test_voting', 'enrollment_decisions',
        'voting_credentials', 'poll_status', 'final_testing_lockdown',
        'counting_transmission', 'voting_enrollment_activity',
        'access_security', 'attack_detections', 'helpdesk'
    )),
    producer_status text NOT NULL
        CHECK (producer_status IN ('CONNECTED', 'NOT_CONNECTED')),
    reason text,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, sha256),
    CONSTRAINT monitoring_snapshot_manifest_source_key
        UNIQUE (tenant_id, election_event_id, sha256, source),
    CONSTRAINT monitoring_snapshot_manifest_not_connected_says_why
        CHECK ((producer_status = 'NOT_CONNECTED') = (COALESCE(reason, '') <> '')),
    CONSTRAINT monitoring_snapshot_manifest_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

-- The snapshot tables' references to one another are checked at commit, so
-- an event's cascade may remove both ends in either order. Pruning must
-- still remove runs before the manifests, sets and payloads they name, or
-- its commit fails.

-- A manifest's payload for each scope: `''` is the whole event, otherwise
-- the canonical ScopeKey (`region=…&post=…&country=…`).
CREATE TABLE sequent_backend.monitoring_snapshot_manifest_scope (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    manifest_sha256 text NOT NULL,
    scope_key text NOT NULL,
    payload_sha256 text NOT NULL,
    PRIMARY KEY (tenant_id, election_event_id, manifest_sha256, scope_key),
    CONSTRAINT monitoring_snapshot_manifest_scope_of_its_manifest
        FOREIGN KEY (tenant_id, election_event_id, manifest_sha256)
        REFERENCES sequent_backend.monitoring_snapshot_manifest
            (tenant_id, election_event_id, sha256)
        ON DELETE CASCADE,
    CONSTRAINT monitoring_snapshot_manifest_scope_payload_exists
        FOREIGN KEY (tenant_id, election_event_id, payload_sha256)
        REFERENCES sequent_backend.monitoring_snapshot_payload
            (tenant_id, election_event_id, sha256)
        DEFERRABLE INITIALLY DEFERRED
);

-- Pruning keeps every payload a manifest still names.
CREATE INDEX monitoring_snapshot_manifest_scope_payload
    ON sequent_backend.monitoring_snapshot_manifest_scope
        (tenant_id, election_event_id, payload_sha256);

-- A run's manifest for each source and set of elections.
CREATE TABLE sequent_backend.monitoring_snapshot_source (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    revision bigint NOT NULL,
    source text NOT NULL,
    election_set_key text NOT NULL,
    manifest_sha256 text NOT NULL,
    PRIMARY KEY (tenant_id, election_event_id, revision, source, election_set_key),
    CONSTRAINT monitoring_snapshot_source_of_its_run
        FOREIGN KEY (tenant_id, election_event_id, revision)
        REFERENCES sequent_backend.monitoring_snapshot_run
            (tenant_id, election_event_id, revision)
        ON DELETE CASCADE,
    CONSTRAINT monitoring_snapshot_source_manifest_of_the_source
        FOREIGN KEY (tenant_id, election_event_id, manifest_sha256, source)
        REFERENCES sequent_backend.monitoring_snapshot_manifest
            (tenant_id, election_event_id, sha256, source)
        DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT monitoring_snapshot_source_set_is_recorded
        FOREIGN KEY (tenant_id, election_event_id, election_set_key)
        REFERENCES sequent_backend.monitoring_election_set
            (tenant_id, election_event_id, election_set_key)
        DEFERRABLE INITIALLY DEFERRED
);

-- Pruning keeps every manifest and election set a run still names.
CREATE INDEX monitoring_snapshot_source_manifest
    ON sequent_backend.monitoring_snapshot_source
        (tenant_id, election_event_id, manifest_sha256);
CREATE INDEX monitoring_snapshot_source_election_set
    ON sequent_backend.monitoring_snapshot_source
        (tenant_id, election_event_id, election_set_key);

-- One row per voter and election (Post), kept up to date by the snapshot
-- job. `dims` holds derived values only (an age band, never a birth date).
-- A projection, so `area_id` is as last read and not a foreign key; a
-- missing region or country is NULL, which the dashboards show as Unknown.
CREATE TABLE sequent_backend.monitoring_voter (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    voter_id text NOT NULL CHECK (voter_id <> ''),
    area_id uuid,
    region text CHECK (region <> ''),
    country text CHECK (country <> ''),
    dims jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(dims) = 'object'),
    enrollment_state text
        CHECK (enrollment_state IN ('PENDING', 'ACCEPTED', 'REJECTED')),
    enrollment_reason text CHECK (enrollment_reason <> ''),
    pre_enrolled_at timestamptz,
    credentials_at timestamptz,
    test_voted_at timestamptz,
    first_voted_at timestamptz,
    -- Hash of the attributes the row was derived from, so an unchanged
    -- voter is not rewritten.
    attributes_hash text NOT NULL CHECK (attributes_hash <> ''),
    settings_revision integer NOT NULL CHECK (settings_revision > 0),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, election_id, voter_id),
    CONSTRAINT monitoring_voter_reason_is_for_rejection
        CHECK (enrollment_reason IS NULL OR enrollment_state IS NOT DISTINCT FROM 'REJECTED'),
    CONSTRAINT monitoring_voter_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE,
    CONSTRAINT monitoring_voter_of_an_election_of_the_event
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
        CHECK (date_bin('15 minutes', bucket_start, '2000-01-01T00:00:00Z') = bucket_start),
    event_type text NOT NULL CHECK (event_type ~ '^[A-Z][A-Z0-9_]{0,63}$'),
    registration text NOT NULL CHECK (registration IN ('REGISTERED', 'UNREGISTERED')),
    area_id uuid,
    -- The key's stand-in for a missing area, so the natural key can be the
    -- primary key.
    area_key uuid GENERATED ALWAYS AS (
        COALESCE(area_id, '00000000-0000-0000-0000-000000000000')
    ) STORED,
    attempts bigint NOT NULL CHECK (attempts > 0),
    PRIMARY KEY (tenant_id, election_event_id, bucket_start, event_type, registration, area_key),
    CONSTRAINT monitoring_login_counter_of_its_tenant
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- Deliveries already counted, so a redelivered log message counts once.
-- The job prunes receipts after the electoral log's redelivery window; a
-- message redelivered later than that would count again.
CREATE TABLE sequent_backend.monitoring_login_counter_receipt (
    -- The electoral log's delivery id, a SHA-256 in lowercase hex.
    delivery_id text PRIMARY KEY CHECK (delivery_id ~ '^[0-9a-f]{64}$'),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT monitoring_login_counter_receipt_of_its_tenant
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

CREATE INDEX monitoring_login_counter_receipt_received_at
    ON sequent_backend.monitoring_login_counter_receipt (received_at);
CREATE INDEX monitoring_login_counter_receipt_event
    ON sequent_backend.monitoring_login_counter_receipt (tenant_id, election_event_id);

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
