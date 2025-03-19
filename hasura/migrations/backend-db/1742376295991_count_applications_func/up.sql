CREATE OR REPLACE FUNCTION sequent_backend.count_applications_func(
  election_event_id_param UUID,
  permission_label_param TEXT DEFAULT NULL,
  regular_filters JSONB DEFAULT '{}'::jsonb,
  jsonb_filters JSONB DEFAULT '{}'::jsonb
)
RETURNS TABLE(total INTEGER)  -- Hasura-friendly return type
LANGUAGE sql
STABLE
AS $$
  SELECT COUNT(*) AS total
  FROM sequent_backend.applications AS app
  WHERE 
    app.election_event_id = election_event_id_param
    AND (permission_label_param IS NULL OR app.permission_label = permission_label_param)
    
    -- Regular filters (exact/partial matches)
    AND (regular_filters->>'id' IS NULL OR app.id::TEXT = (regular_filters->>'id')::TEXT)
    AND (regular_filters->>'applicant_id' IS NULL OR app.applicant_id::TEXT = (regular_filters->>'applicant_id')::TEXT)
    AND (regular_filters->>'verification_type' IS NULL OR app.verification_type ILIKE ('%' || (regular_filters->>'verification_type') || '%'))
    AND (regular_filters->>'status' IS NULL OR app.status ILIKE ('%' || (regular_filters->>'status') || '%'))

    -- JSONB filters (exact/partial matches)
    AND (
      jsonb_filters = '{}'::jsonb OR
      (
        app.applicant_data @> (jsonb_filters - 'dateOfBirth') AND
        (jsonb_filters->>'dateOfBirth' IS NULL OR app.applicant_data->>'dateOfBirth' = jsonb_filters->>'dateOfBirth')
      )
    );
$$;
