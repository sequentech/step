-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Configurable monitoring dashboards. Internal to Windmill and Harvest: no
-- Hasura metadata tracks these tables, so the Admin Portal reaches them only
-- through Harvest's permission-checked routes. Every row belongs to one
-- election event of its own tenant and goes when the event goes.
--
-- The lists of configuration kinds and data sources mirror
-- sequent_core::monitoring::{ConfigKind, DataSourceId}; a new variant needs a
-- migration deployed before the code that writes it.

-- Raises for the table and operation that fired it; attached to what must
-- not change.
CREATE FUNCTION sequent_backend.monitoring_refuse_change()
RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% of % is not allowed', TG_OP, TG_TABLE_NAME
        USING ERRCODE = 'integrity_constraint_violation',
              CONSTRAINT = TG_ARGV[0];
END;
$$ LANGUAGE plpgsql;

-- An event without a row keeps the standard dashboard. Going back to it is an
-- UPDATE to LEGACY: the row, and the configuration history with it, is only
-- deleted with its election event.
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

CREATE FUNCTION sequent_backend.monitoring_event_is_kept()
RETURNS trigger AS $$
BEGIN
    -- Inside the election event's cascade the event is already gone.
    IF EXISTS (
        SELECT 1 FROM sequent_backend.election_event
        WHERE tenant_id = OLD.tenant_id AND id = OLD.election_event_id
    ) THEN
        RAISE EXCEPTION 'monitoring rows are deleted with their election event'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_event_is_kept';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_event_is_kept
    BEFORE DELETE ON sequent_backend.monitoring_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_event_is_kept();

-- Every saved version of every document. Rows are never changed, and are
-- deleted only with their event: a DELETE revision records a removed
-- document.
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
    -- Who saved it or reset to the preset.
    author_id text NOT NULL,
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
    CONSTRAINT monitoring_config_author_is_named CHECK (author_id <> ''),
    CONSTRAINT monitoring_config_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

CREATE FUNCTION sequent_backend.monitoring_config_is_append_only()
RETURNS trigger AS $$
BEGIN
    -- Inside the event's cascade the event row is already gone; any other
    -- deletion, from a statement or another trigger, still sees it.
    IF TG_OP = 'DELETE' AND NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_event
        WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
    ) THEN
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
CREATE TRIGGER monitoring_config_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_config
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_config_is_append_only');

-- The live revision of each document, a DELETE revision for a removed one.
-- A save, under READ COMMITTED (a stricter isolation level fails the later
-- save with a serialization error instead):
--   1. UPDATE monitoring_event SET config_generation = config_generation + 1,
--      which also queues saves and resets of one event behind each other;
--   2. UPDATE the head SET revision = <expected + 1>
--      WHERE ... AND revision = <expected>, or for a new document
--      INSERT the head (revision 1) ON CONFLICT DO NOTHING;
--   3. INSERT the revision the head now names.
-- Touching no head row in step 2 is the conflict to report, never an error.
-- The head's foreign key is checked at commit, so step 2 may come first, and
-- a revision the head does not reach by then is refused.
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

CREATE FUNCTION sequent_backend.monitoring_config_revision_is_the_head()
RETURNS trigger AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_config
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND kind = NEW.kind AND key = NEW.key AND revision = NEW.revision
    ) AND NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_config_head
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND kind = NEW.kind AND key = NEW.key AND revision >= NEW.revision
    ) THEN
        RAISE EXCEPTION 'revision % of % % is not the head', NEW.revision, NEW.kind, NEW.key
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_config_revision_is_the_head';
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- A revision is written only as the new head, so a later save never finds
-- its next revision number taken.
CREATE CONSTRAINT TRIGGER monitoring_config_revision_is_the_head
    AFTER INSERT ON sequent_backend.monitoring_config
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_config_revision_is_the_head();

-- True when the ids are in strictly ascending order.
CREATE FUNCTION sequent_backend.monitoring_uuids_ascend(ids uuid[])
RETURNS boolean AS $$
    SELECT COALESCE(bool_and(ids[i] < ids[i + 1]), true)
    FROM generate_subscripts(ids, 1) AS i
    WHERE i < array_upper(ids, 1)
$$ LANGUAGE sql IMMUTABLE STRICT;

-- The sets of elections viewers may see, as their permission labels allow;
-- the empty set for a viewer allowed none. Harvest records a set a viewer
-- asks for, refreshing `requested_at` at most every few minutes rather than
-- on each render; the snapshot job counts for every recorded set, and stops
-- when a set is no longer asked for. An election deleted later stays listed
-- until then.
CREATE TABLE sequent_backend.monitoring_election_set (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    -- sequent_core::monitoring::scope::election_set_key: the first 16 hex
    -- digits of the SHA-256 of the ids, ascending, lowercase and
    -- comma-separated.
    election_set_key text NOT NULL,
    election_ids uuid[] NOT NULL,
    requested_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, election_set_key),
    CONSTRAINT monitoring_election_set_ids_ascend
        CHECK (
            (cardinality(election_ids) = 0 OR array_ndims(election_ids) = 1)
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

-- One pass of the snapshot job, recorded as RUNNING in a transaction of its
-- own so a failed pass can still be marked FAILED. Once complete or failed a
-- run is final, but for `checked_at`.
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

CREATE FUNCTION sequent_backend.monitoring_snapshot_run_is_final()
RETURNS trigger AS $$
BEGIN
    IF OLD.status <> 'RUNNING' AND (
        NEW.tenant_id, NEW.election_event_id, NEW.revision, NEW.status, NEW.started_at,
        NEW.finished_at, NEW.as_of, NEW.settings_revision, NEW.config_generation, NEW.error
    ) IS DISTINCT FROM (
        OLD.tenant_id, OLD.election_event_id, OLD.revision, OLD.status, OLD.started_at,
        OLD.finished_at, OLD.as_of, OLD.settings_revision, OLD.config_generation, OLD.error
    ) THEN
        RAISE EXCEPTION 'snapshot run % is final', OLD.revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_run_is_final';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_snapshot_run_is_final
    BEFORE UPDATE ON sequent_backend.monitoring_snapshot_run
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_run_is_final();

-- Viewers are only shown a complete run, which cannot be pruned while shown.
ALTER TABLE sequent_backend.monitoring_event
    ADD CONSTRAINT monitoring_event_shows_a_complete_run
        FOREIGN KEY (tenant_id, election_event_id, live_snapshot_revision, live_snapshot_status)
        REFERENCES sequent_backend.monitoring_snapshot_run
            (tenant_id, election_event_id, revision, status);

-- The snapshot tables' references to one another are checked at commit, so
-- an event's cascade may remove both ends in either order. The snapshot job
-- prunes in a transaction of its own that starts with
-- SET CONSTRAINTS ALL IMMEDIATE, so a pruning mistake fails at the statement
-- that made it and leaves the snapshot it wrote alone.

-- Whether each source was counted in a run, for each set of elections.
CREATE TABLE sequent_backend.monitoring_snapshot_source (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    revision bigint NOT NULL,
    source text NOT NULL CHECK (source IN (
        'voter_turnout', 'test_voting', 'enrollment_decisions',
        'voting_credentials', 'poll_status', 'final_testing_lockdown',
        'counting_transmission', 'voting_enrollment_activity',
        'access_security', 'attack_detections', 'helpdesk'
    )),
    election_set_key text NOT NULL,
    producer_status text NOT NULL
        CHECK (producer_status IN ('CONNECTED', 'NOT_CONNECTED')),
    reason text,
    PRIMARY KEY (tenant_id, election_event_id, revision, source, election_set_key),
    CONSTRAINT monitoring_snapshot_source_not_connected_says_why
        CHECK ((producer_status = 'NOT_CONNECTED') = (COALESCE(reason, '') <> '')),
    CONSTRAINT monitoring_snapshot_source_of_its_run
        FOREIGN KEY (tenant_id, election_event_id, revision)
        REFERENCES sequent_backend.monitoring_snapshot_run
            (tenant_id, election_event_id, revision)
        ON DELETE CASCADE,
    CONSTRAINT monitoring_snapshot_source_set_is_recorded
        FOREIGN KEY (tenant_id, election_event_id, election_set_key)
        REFERENCES sequent_backend.monitoring_election_set
            (tenant_id, election_event_id, election_set_key)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX monitoring_snapshot_source_election_set
    ON sequent_backend.monitoring_snapshot_source
        (tenant_id, election_event_id, election_set_key);
CREATE TRIGGER monitoring_snapshot_source_is_immutable
    BEFORE UPDATE ON sequent_backend.monitoring_snapshot_source
    FOR EACH ROW
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_snapshot_source_is_immutable');

-- Figures for one scope, stored once by the SHA-256 of their content, so
-- Harvest may cache them by hash and an unchanged scope stores nothing new.
CREATE TABLE sequent_backend.monitoring_snapshot_payload (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    -- SHA-256, as bytes.
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, sha256),
    CONSTRAINT monitoring_snapshot_payload_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE
);

CREATE TRIGGER monitoring_snapshot_payload_is_immutable
    BEFORE UPDATE ON sequent_backend.monitoring_snapshot_payload
    FOR EACH ROW
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_snapshot_payload_is_immutable');

-- Which payload a scope showed, over the runs from `from_revision` up to but
-- not including `to_revision` (still showing when NULL). A pass writes only
-- the scopes that changed: it closes the old row at its revision and opens a
-- new one. The figures of revision R are the rows whose range holds R, one
-- index probe per scope; pruning to the oldest kept revision K deletes the
-- rows closed at or before K, then the payloads no row names any more.
CREATE TABLE sequent_backend.monitoring_snapshot_figure (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    source text NOT NULL CHECK (source IN (
        'voter_turnout', 'test_voting', 'enrollment_decisions',
        'voting_credentials', 'poll_status', 'final_testing_lockdown',
        'counting_transmission', 'voting_enrollment_activity',
        'access_security', 'attack_detections', 'helpdesk'
    )),
    election_set_key text NOT NULL,
    -- `event`, or the canonical ScopeKey: `region=…`, `post=…` and
    -- `country=…` in that order, joined by `&`, values percent-encoded.
    scope_key text NOT NULL CHECK (scope_key ~ (
        '^(event'
        || '|region=[A-Za-z0-9._~%-]+(&post=[A-Za-z0-9._~%-]+)?(&country=[A-Za-z0-9._~%-]+)?'
        || '|post=[A-Za-z0-9._~%-]+(&country=[A-Za-z0-9._~%-]+)?'
        || '|country=[A-Za-z0-9._~%-]+)$'
    )),
    from_revision bigint NOT NULL CHECK (from_revision > 0),
    to_revision bigint,
    payload_sha256 bytea NOT NULL CHECK (octet_length(payload_sha256) = 32),
    PRIMARY KEY (tenant_id, election_event_id, source, election_set_key, scope_key, from_revision),
    CONSTRAINT monitoring_snapshot_figure_range_is_forward
        CHECK (to_revision > from_revision),
    CONSTRAINT monitoring_snapshot_figure_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE,
    CONSTRAINT monitoring_snapshot_figure_set_is_recorded
        FOREIGN KEY (tenant_id, election_event_id, election_set_key)
        REFERENCES sequent_backend.monitoring_election_set
            (tenant_id, election_event_id, election_set_key)
        DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT monitoring_snapshot_figure_payload_exists
        FOREIGN KEY (tenant_id, election_event_id, payload_sha256)
        REFERENCES sequent_backend.monitoring_snapshot_payload
            (tenant_id, election_event_id, sha256)
        DEFERRABLE INITIALLY DEFERRED
);

-- One showing row per scope.
CREATE UNIQUE INDEX monitoring_snapshot_figure_one_open
    ON sequent_backend.monitoring_snapshot_figure
        (tenant_id, election_event_id, source, election_set_key, scope_key)
    WHERE to_revision IS NULL;
-- Pruning by revision, and payloads still named.
CREATE INDEX monitoring_snapshot_figure_closed
    ON sequent_backend.monitoring_snapshot_figure (tenant_id, election_event_id, to_revision)
    WHERE to_revision IS NOT NULL;
CREATE INDEX monitoring_snapshot_figure_payload
    ON sequent_backend.monitoring_snapshot_figure (tenant_id, election_event_id, payload_sha256);
CREATE INDEX monitoring_snapshot_figure_election_set
    ON sequent_backend.monitoring_snapshot_figure (tenant_id, election_event_id, election_set_key);

-- A figure row is only ever closed, and a scope's ranges never overlap: the
-- row before a new one ends where it starts, and the row after it starts
-- where it ends. Checking both neighbours keeps the whole history disjoint,
-- since every row goes through this check.
CREATE FUNCTION sequent_backend.monitoring_snapshot_figure_is_disjoint()
RETURNS trigger AS $$
DECLARE
    previous_end bigint;
    previous_open boolean;
    next_start bigint;
BEGIN
    IF TG_OP = 'UPDATE' AND NOT (
        OLD.to_revision IS NULL AND NEW.to_revision IS NOT NULL
        AND (NEW.tenant_id, NEW.election_event_id, NEW.source, NEW.election_set_key,
             NEW.scope_key, NEW.from_revision, NEW.payload_sha256)
            IS NOT DISTINCT FROM
            (OLD.tenant_id, OLD.election_event_id, OLD.source, OLD.election_set_key,
             OLD.scope_key, OLD.from_revision, OLD.payload_sha256)
    ) THEN
        RAISE EXCEPTION 'a snapshot figure is only ever closed'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_figure_only_closes';
    END IF;
    SELECT to_revision, to_revision IS NULL INTO previous_end, previous_open
    FROM sequent_backend.monitoring_snapshot_figure
    WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
      AND source = NEW.source AND election_set_key = NEW.election_set_key
      AND scope_key = NEW.scope_key AND from_revision < NEW.from_revision
    ORDER BY from_revision DESC LIMIT 1;
    SELECT from_revision INTO next_start
    FROM sequent_backend.monitoring_snapshot_figure
    WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
      AND source = NEW.source AND election_set_key = NEW.election_set_key
      AND scope_key = NEW.scope_key AND from_revision > NEW.from_revision
    ORDER BY from_revision ASC LIMIT 1;
    IF previous_open OR previous_end > NEW.from_revision
       OR (next_start IS NOT NULL AND (NEW.to_revision IS NULL OR NEW.to_revision > next_start))
    THEN
        RAISE EXCEPTION 'snapshot figures of one scope overlap'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_figure_is_disjoint';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_snapshot_figure_is_disjoint
    BEFORE INSERT OR UPDATE ON sequent_backend.monitoring_snapshot_figure
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_figure_is_disjoint();

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
        CHECK (
            isfinite(bucket_start)
            AND date_bin('15 minutes', bucket_start, '2000-01-01T00:00:00Z') = bucket_start
        ),
    event_type text NOT NULL CHECK (event_type ~ '^[A-Z][A-Z0-9_]{0,63}$'),
    registration text NOT NULL CHECK (registration IN ('REGISTERED', 'UNREGISTERED')),
    area_id uuid CHECK (area_id <> '00000000-0000-0000-0000-000000000000'),
    -- The key's stand-in for a missing area, so the natural key can be the
    -- primary key. Writers name it in ON CONFLICT (..., area_key).
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
