-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Internal, tenant-scoped cache/outbox. Deliberately not exposed through Hasura.
CREATE TABLE sequent_backend.report_prerender (
  report_id uuid PRIMARY KEY REFERENCES sequent_backend.report(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL,
  election_event_id uuid NOT NULL,
  generation bigint NOT NULL DEFAULT 1,
  ready_generation bigint,
  state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','building','ready','failed')),
  lease_token uuid,
  lease_until timestamptz,
  attempts integer NOT NULL DEFAULT 0,
  retry_after timestamptz NOT NULL DEFAULT now(),
  fingerprint text,
  manifest jsonb,
  background bytea CHECK (octet_length(background) <= 24000000),
  error text,
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX report_prerender_pending ON sequent_backend.report_prerender (retry_after, updated_at)
  WHERE state <> 'ready';

CREATE FUNCTION sequent_backend.queue_report_prerenders(p_tenant uuid, p_event uuid DEFAULT NULL, p_alias text DEFAULT NULL)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  INSERT INTO sequent_backend.report_prerender AS cache (report_id, tenant_id, election_event_id)
  SELECT r.id, r.tenant_id, r.election_event_id FROM sequent_backend.report r
  JOIN sequent_backend.template t ON t.tenant_id=r.tenant_id AND t.alias=r.template_alias
  WHERE r.tenant_id=p_tenant AND (p_event IS NULL OR r.election_event_id=p_event)
    AND (p_alias IS NULL OR r.template_alias=p_alias)
    AND t.template #>> '{pre_render,enabled}' = 'true'
  ORDER BY r.id
  ON CONFLICT (report_id) DO UPDATE SET
    tenant_id=EXCLUDED.tenant_id, election_event_id=EXCLUDED.election_event_id,
    generation=cache.generation+1, state='pending', attempts=0,
    retry_after=now(), lease_token=NULL, lease_until=NULL, error=NULL, updated_at=now();
  -- Disabling/deleting a template must not leave an apparently usable cache.
  DELETE FROM sequent_backend.report_prerender cache
  USING sequent_backend.report r
  WHERE cache.report_id=r.id AND r.tenant_id=p_tenant
    AND (p_event IS NULL OR r.election_event_id=p_event)
    AND (p_alias IS NULL OR r.template_alias=p_alias)
    AND NOT EXISTS (SELECT 1 FROM sequent_backend.template t WHERE t.tenant_id=r.tenant_id
      AND t.alias=r.template_alias AND t.template #>> '{pre_render,enabled}'='true');
END $$;

CREATE FUNCTION sequent_backend.invalidate_report_prerenders() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE old_value jsonb; new_value jsonb; value jsonb;
BEGIN
  IF TG_OP <> 'INSERT' THEN old_value=to_jsonb(OLD); END IF;
  IF TG_OP <> 'DELETE' THEN new_value=to_jsonb(NEW); END IF;
  IF TG_OP='UPDATE' THEN
    IF TG_TABLE_NAME='template' AND
      (old_value->'template'->'document', old_value->'template'->'assets', old_value->'template'->'pdf_options', old_value->'template'->'pre_render', old_value->'alias', old_value->'tenant_id') IS NOT DISTINCT FROM
      (new_value->'template'->'document', new_value->'template'->'assets', new_value->'template'->'pdf_options', new_value->'template'->'pre_render', new_value->'alias', new_value->'tenant_id') THEN RETURN NEW;
    ELSIF TG_TABLE_NAME='report' AND
      (old_value->'template_alias',old_value->'election_id',old_value->'election_event_id',old_value->'tenant_id',old_value->'report_type') IS NOT DISTINCT FROM
      (new_value->'template_alias',new_value->'election_id',new_value->'election_event_id',new_value->'tenant_id',new_value->'report_type') THEN RETURN NEW;
    ELSIF TG_TABLE_NAME IN ('election_event','election','contest','candidate') AND
      (old_value - ARRAY['created_at','last_updated_at','updated_at','statistics','status']) IS NOT DISTINCT FROM
      (new_value - ARRAY['created_at','last_updated_at','updated_at','statistics','status']) THEN RETURN NEW;
    END IF;
  END IF;
  -- Re-evaluate both scopes when an alias/event/tenant changes.
  FOR value IN SELECT DISTINCT item FROM unnest(ARRAY[old_value,new_value]) item WHERE item IS NOT NULL LOOP
    IF TG_TABLE_NAME='template' THEN
      PERFORM sequent_backend.queue_report_prerenders((value->>'tenant_id')::uuid,NULL,value->>'alias');
    ELSE
      PERFORM sequent_backend.queue_report_prerenders((value->>'tenant_id')::uuid,
        (CASE WHEN TG_TABLE_NAME='election_event' THEN value->>'id' ELSE value->>'election_event_id' END)::uuid,NULL);
    END IF;
  END LOOP;
  RETURN COALESCE(NEW,OLD);
END $$;

DO $$ DECLARE table_name text; BEGIN
  FOREACH table_name IN ARRAY ARRAY['template','report','election_event','election','contest','candidate','ballot_publication','tally_results_publication'] LOOP
    EXECUTE format('CREATE TRIGGER invalidate_report_prerender AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.%I FOR EACH ROW EXECUTE FUNCTION sequent_backend.invalidate_report_prerenders()',table_name);
  END LOOP;
END $$;
-- Existing opted-in report definitions are warmed after migration as well.
DO $$ DECLARE tenant uuid; BEGIN
  FOR tenant IN SELECT DISTINCT tenant_id FROM sequent_backend.report LOOP
    PERFORM sequent_backend.queue_report_prerenders(tenant);
  END LOOP;
END $$;
