-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Publication completion and immutable file roots are server authority. A raw
-- published_at write must not skip configuration approval on a later publish.
CREATE FUNCTION sequent_backend.guard_publication_authority()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF sequent_backend.trusted_write_allowed() THEN RETURN NEW; END IF;
    IF TG_OP = 'INSERT' THEN
        IF COALESCE(NEW.is_generated, false) OR NEW.published_at IS NOT NULL
           OR NULLIF(NEW.annotations->'ballot_files_v1', 'null'::jsonb) IS NOT NULL THEN
            RAISE EXCEPTION 'Publication authority can only change through the server publication workflow'
                USING ERRCODE = '42501';
        END IF;
        RETURN NEW;
    END IF;
    IF COALESCE(OLD.is_generated, false) IS DISTINCT FROM COALESCE(NEW.is_generated, false)
       OR OLD.published_at IS DISTINCT FROM NEW.published_at
       OR NULLIF(OLD.annotations->'ballot_files_v1', 'null'::jsonb)
          IS DISTINCT FROM NULLIF(NEW.annotations->'ballot_files_v1', 'null'::jsonb)
       OR ((COALESCE(OLD.is_generated, false) OR OLD.published_at IS NOT NULL
            OR NULLIF(OLD.annotations->'ballot_files_v1', 'null'::jsonb) IS NOT NULL)
           AND ROW(OLD.id, OLD.tenant_id, OLD.election_event_id, OLD.election_id, OLD.election_ids)
               IS DISTINCT FROM ROW(NEW.id, NEW.tenant_id, NEW.election_event_id, NEW.election_id, NEW.election_ids)) THEN
        RAISE EXCEPTION 'Publication authority can only change through the server publication workflow'
            USING ERRCODE = '42501', HINT = 'Generate and publish ballots normally; reload a stale form before saving.';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_publication_authority
BEFORE INSERT OR UPDATE ON sequent_backend.ballot_publication
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_publication_authority();

-- The caller already owns a style row during UPDATE/DELETE. Never wait for
-- its parent in the inverse order to publication, which owns parent first.
-- Both parents are checked when moving a style; draft material remains editable.
-- Tombstones change the published country set just as deletion does.
CREATE FUNCTION sequent_backend.guard_publication_style_material()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    parents uuid[] := '{}';
    publication record;
    old_tenant uuid;
    old_event uuid;
    old_parent uuid;
    new_tenant uuid;
    new_event uuid;
    new_parent uuid;
BEGIN
    IF sequent_backend.trusted_write_allowed() THEN
        IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' AND
       ROW(OLD.id, OLD.tenant_id, OLD.election_event_id, OLD.election_id,
           OLD.area_id, OLD.ballot_publication_id, OLD.ballot_eml, OLD.ballot_signature, OLD.deleted_at)
       IS NOT DISTINCT FROM
       ROW(NEW.id, NEW.tenant_id, NEW.election_event_id, NEW.election_id,
           NEW.area_id, NEW.ballot_publication_id, NEW.ballot_eml, NEW.ballot_signature, NEW.deleted_at) THEN
        RETURN NEW;
    END IF;
    IF TG_OP <> 'INSERT' THEN
        old_tenant := OLD.tenant_id; old_event := OLD.election_event_id; old_parent := OLD.ballot_publication_id;
        parents := array_append(parents, old_parent);
    END IF;
    IF TG_OP <> 'DELETE' THEN
        new_tenant := NEW.tenant_id; new_event := NEW.election_event_id; new_parent := NEW.ballot_publication_id;
        parents := array_append(parents, new_parent);
    END IF;
    FOR publication IN
        SELECT id, is_generated, published_at, annotations
        FROM sequent_backend.ballot_publication
        WHERE id = ANY(parents)
          AND ((id = old_parent AND tenant_id IS NOT DISTINCT FROM old_tenant
                AND election_event_id IS NOT DISTINCT FROM old_event)
               OR (id = new_parent AND tenant_id IS NOT DISTINCT FROM new_tenant
                   AND election_event_id IS NOT DISTINCT FROM new_event))
        ORDER BY tenant_id, election_event_id, id FOR UPDATE NOWAIT
    LOOP
        IF COALESCE(publication.is_generated, false) OR publication.published_at IS NOT NULL
           OR NULLIF(publication.annotations->'ballot_files_v1', 'null'::jsonb) IS NOT NULL THEN
            RAISE EXCEPTION 'Generated or published ballot material can only change through the server publication workflow'
                USING ERRCODE = '42501', HINT = 'Generate a new publication to change ballots.';
        END IF;
    END LOOP;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
EXCEPTION WHEN lock_not_available THEN
    RAISE EXCEPTION 'Ballot publication is busy; retry the ballot change'
        USING ERRCODE = '55P03';
END;
$$;
CREATE TRIGGER guard_publication_style_material
BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.ballot_style
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_publication_style_material();
