-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- These tables are intentionally not tracked by Hasura. The authenticated
-- plugin gateway is their only public API; plugins cannot issue SQL.
CREATE TABLE sequent_backend.plugin_tenant (
    tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant(id) ON DELETE CASCADE,
    plugin_id text NOT NULL,
    state text NOT NULL DEFAULT 'disabled' CHECK (state IN ('enabled', 'disabled')),
    data_version integer NOT NULL CHECK (data_version > 0),
    version integer NOT NULL DEFAULT 0 CHECK (version >= 0),
    settings jsonb NOT NULL DEFAULT '{}',
    updated_at timestamptz NOT NULL DEFAULT now(),
    updated_by text NOT NULL,
    PRIMARY KEY (tenant_id, plugin_id)
);

CREATE TABLE sequent_backend.plugin_record (
    tenant_id uuid NOT NULL,
    plugin_id text NOT NULL,
    id uuid NOT NULL,
    kind text NOT NULL,
    parent_id uuid,
    data jsonb NOT NULL CHECK (octet_length(data::text) <= 65536),
    version integer NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    updated_by text NOT NULL,
    PRIMARY KEY (tenant_id, plugin_id, id),
    FOREIGN KEY (tenant_id, plugin_id) REFERENCES sequent_backend.plugin_tenant ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, plugin_id, parent_id)
        REFERENCES sequent_backend.plugin_record (tenant_id, plugin_id, id)
);
CREATE INDEX plugin_record_parent ON sequent_backend.plugin_record (tenant_id, plugin_id, kind, parent_id, id);

CREATE TABLE sequent_backend.plugin_record_event (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id uuid NOT NULL,
    plugin_id text NOT NULL,
    record_id uuid,
    -- The record's parent when the event was written. A parent's history keeps
    -- its children's changes, including deletions.
    parent_id uuid,
    action text NOT NULL,
    actor text NOT NULL,
    at timestamptz NOT NULL DEFAULT now(),
    before_data jsonb,
    after_data jsonb,
    FOREIGN KEY (tenant_id, plugin_id) REFERENCES sequent_backend.plugin_tenant ON DELETE CASCADE
);
CREATE INDEX plugin_record_event_history ON sequent_backend.plugin_record_event (tenant_id, plugin_id, record_id, id DESC);
CREATE INDEX plugin_record_event_children ON sequent_backend.plugin_record_event (tenant_id, plugin_id, parent_id, id DESC);

CREATE FUNCTION sequent_backend.plugin_history_append_only() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- Preserve the existing whole-tenant deletion contract. Turning a plugin
    -- off never deletes its tenant, records, roles, or history.
    IF TG_OP = 'DELETE' AND NOT EXISTS (
        SELECT 1 FROM sequent_backend.tenant WHERE id = OLD.tenant_id
    ) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'Plugin history is append only';
END;
$$;
CREATE TRIGGER plugin_history_append_only
    BEFORE UPDATE OR DELETE ON sequent_backend.plugin_record_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.plugin_history_append_only();

-- Plugin changes made for an admin are logged in the tenant's audit log like
-- other admin changes (SW-F-0364); the gateway names the admin with
-- audit::act_for. The plugin's own history above is what its screens show.
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.plugin_tenant
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.plugin_record
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
