CREATE OR REPLACE FUNCTION sequent_backend.count_applications_fn(
  election_event_id UUID,
  permission_label TEXT DEFAULT NULL,
  regular_filters JSONB DEFAULT '{}',
  jsonb_filters JSONB DEFAULT '{}'
)
RETURNS INTEGER
LANGUAGE plpgsql
AS $$
DECLARE
  total_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO total_count
  FROM sequent_backend.applications
  WHERE 
    election_event_id = count_applications.election_event_id
    AND (permission_label IS NULL OR permission_label = count_applications.permission_label)
    
    -- Regular column filters
    AND (regular_filters->>'id' IS NULL OR id::text = regular_filters->>'id')
    AND (regular_filters->>'applicant_id' IS NULL OR applicant_id::text = regular_filters->>'applicant_id')
    AND (regular_filters->>'verification_type' IS NULL OR verification_type ILIKE '%' || (regular_filters->>'verification_type') || '%')
    AND (regular_filters->>'status' IS NULL OR status ILIKE '%' || (regular_filters->>'status') || '%')

    -- JSONB filters for applicant_data
    AND (
      jsonb_filters = '{}'::jsonb
      OR NOT EXISTS (
        SELECT 1 
        FROM jsonb_each_text(jsonb_filters) AS jf(key, value)
        WHERE NOT (
          (jf.key = 'dateOfBirth' AND applicant_data->>jf.key = jf.value)
          OR (jf.key != 'dateOfBirth' AND applicant_data->>jf.key ILIKE '%' || jf.value || '%')
        )
      )
    );

  RETURN total_count;
END;
$$;
