-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE FUNCTION sequent_backend.administration_settings(value jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE AS $$
SELECT COALESCE((SELECT jsonb_object_agg(key,val) FROM jsonb_each(CASE WHEN jsonb_typeof(value)='object' THEN value ELSE '{}'::jsonb END) AS fields(key,val)
    WHERE key=ANY(ARRAY['keys_ceremony_policy','contest_encryption_policy','decoded_ballot_inclusion_policy',
    'locked_down','otp','voter_signing_policy','voter_certificate_policy','ceremonies_policy','batch_anonymity_policy',
    'ballot_box_seal_policy','ballot_box_seal_record_policy','ballot_tracker_policy','timezones',
    'initialization_report_policy','security_confirmation_policy'])), '{}'::jsonb)
    || jsonb_build_object('i18n',COALESCE((SELECT jsonb_object_agg(locale,jsonb_build_object('name',text->'name'))
        FROM jsonb_each(CASE WHEN jsonb_typeof(value->'i18n')='object' THEN value->'i18n' ELSE '{}'::jsonb END) AS titles(locale,text)), '{}'::jsonb));
$$;
ALTER TABLE sequent_backend.election_event ADD COLUMN administration_settings jsonb
    GENERATED ALWAYS AS (sequent_backend.administration_settings(presentation::jsonb)) STORED;
ALTER TABLE sequent_backend.election ADD COLUMN administration_settings jsonb
    GENERATED ALWAYS AS (sequent_backend.administration_settings(presentation::jsonb)) STORED;

-- Hasura administrators may edit security settings, but cannot replace the
-- ballot/display/rule/date data hidden by their projection, even with raw JSON.
CREATE FUNCTION sequent_backend.guard_administration_settings() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE session jsonb; supplied jsonb; safe jsonb;
BEGIN
    session=NULLIF(current_setting('hasura.user',true),'')::jsonb;
    IF session IS NULL THEN RETURN NEW; END IF;
    IF session->>'x-hasura-role' NOT IN ('admin-user','election-event-create') THEN
        RAISE EXCEPTION 'Election configuration changes require the board-approved import workflow' USING ERRCODE='42501';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle WHERE tenant_id=NEW.tenant_id
        AND election_event_id=NEW.id AND phase='preparation') THEN
        RAISE EXCEPTION 'Security settings may be changed only in Preparation' USING ERRCODE='42501';
    END IF;
    supplied=COALESCE(NEW.presentation::jsonb,'{}'::jsonb);
    safe=sequent_backend.administration_settings(supplied);
    IF supplied-'i18n' <> safe-'i18n' OR (supplied ? 'i18n' AND supplied->'i18n' <> sequent_backend.administration_settings(OLD.presentation::jsonb)->'i18n') THEN
        RAISE EXCEPTION 'Administrators may change security settings only' USING ERRCODE='42501';
    END IF;
    NEW.presentation=COALESCE(OLD.presentation::jsonb,'{}'::jsonb)||(supplied-'i18n');
    RETURN NEW;
END $$;
CREATE TRIGGER aaa_guard_administration_settings BEFORE UPDATE OF presentation
ON sequent_backend.election_event FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_administration_settings();
