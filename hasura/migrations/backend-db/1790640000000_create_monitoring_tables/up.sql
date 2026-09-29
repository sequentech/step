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
--
-- The triggers below keep the application's writes to the protocols the
-- comments describe; they stop mistakes, not a database owner, who can
-- disable triggers. No table here may be truncated: rows go with their event
-- or by the pruning the snapshot job does.

-- Figure ranges exclude one another; btree_gist lets that constraint compare
-- ids and keys for equality alongside the range overlap. A trusted extension
-- from PostgreSQL's contrib modules, so the database owner may create it; a
-- managed server may need it allowed first (on Azure, in azure.extensions).
-- Marked when this migration creates it, so rolling back removes only what
-- it added.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'btree_gist') THEN
        CREATE EXTENSION btree_gist;
        COMMENT ON EXTENSION btree_gist IS
            'created by migration 1790640000000_create_monitoring_tables';
    END IF;
END;
$$;

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

-- Refuses deleting a row while its monitoring event is there: inside the
-- event's cascade the event row is already gone, and any other deletion,
-- from a statement or another trigger, still sees it.
CREATE FUNCTION sequent_backend.monitoring_deleted_with_event()
RETURNS trigger AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_event
        WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
    ) THEN
        RAISE EXCEPTION '% rows are deleted with their monitoring event', TG_TABLE_NAME
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = TG_ARGV[0];
    END IF;
    RETURN OLD;
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
    -- Raised by every saved change, reset and mode switch, so a snapshot and
    -- an export say which configuration they were counted for. Only those
    -- write this row: it is what queues them behind one another, and the
    -- snapshot job keeps its own row in monitoring_snapshot_state.
    -- It starts at 0 and only moves on by one.
    config_generation bigint NOT NULL DEFAULT 0 CHECK (config_generation >= 0),
    -- The transaction that last raised config_generation, kept by a trigger:
    -- a revision is written only by the change that raised the generation.
    -- Meaningful only to the cluster that wrote it: after a dump is restored
    -- elsewhere, one transaction may happen to have this id, and only a bug
    -- in exactly that one could then write a revision without a raise.
    config_generation_xact xid8,
    created_at timestamptz NOT NULL DEFAULT now(),
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

CREATE FUNCTION sequent_backend.monitoring_event_generation_moves_by_one()
RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND (NEW.tenant_id, NEW.election_event_id) IS DISTINCT FROM (OLD.tenant_id, OLD.election_event_id)
    THEN
        RAISE EXCEPTION 'a monitoring event keeps its election event'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_event_keeps_its_election_event';
    END IF;
    IF (TG_OP = 'INSERT' AND NEW.config_generation <> 0)
       OR (TG_OP = 'UPDATE'
           AND NEW.config_generation NOT IN (OLD.config_generation, OLD.config_generation + 1))
    THEN
        RAISE EXCEPTION 'the configuration generation starts at 0 and moves on by one'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_event_generation_moves_by_one';
    END IF;
    IF TG_OP = 'INSERT' THEN
        NEW.config_generation_xact := NULL;
    ELSIF NEW.config_generation <> OLD.config_generation THEN
        -- The top-level transaction, savepoints included.
        NEW.config_generation_xact := pg_current_xact_id();
    ELSE
        NEW.config_generation_xact := OLD.config_generation_xact;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_event_generation_moves_by_one
    BEFORE INSERT OR UPDATE ON sequent_backend.monitoring_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_event_generation_moves_by_one();

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
    -- The event's generation once the change that wrote the revision is
    -- saved. The configuration of generation G is, for each document, its
    -- revision of greatest generation up to G, unless that removed it.
    config_generation bigint NOT NULL CHECK (config_generation > 0),
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

CREATE FUNCTION sequent_backend.monitoring_config_generation_is_current()
RETURNS trigger AS $$
DECLARE
    current bigint;
    raised_by xid8;
BEGIN
    SELECT config_generation, config_generation_xact INTO current, raised_by
    FROM sequent_backend.monitoring_event
    WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id;
    -- Without its event the revision's foreign key refuses it.
    IF FOUND AND (
        NEW.config_generation IS DISTINCT FROM current
        OR raised_by IS DISTINCT FROM pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'revision % of % % is not of the generation this transaction raised',
                NEW.revision, NEW.kind, NEW.key
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_config_generation_is_current';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- A revision takes the generation its change raised the event to, in the
-- same transaction.
CREATE TRIGGER monitoring_config_generation_is_current
    BEFORE INSERT ON sequent_backend.monitoring_config
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_config_generation_is_current();

-- The configuration as of a generation, for each document.
CREATE INDEX monitoring_config_by_generation
    ON sequent_backend.monitoring_config
        (tenant_id, election_event_id, kind, key, config_generation);

-- The live revision of each document, a DELETE revision for a removed one.
-- A save, under READ COMMITTED (a stricter isolation level fails the later
-- save with a serialization error instead):
--   1. UPDATE monitoring_event SET config_generation = config_generation + 1
--      RETURNING config_generation, which also queues saves, resets and mode
--      switches of one event behind each other;
--   2. UPDATE the head SET revision = revision + 1
--      WHERE ... AND revision = <expected>, or for a new document
--      INSERT the head (revision 1) ON CONFLICT DO NOTHING;
--   3. INSERT the revision the head now names, of that generation.
-- Touching no head row in step 2 is the conflict to report, never an error.
-- A head only moves on by one, and at commit every revision written must be
-- its document's head, so the history is exactly the changes that took
-- effect, one revision each.
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

CREATE FUNCTION sequent_backend.monitoring_config_head_moves_by_one()
RETURNS trigger AS $$
BEGIN
    IF (TG_OP = 'INSERT' AND NEW.revision <> 1)
       OR (TG_OP = 'UPDATE' AND (
           NEW.revision <> OLD.revision + 1
           OR (NEW.tenant_id, NEW.election_event_id, NEW.kind, NEW.key)
              IS DISTINCT FROM (OLD.tenant_id, OLD.election_event_id, OLD.kind, OLD.key)
       ))
    THEN
        RAISE EXCEPTION 'the head of % % moves on by one revision', NEW.kind, NEW.key
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_config_head_moves_by_one';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_config_head_moves_by_one
    BEFORE INSERT OR UPDATE ON sequent_backend.monitoring_config_head
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_config_head_moves_by_one();
-- A removed document keeps its head, at its DELETE revision.
CREATE TRIGGER monitoring_config_head_is_kept
    BEFORE DELETE ON sequent_backend.monitoring_config_head
    FOR EACH ROW
    EXECUTE FUNCTION sequent_backend.monitoring_deleted_with_event('monitoring_config_head_is_kept');

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
          AND kind = NEW.kind AND key = NEW.key AND revision = NEW.revision
    ) THEN
        RAISE EXCEPTION 'revision % of % % is not the head', NEW.revision, NEW.kind, NEW.key
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_config_revision_is_the_head';
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Checked at commit, when the head has moved: a revision the head does not
-- name by then never took effect.
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
-- run is final, but for `checked_at`, which only moves on. Runs complete in
-- revision order (see monitoring_snapshot_state.last_complete_revision), so
-- a pass that fell behind a later one can no longer complete: the job takes
-- the event's lock before it records its run, and a pass that finds a newer
-- complete run marks its own FAILED as superseded. A run left RUNNING by a
-- crash is marked FAILED by a later pass. One transaction writes everything
-- a pass counted and finishes its run; what another transaction writes for
-- the run is refused at its commit.
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
    -- The transaction that completed or failed the run, set by
    -- monitoring_snapshot_run_records_who_finished. Meaningful only to the
    -- cluster that wrote it: a dump restored elsewhere starts transaction
    -- ids afresh, which is harmless here, as rows are only written for a
    -- RUNNING run and a finished run never runs again.
    finished_xact xid8,
    PRIMARY KEY (tenant_id, election_event_id, revision),
    CONSTRAINT monitoring_snapshot_run_status_key
        UNIQUE (tenant_id, election_event_id, revision, status),
    CONSTRAINT monitoring_snapshot_run_running_is_open
        CHECK (status <> 'RUNNING' OR (finished_at IS NULL AND error IS NULL)),
    CONSTRAINT monitoring_snapshot_run_finished_by_a_transaction
        CHECK ((status = 'RUNNING') = (finished_xact IS NULL)),
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
    CONSTRAINT monitoring_snapshot_run_checked_when_complete
        CHECK (checked_at IS NULL OR (status = 'COMPLETE' AND checked_at >= as_of)),
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
        NEW.finished_at, NEW.as_of, NEW.settings_revision, NEW.config_generation, NEW.error,
        NEW.finished_xact
    ) IS DISTINCT FROM (
        OLD.tenant_id, OLD.election_event_id, OLD.revision, OLD.status, OLD.started_at,
        OLD.finished_at, OLD.as_of, OLD.settings_revision, OLD.config_generation, OLD.error,
        OLD.finished_xact
    ) THEN
        RAISE EXCEPTION 'snapshot run % is final', OLD.revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_run_is_final';
    END IF;
    IF NEW.checked_at IS DISTINCT FROM OLD.checked_at
       AND (NEW.checked_at IS NULL OR NEW.checked_at < OLD.checked_at)
    THEN
        RAISE EXCEPTION 'snapshot run % was checked later than that', OLD.revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_run_is_final';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_snapshot_run_is_final
    BEFORE UPDATE ON sequent_backend.monitoring_snapshot_run
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_run_is_final();

-- Whatever a statement says, `finished_xact` names the transaction that took
-- the run out of RUNNING, and never changes after.
CREATE FUNCTION sequent_backend.monitoring_snapshot_run_records_who_finished()
RETURNS trigger AS $$
BEGIN
    IF NEW.status = 'RUNNING' THEN
        NEW.finished_xact := NULL;
    ELSIF TG_OP = 'INSERT' OR OLD.status = 'RUNNING' THEN
        NEW.finished_xact := pg_current_xact_id();
    ELSE
        NEW.finished_xact := OLD.finished_xact;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Named to fire before monitoring_snapshot_run_is_final, which then sees
-- what this kept.
CREATE TRIGGER monitoring_snapshot_run_finish_is_recorded
    BEFORE INSERT OR UPDATE ON sequent_backend.monitoring_snapshot_run
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_run_records_who_finished();

-- What the snapshot job keeps for an event: the run viewers are shown and
-- where each incremental pass stopped. Its own row, so the job never waits
-- for a configuration save, nor a save for a pass: they share no row they
-- both write.
CREATE TABLE sequent_backend.monitoring_snapshot_state (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    -- The run viewers are shown; none until the first completes. It only
    -- moves on.
    live_snapshot_revision bigint,
    -- Lets the foreign key below require the shown run to be complete.
    live_snapshot_status text GENERATED ALWAYS AS (
        CASE WHEN live_snapshot_revision IS NOT NULL THEN 'COMPLETE' END
    ) STORED,
    -- The newest run that completed, recorded by the completion itself: of
    -- two passes of one event, the older can then never complete after the
    -- newer. Under REPEATABLE READ it fails to serialize; under READ
    -- COMMITTED it waits for the newer, then is refused. It only moves on.
    last_complete_revision bigint,
    watermarks jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(watermarks) = 'object'),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id),
    CONSTRAINT monitoring_snapshot_state_shows_what_completed
        CHECK (
            live_snapshot_revision IS NULL
            OR (
                last_complete_revision IS NOT NULL
                AND live_snapshot_revision <= last_complete_revision
            )
        ),
    CONSTRAINT monitoring_snapshot_state_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.monitoring_event (tenant_id, election_event_id)
        ON DELETE CASCADE,
    -- Viewers are only shown a complete run, which cannot be pruned while
    -- shown.
    CONSTRAINT monitoring_snapshot_state_shows_a_complete_run
        FOREIGN KEY (tenant_id, election_event_id, live_snapshot_revision, live_snapshot_status)
        REFERENCES sequent_backend.monitoring_snapshot_run
            (tenant_id, election_event_id, revision, status)
);

CREATE FUNCTION sequent_backend.monitoring_snapshot_state_moves_forward()
RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF EXISTS (
            SELECT 1 FROM sequent_backend.monitoring_event
            WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
        ) THEN
            RAISE EXCEPTION 'the snapshot state goes with its event'
                USING ERRCODE = 'integrity_constraint_violation',
                      CONSTRAINT = 'monitoring_snapshot_state_moves_forward';
        END IF;
        RETURN OLD;
    END IF;
    IF OLD.live_snapshot_revision IS NOT NULL AND (
        NEW.live_snapshot_revision IS NULL
        OR NEW.live_snapshot_revision < OLD.live_snapshot_revision
    ) THEN
        RAISE EXCEPTION 'viewers were shown revision %; they are not shown an older one',
                OLD.live_snapshot_revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_state_moves_forward';
    END IF;
    IF OLD.last_complete_revision IS NOT NULL AND (
        NEW.last_complete_revision IS NULL
        OR NEW.last_complete_revision < OLD.last_complete_revision
    ) THEN
        RAISE EXCEPTION 'run % completed; no earlier run completes after it',
                OLD.last_complete_revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_state_moves_forward';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_snapshot_state_moves_forward
    BEFORE UPDATE OR DELETE ON sequent_backend.monitoring_snapshot_state
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_state_moves_forward();

CREATE FUNCTION sequent_backend.monitoring_snapshot_run_completes_in_order()
RETURNS trigger AS $$
DECLARE
    recorded integer;
BEGIN
    IF NEW.status <> 'COMPLETE' OR (TG_OP = 'UPDATE' AND OLD.status = 'COMPLETE') THEN
        RETURN NEW;
    END IF;
    -- Without its event the run's foreign key refuses it.
    IF NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_event
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
    ) THEN
        RETURN NEW;
    END IF;
    INSERT INTO sequent_backend.monitoring_snapshot_state AS state
        (tenant_id, election_event_id, last_complete_revision)
    VALUES (NEW.tenant_id, NEW.election_event_id, NEW.revision)
    ON CONFLICT (tenant_id, election_event_id) DO UPDATE
        SET last_complete_revision = EXCLUDED.last_complete_revision
        WHERE state.last_complete_revision IS NULL
           OR state.last_complete_revision < EXCLUDED.last_complete_revision;
    GET DIAGNOSTICS recorded = ROW_COUNT;
    IF recorded = 0 THEN
        RAISE EXCEPTION 'snapshot run % would complete after a later run did', NEW.revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_run_completes_in_order';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- A run completes once, after every run of its event before it that did.
CREATE TRIGGER monitoring_snapshot_run_completes_in_order
    BEFORE INSERT OR UPDATE ON sequent_backend.monitoring_snapshot_run
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_run_completes_in_order();

-- The snapshot tables' references to one another are checked at commit, so
-- an event's cascade may remove both ends in either order. A pass never sets
-- all constraints immediate: what it writes is checked at commit against its
-- run having finished, so it names any constraint it wants checked early.
-- The snapshot job prunes in a transaction of its own that starts with
-- SET CONSTRAINTS ALL IMMEDIATE, so a pruning mistake fails at the statement
-- that made it and leaves the snapshot it wrote alone. It prunes an event
-- after its pass, under the same per-event lock, so pruning never races a
-- pass for a payload the pass is about to name again.
--
-- A reader of revision R reads the run and its figures in one statement, or
-- in one REPEATABLE READ transaction that first finds the run COMPLETE: a
-- run pruned meanwhile then still shows all its figures, and one pruned
-- before is gone, never partly there.

-- Whether each source was counted in a run, for each set of elections.
-- Written by the pass of a RUNNING run, whose transaction has finished the
-- run, complete or failed, by commit; then kept until the run is pruned.
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

CREATE FUNCTION sequent_backend.monitoring_snapshot_source_is_kept()
RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        -- Inside its run's cascade, or its event's, the row above is gone.
        IF EXISTS (
            SELECT 1 FROM sequent_backend.monitoring_event
            WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
        ) AND EXISTS (
            SELECT 1 FROM sequent_backend.monitoring_snapshot_run
            WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
              AND revision = OLD.revision
        ) THEN
            RAISE EXCEPTION 'what run % counted goes with the run', OLD.revision
                USING ERRCODE = 'integrity_constraint_violation',
                      CONSTRAINT = 'monitoring_snapshot_source_is_kept';
        END IF;
        RETURN OLD;
    END IF;
    -- Without its run the foreign key refuses it.
    IF EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_run
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND revision = NEW.revision AND status <> 'RUNNING'
    ) THEN
        RAISE EXCEPTION 'snapshot run % is finished; what it counted is not added to', NEW.revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_source_of_a_running_pass';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_snapshot_source_is_kept
    BEFORE INSERT OR DELETE ON sequent_backend.monitoring_snapshot_source
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_source_is_kept();

CREATE FUNCTION sequent_backend.monitoring_snapshot_source_finishes_its_run()
RETURNS trigger AS $$
BEGIN
    -- A row deleted since, with its run or its event, is not kept.
    IF EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_source
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND revision = NEW.revision AND source = NEW.source
          AND election_set_key = NEW.election_set_key
    ) AND NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_run
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND revision = NEW.revision AND finished_xact = pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'snapshot run % was not finished by the pass that counted it', NEW.revision
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_source_finishes_its_run';
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Checked at commit: the transaction that wrote the row completed or failed
-- its run.
CREATE CONSTRAINT TRIGGER monitoring_snapshot_source_finishes_its_run
    AFTER INSERT ON sequent_backend.monitoring_snapshot_source
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_source_finishes_its_run();

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

-- A payload a figure names is kept, so no DELETE and INSERT under the same
-- hash changes what a revision shows; pruning deletes it once the figures
-- naming it are gone. Inside the event's cascade the event row is gone.
CREATE FUNCTION sequent_backend.monitoring_snapshot_payload_is_kept()
RETURNS trigger AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_event
        WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
    ) AND EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_figure
        WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
          AND payload_sha256 = OLD.sha256
    ) THEN
        RAISE EXCEPTION 'a snapshot figure names this payload'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_payload_is_kept';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_snapshot_payload_is_kept
    BEFORE DELETE ON sequent_backend.monitoring_snapshot_payload
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_payload_is_kept();

-- Which payload a scope showed, over the runs from `from_revision` up to but
-- not including `to_revision` (still showing when NULL). Only the pass of a
-- RUNNING run writes rows, and at commit its run is COMPLETE, so a complete
-- run's figures never change: a pass opens rows from its own revision and
-- closes rows at it, and never changes what an earlier complete run holds.
-- It writes only the scopes that changed, closing the old row before opening
-- the new one, and closes the rows of scopes and election sets it no longer
-- counts. The figures of revision R are the rows whose range holds R:
--   int8range(from_revision, to_revision) @> R
-- A row is deleted only once no complete run it holds is left, so pruning
-- deletes the runs it drops first, then the rows holding none of the rest,
-- then the payloads no row names any more.
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
    -- Enforced by an index, so two writers cannot both miss the other's row.
    CONSTRAINT monitoring_snapshot_figure_is_disjoint
        EXCLUDE USING gist (
            tenant_id WITH =, election_event_id WITH =, source WITH =,
            election_set_key WITH =, scope_key WITH =,
            int8range(from_revision, to_revision) WITH &&
        ),
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

-- Payloads still named, and sets still counted.
CREATE INDEX monitoring_snapshot_figure_payload
    ON sequent_backend.monitoring_snapshot_figure (tenant_id, election_event_id, payload_sha256);
CREATE INDEX monitoring_snapshot_figure_election_set
    ON sequent_backend.monitoring_snapshot_figure (tenant_id, election_event_id, election_set_key);

-- A figure row starts and ends at runs of its event, is only ever closed,
-- and is deleted only once it holds no complete run: the one viewers are
-- shown, or one kept for export.
CREATE FUNCTION sequent_backend.monitoring_snapshot_figure_is_kept()
RETURNS trigger AS $$
DECLARE
    pass bigint;
BEGIN
    IF TG_OP = 'DELETE' THEN
        -- Inside the event's cascade the event row is already gone.
        IF EXISTS (
            SELECT 1 FROM sequent_backend.monitoring_event
            WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
        ) AND EXISTS (
            SELECT 1 FROM sequent_backend.monitoring_snapshot_run
            WHERE tenant_id = OLD.tenant_id AND election_event_id = OLD.election_event_id
              AND revision >= OLD.from_revision
              AND (OLD.to_revision IS NULL OR revision < OLD.to_revision)
              AND status = 'COMPLETE'
        ) THEN
            RAISE EXCEPTION 'snapshot figures from revision % are still read', OLD.from_revision
                USING ERRCODE = 'integrity_constraint_violation',
                      CONSTRAINT = 'monitoring_snapshot_figure_is_read';
        END IF;
        RETURN OLD;
    END IF;
    -- Without its event the row's foreign key refuses it.
    IF NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_event
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
    ) THEN
        RETURN NEW;
    END IF;
    IF (TG_OP = 'INSERT' AND NEW.to_revision IS NOT NULL)
       OR (TG_OP = 'UPDATE' AND NOT (
           OLD.to_revision IS NULL AND NEW.to_revision IS NOT NULL
           AND (NEW.tenant_id, NEW.election_event_id, NEW.source, NEW.election_set_key,
                NEW.scope_key, NEW.from_revision, NEW.payload_sha256)
               IS NOT DISTINCT FROM
               (OLD.tenant_id, OLD.election_event_id, OLD.source, OLD.election_set_key,
                OLD.scope_key, OLD.from_revision, OLD.payload_sha256)
       ))
    THEN
        RAISE EXCEPTION 'a snapshot figure is opened showing and only ever closed'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_figure_only_closes';
    END IF;
    pass := CASE WHEN TG_OP = 'INSERT' THEN NEW.from_revision ELSE NEW.to_revision END;
    IF NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_run
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND revision = pass AND status = 'RUNNING'
    ) THEN
        RAISE EXCEPTION 'snapshot figures are written by the pass of a running run of their event'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_figure_of_a_running_pass';
    END IF;
    -- A pass left running by a crash is older than later complete runs.
    IF EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_run
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND revision >= pass AND status = 'COMPLETE'
    ) THEN
        RAISE EXCEPTION 'run % would change what a complete run shows', pass
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_figure_is_read';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- At commit the transaction that wrote a row has completed its run: figures
-- of a failed or abandoned pass never become what later passes carry
-- forward, and no other transaction adds to a run once it completed.
CREATE FUNCTION sequent_backend.monitoring_snapshot_figure_completes_its_run()
RETURNS trigger AS $$
BEGIN
    -- A row deleted since, with its event or otherwise, is not shown.
    IF NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_figure
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND source = NEW.source AND election_set_key = NEW.election_set_key
          AND scope_key = NEW.scope_key AND from_revision = NEW.from_revision
    ) THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM sequent_backend.monitoring_snapshot_run
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.election_event_id
          AND revision = CASE WHEN TG_OP = 'INSERT' THEN NEW.from_revision ELSE NEW.to_revision END
          AND status = 'COMPLETE' AND finished_xact = pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'snapshot figures commit in the transaction that completed their run'
            USING ERRCODE = 'integrity_constraint_violation',
                  CONSTRAINT = 'monitoring_snapshot_figure_completes_its_run';
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER monitoring_snapshot_figure_is_kept
    BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.monitoring_snapshot_figure
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_figure_is_kept();
CREATE CONSTRAINT TRIGGER monitoring_snapshot_figure_completes_its_run
    AFTER INSERT OR UPDATE ON sequent_backend.monitoring_snapshot_figure
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_figure_completes_its_run();
CREATE TRIGGER monitoring_snapshot_figure_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_snapshot_figure
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_snapshot_figure_is_read');

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
CREATE TRIGGER set_sequent_backend_monitoring_snapshot_state_updated_at
    BEFORE UPDATE ON sequent_backend.monitoring_snapshot_state
    FOR EACH ROW EXECUTE PROCEDURE sequent_backend.set_current_timestamp_updated_at();
CREATE TRIGGER set_sequent_backend_monitoring_config_head_updated_at
    BEFORE UPDATE ON sequent_backend.monitoring_config_head
    FOR EACH ROW EXECUTE PROCEDURE sequent_backend.set_current_timestamp_updated_at();
CREATE TRIGGER set_sequent_backend_monitoring_voter_updated_at
    BEFORE UPDATE ON sequent_backend.monitoring_voter
    FOR EACH ROW EXECUTE PROCEDURE sequent_backend.set_current_timestamp_updated_at();

-- TRUNCATE skips the row triggers above.
CREATE TRIGGER monitoring_event_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_event
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_event_is_not_truncated');
CREATE TRIGGER monitoring_config_head_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_config_head
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_config_head_is_not_truncated');
CREATE TRIGGER monitoring_election_set_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_election_set
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_election_set_is_not_truncated');
CREATE TRIGGER monitoring_snapshot_run_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_snapshot_run
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_snapshot_run_is_not_truncated');
CREATE TRIGGER monitoring_snapshot_state_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_snapshot_state
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_snapshot_state_is_not_truncated');
CREATE TRIGGER monitoring_snapshot_source_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_snapshot_source
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_snapshot_source_is_not_truncated');
CREATE TRIGGER monitoring_snapshot_payload_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_snapshot_payload
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_snapshot_payload_is_not_truncated');
CREATE TRIGGER monitoring_voter_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_voter
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_voter_is_not_truncated');
CREATE TRIGGER monitoring_login_counter_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_login_counter
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_login_counter_is_not_truncated');
CREATE TRIGGER monitoring_login_counter_receipt_is_not_truncated
    BEFORE TRUNCATE ON sequent_backend.monitoring_login_counter_receipt
    FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.monitoring_refuse_change('monitoring_login_counter_receipt_is_not_truncated');
