CREATE OR REPLACE FUNCTION sequent_backend.count_applications_func(
  election_event_id UUID,
  permission_label TEXT DEFAULT NULL,
  regular_filters JSONB DEFAULT '{}'::jsonb,
  jsonb_filters JSONB DEFAULT '{}'::jsonb
)
RETURNS SETOF sequent_backend.application_count  -- Now references existing type
LANGUAGE sql
STABLE
AS $$
  SELECT COUNT(*)::INTEGER AS total
  FROM sequent_backend.applications AS app
  WHERE 
    app.election_event_id = $1
    AND (permission_label IS NULL OR app.permission_label = permission_label)
    
    -- Regular filters
    AND (regular_filters->>'id' IS NULL OR app.id::TEXT = (regular_filters->>'id')::TEXT)
    AND (regular_filters->>'applicant_id' IS NULL OR app.applicant_id::TEXT = (regular_filters->>'applicant_id')::TEXT)
    AND (regular_filters->>'verification_type' IS NULL OR app.verification_type ILIKE ('%' || (regular_filters->>'verification_type') || '%'))
    AND (regular_filters->>'status' IS NULL OR app.status ILIKE ('%' || (regular_filters->>'status') || '%'))

    -- JSONB filters
    AND (
      jsonb_filters = '{}'::jsonb OR
      NOT EXISTS (
        SELECT 1 
        FROM jsonb_each_text(jsonb_filters) AS jf(key, value)
        WHERE NOT (
          (jf.key = 'dateOfBirth' AND app.applicant_data->>jf.key = jf.value)
          OR 
          (jf.key <> 'dateOfBirth' AND app.applicant_data->>jf.key ILIKE ('%' || jf.value || '%'))
        )
      )
    );
$$;
