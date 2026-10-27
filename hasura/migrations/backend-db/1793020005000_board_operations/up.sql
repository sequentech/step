-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- These ledgers have no GraphQL permissions. Only a consumed signed request can
-- bind one durable task/session; queue messages never grant authority.
CREATE TABLE sequent_backend.board_operation_binding (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    request_id uuid NOT NULL UNIQUE,
    binding_kind text NOT NULL CHECK(binding_kind IN ('import','export','deletion','tally','voter_export')),
    binding_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(tenant_id,election_event_id,binding_kind,binding_id)
);
CREATE TABLE sequent_backend.register_read_session (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    user_id text NOT NULL,
    request_id uuid NOT NULL,
    lifecycle_generation bigint NOT NULL,
    source_revision bigint NOT NULL,
    expires_at timestamptz NOT NULL,
    PRIMARY KEY(tenant_id,election_event_id,user_id)
);
CREATE TABLE sequent_backend.election_execution_revision (
    tenant_id uuid NOT NULL,
    election_event_id uuid PRIMARY KEY REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE,
    revision bigint NOT NULL DEFAULT 0
);
INSERT INTO sequent_backend.election_execution_revision(tenant_id,election_event_id)
SELECT tenant_id,id FROM sequent_backend.election_event;
CREATE TABLE sequent_backend.execution_export (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    document_id uuid PRIMARY KEY,
    request_id uuid NOT NULL,
    source_revision bigint NOT NULL,
    lifecycle_generation bigint NOT NULL,
    manifest_sha256 text NOT NULL CHECK(length(manifest_sha256)=64),
    archive_sha256 text NOT NULL CHECK(length(archive_sha256)=64),
    signing_public_key text NOT NULL,
    completed_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE FUNCTION sequent_backend.authorized_execution_deletion(tenant uuid,event uuid)
RETURNS boolean LANGUAGE sql STABLE AS $$
SELECT sequent_backend.trusted_write_allowed() AND EXISTS (
    SELECT 1 FROM sequent_backend.board_operation_binding b
    JOIN sequent_backend.signing_request r ON r.id=b.request_id
    JOIN sequent_backend.election_lifecycle l ON l.election_event_id=b.election_event_id AND l.tenant_id=b.tenant_id
    JOIN sequent_backend.election_execution_revision v ON v.election_event_id=b.election_event_id AND v.tenant_id=b.tenant_id
    JOIN sequent_backend.execution_export x ON x.election_event_id=b.election_event_id AND x.tenant_id=b.tenant_id
    WHERE b.tenant_id=tenant AND b.election_event_id=event AND b.binding_kind='deletion'
      AND b.request_id::text=current_setting('sequent.authorized_deletion',true)
      AND r.action='delete-execution-data' AND r.status='executed'
      AND l.phase='post_processing' AND l.generation=x.lifecycle_generation
      AND v.revision=x.source_revision
      AND r.subject->>'lifecycle_generation'=l.generation::text
      AND r.subject->>'source_revision'=v.revision::text
);
$$;
CREATE FUNCTION sequent_backend.track_execution_revision() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE row_data jsonb; event uuid; tenant uuid;
BEGIN
    row_data=CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
    tenant=(row_data->>'tenant_id')::uuid;
    event=CASE WHEN TG_TABLE_NAME='election_event' THEN (row_data->>'id')::uuid ELSE (row_data->>'election_event_id')::uuid END;
    IF event IS NULL OR sequent_backend.authorized_execution_deletion(tenant,event) THEN
        RETURN NULL;
    END IF;
    -- Parent deletes cascade the ledger; do not recreate it during cascades.
    IF TG_TABLE_NAME='election_event' AND TG_OP='DELETE' THEN RETURN NULL; END IF;
    INSERT INTO sequent_backend.election_execution_revision(tenant_id,election_event_id,revision)
    VALUES(tenant,event,1) ON CONFLICT(election_event_id) DO UPDATE SET revision=election_execution_revision.revision+1;
    RETURN NULL;
END;
$$;
DO $$ DECLARE tab text; BEGIN
    FOREACH tab IN ARRAY ARRAY['election_event','election','contest','candidate','area','area_contest',
        'scheduled_event','ballot_publication','ballot_style','election_result','keys_ceremony','protocol_board',
        'tally_session','tally_session_contest','tally_session_execution','tally_session_resolution',
        'tally_sheet','ballot_box_seal','results_event','results_election','results_contest',
        'results_contest_candidate','results_election_area','results_area_contest','results_area_contest_candidate'] LOOP
        EXECUTE format('CREATE TRIGGER track_execution_revision AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.%I FOR EACH ROW EXECUTE FUNCTION sequent_backend.track_execution_revision()',tab);
    END LOOP;
END $$;

-- Seals remain immutable until the board authorizes a fully exported teardown.
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_permanence"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    -- Columns a pending seal may change while it stays pending.
    provenance text[] := ARRAY['close_request_id', 'closed_by', 'last_attempt_at', 'waiting_reason'];
    seal_fields text[] := ARRAY[
        'sealed_at', 'ballots_in_box', 'ballots_counted', 'seal_hash',
        'manifest', 'signed_message', 'election_name', 'area_name'
    ];
    -- The seal's (or the failure's) record on the event's electoral log.
    log_fields text[] := ARRAY['log_delivery_id', 'log_uid', 'log_leaf_index'];
    publication_fields text[] := ARRAY[
        'public_document_id', 'public_path', 'published_at'
    ];
BEGIN
    IF TG_OP = 'DELETE' AND sequent_backend.authorized_execution_deletion(OLD.tenant_id,OLD.election_event_id) THEN RETURN OLD; END IF;
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'The seal of ballot box % is permanent and can''t be deleted', OLD.id
            USING ERRCODE = '42501',
                  HINT = 'Ballot box seals are kept for the audit of the election.';
    END IF;

    IF TG_OP = 'INSERT' THEN
        IF NEW.status = 'pending'
           AND NEW.sealed_at IS NULL AND NEW.ballots_in_box IS NULL
           AND NEW.ballots_counted IS NULL AND NEW.seal_hash IS NULL
           AND NEW.manifest IS NULL AND NEW.signed_message IS NULL
           AND NEW.failure_reason IS NULL AND NEW.log_delivery_id IS NULL
           AND NEW.log_uid IS NULL AND NEW.log_leaf_index IS NULL
           AND NEW.public_document_id IS NULL AND NEW.public_path IS NULL
           AND NEW.published_at IS NULL
           AND NEW.election_name IS NULL AND NEW.area_name IS NULL THEN
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'A new ballot box seal can only start pending, without seal data'
            USING ERRCODE = '42501',
                  HINT = 'Seals are created when voting closes and sealed by the server.';
    END IF;

    IF OLD.status = 'pending' THEN
        IF NEW.status = 'pending'
           AND to_jsonb(NEW) - provenance = to_jsonb(OLD) - provenance THEN
            RETURN NEW;
        END IF;
        IF NEW.status = 'sealed'
           AND to_jsonb(NEW) - provenance - seal_fields - log_fields - 'status'
               = to_jsonb(OLD) - provenance - seal_fields - log_fields - 'status'
           AND NEW.sealed_at IS NOT NULL AND NEW.ballots_in_box IS NOT NULL
           AND NEW.ballots_counted IS NOT NULL AND NEW.seal_hash IS NOT NULL
           AND NEW.manifest IS NOT NULL AND NEW.signed_message IS NOT NULL
           AND NEW.log_delivery_id IS NOT NULL AND NEW.log_uid IS NOT NULL
           AND NEW.log_leaf_index IS NOT NULL THEN
            RETURN NEW;
        END IF;
        IF NEW.status = 'failed'
           AND to_jsonb(NEW) - provenance - log_fields - 'failure_reason' - 'status'
               - 'election_name' - 'area_name'
               = to_jsonb(OLD) - provenance - log_fields - 'failure_reason' - 'status'
               - 'election_name' - 'area_name'
           AND NEW.failure_reason IS NOT NULL
           AND (NEW.log_delivery_id IS NULL) = (NEW.log_uid IS NULL)
           AND (NEW.log_uid IS NULL) = (NEW.log_leaf_index IS NULL) THEN
            RETURN NEW;
        END IF;
    ELSIF OLD.status = 'sealed' THEN
        IF NEW.status IN ('sealed', 'published')
           AND to_jsonb(NEW) - publication_fields - 'status'
               = to_jsonb(OLD) - publication_fields - 'status'
           AND (OLD.public_document_id IS NULL OR NEW.public_document_id IS NOT DISTINCT FROM OLD.public_document_id)
           AND (OLD.public_path IS NULL OR NEW.public_path IS NOT DISTINCT FROM OLD.public_path)
           AND (OLD.published_at IS NULL OR NEW.published_at IS NOT DISTINCT FROM OLD.published_at)
           AND (NEW.status = 'sealed' OR (
                NEW.public_document_id IS NOT NULL
                AND NEW.published_at IS NOT NULL
                -- A public path exactly when the event's record policy is public.
                AND (NEW.public_path IS NOT NULL) = EXISTS (
                    SELECT 1
                    FROM "sequent_backend"."election_event" AS event
                    WHERE event.tenant_id = NEW.tenant_id
                      AND event.id = NEW.election_event_id
                      AND "sequent_backend"."ballot_box_seal_effective_policy"(
                              event.presentation::jsonb, 'ballot_box_seal_record_policy', 'restricted')
                          = '"public"'::jsonb
                )
           )) THEN
            RETURN NEW;
        END IF;
    END IF;

    RAISE EXCEPTION 'The % seal of ballot box % can''t change this way (to %)',
        OLD.status, OLD.id, NEW.status
        USING ERRCODE = '42501',
              HINT = 'A seal only moves pending → sealed → published, or pending → failed; published and failed seals are final.';
END;
$$;
